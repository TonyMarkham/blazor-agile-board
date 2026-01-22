  using Microsoft.AspNetCore.Components.Web;
  using Microsoft.AspNetCore.Components.WebAssembly.Hosting;
  using ProjectManagement.Core.Interfaces;
  using ProjectManagement.Core.Models;
  using ProjectManagement.Core.Validation;
  using ProjectManagement.Core.ViewModels;
  using ProjectManagement.Services.Desktop;
  using ProjectManagement.Services.Logging;
  using ProjectManagement.Services.Resilience;
  using ProjectManagement.Services.State;
  using ProjectManagement.Services.WebSocket;
  using ProjectManagement.Wasm;
  using Radzen;

  var builder = WebAssemblyHostBuilder.CreateDefault(args);
  builder.RootComponents.Add<App>("#app");
  builder.RootComponents.Add<HeadOutlet>("head::after");

  // === Phase 1: Detect Desktop Mode ===

  // Register minimal services for detection
  builder.Services.AddScoped<DesktopConfigService>();
  builder.Logging.SetMinimumLevel(LogLevel.Information);

  // Build temporary host for desktop detection
  var tempHost = builder.Build();
  var desktopConfigService = tempHost.Services.GetRequiredService<DesktopConfigService>();
  var logger = tempHost.Services.GetRequiredService<ILogger<Program>>();

  var isDesktopMode = await desktopConfigService.IsDesktopModeAsync();
  string? serverUrl = null;

  if (isDesktopMode)
  {
      logger.LogInformation("🖥️  Desktop mode detected - waiting for embedded server...");

      try
      {
          // Wait for embedded server to be ready (30 second timeout)
          serverUrl = await desktopConfigService.WaitForServerAsync(
              timeout: TimeSpan.FromSeconds(30));

          logger.LogInformation("✅ Embedded server ready at: {ServerUrl}", serverUrl);
      }
      catch (Exception ex)
      {
          logger.LogError(ex, "❌ Failed to connect to embedded server");
          throw new InvalidOperationException(
              "Unable to connect to embedded server. Please restart the application.", ex);
      }

      await tempHost.DisposeAsync();
  }
  else
  {
      logger.LogInformation("🌐 Web mode detected - using config from appsettings.json");
      await tempHost.DisposeAsync();
  }

  // === Phase 2: Build Final Host with Correct Configuration ===

  builder = WebAssemblyHostBuilder.CreateDefault(args);
  builder.RootComponents.Add<App>("#app");
  builder.RootComponents.Add<HeadOutlet>("head::after");

  // Configuration
  builder.Services.Configure<WebSocketOptions>(options =>
  {
      var config = builder.Configuration.GetSection("WebSocket");

      if (isDesktopMode && !string.IsNullOrEmpty(serverUrl))
      {
          // Desktop mode: use discovered server URL
          options.ServerUrl = serverUrl;
          options.HeartbeatInterval = TimeSpan.FromSeconds(15); // Shorter for local server
          options.HeartbeatTimeout = TimeSpan.FromSeconds(30);
          logger.LogInformation("Using desktop server URL: {Url}", serverUrl);
      }
      else
      {
          // Web mode: use config from appsettings.json
          options.ServerUrl = config["ServerUrl"] ?? "ws://localhost:8000/ws";
          options.HeartbeatInterval = config.GetValue<TimeSpan>("HeartbeatInterval");
          options.HeartbeatTimeout = config.GetValue<TimeSpan>("HeartbeatTimeout");
          options.RequestTimeout = config.GetValue<TimeSpan>("RequestTimeout");
          options.ReceiveBufferSize = config.GetValue<int>("ReceiveBufferSize");
      }
  });

  builder.Services.Configure<CircuitBreakerOptions>(
      builder.Configuration.GetSection("CircuitBreaker"));
  builder.Services.Configure<RetryPolicyOptions>(
      builder.Configuration.GetSection("Retry"));
  builder.Services.Configure<ReconnectionOptions>(
      builder.Configuration.GetSection("Reconnection"));

  // Logging
  builder.Services.AddLogging(logging =>
  {
      logging.SetMinimumLevel(LogLevel.Debug);
      logging.AddProvider(new CorrelationIdLoggerProvider());
  });

  // Desktop Config Service
  builder.Services.AddScoped<DesktopConfigService>();

  // Validators (required by WebSocketClient)
  builder.Services.AddSingleton<IValidator<CreateWorkItemRequest>, CreateWorkItemRequestValidator>();
  builder.Services.AddSingleton<IValidator<UpdateWorkItemRequest>, UpdateWorkItemRequestValidator>();

  // Resilience
  builder.Services.AddSingleton<CircuitBreaker>();
  builder.Services.AddSingleton<RetryPolicy>();
  builder.Services.AddSingleton<ReconnectionService>();

  // WebSocket
  builder.Services.AddSingleton<WebSocketClient>();
  builder.Services.AddSingleton<IWebSocketClient>(sp =>
  {
      var inner = sp.GetRequiredService<WebSocketClient>();
      var circuitBreaker = sp.GetRequiredService<CircuitBreaker>();
      var retryPolicy = sp.GetRequiredService<RetryPolicy>();
      var wsLogger = sp.GetRequiredService<ILogger<ResilientWebSocketClient>>();
      return new ResilientWebSocketClient(inner, circuitBreaker, retryPolicy, wsLogger);
  });

  // State Management
  builder.Services.AddSingleton<IWorkItemStore, WorkItemStore>();
  builder.Services.AddSingleton<ISprintStore, SprintStore>();
  builder.Services.AddSingleton<IProjectStore, ProjectStore>();
  builder.Services.AddSingleton<AppState>();

  // ViewModels
  builder.Services.AddScoped<ViewModelFactory>();

  // Radzen
  builder.Services.AddRadzenComponents();

  var app = builder.Build();

  // === Setup Desktop Mode Reconnection Handler ===

  if (isDesktopMode)
  {
      var desktopConfig = app.Services.GetRequiredService<DesktopConfigService>();
      var wsClient = app.Services.GetRequiredService<WebSocketClient>();
      var reconnectLogger = app.Services.GetRequiredService<ILogger<Program>>();

      // Subscribe to server state changes (for restarts)
      await desktopConfig.SubscribeToServerStateChangesAsync(async (state) =>
      {
          reconnectLogger.LogInformation("Server state changed: {State}", state);

          // If server is running again after restart, reconnect
          if (state.Contains("Running", StringComparison.OrdinalIgnoreCase))
          {
              try
              {
                  var newStatus = await desktopConfig.GetServerStatusAsync();

                  if (!string.IsNullOrEmpty(newStatus?.WebsocketUrl) &&
                      newStatus.WebsocketUrl != serverUrl)
                  {
                      reconnectLogger.LogInformation(
                          "Server URL changed: {OldUrl} -> {NewUrl}",
                          serverUrl,
                          newStatus.WebsocketUrl);

                      // Reconnect to new URL
                      await wsClient.ReconnectAsync(newStatus.WebsocketUrl);
                      serverUrl = newStatus.WebsocketUrl;
                  }
              }
              catch (Exception ex)
              {
                  reconnectLogger.LogError(ex, "Failed to reconnect after server restart");
              }
          }
      });

      logger.LogInformation("✅ Desktop reconnection handler configured");
  }

  // === Initialize Application State ===

  var appState = app.Services.GetRequiredService<AppState>();
  var initLogger = app.Services.GetRequiredService<ILogger<Program>>();

  try
  {
      await appState.InitializeAsync();
      logger.LogInformation("✅ Application state initialized");
  }
  catch (Exception ex)
  {
      initLogger.LogError(ex, "❌ Failed to initialize application state");
      // App will show connection error UI
  }

  await app.RunAsync();