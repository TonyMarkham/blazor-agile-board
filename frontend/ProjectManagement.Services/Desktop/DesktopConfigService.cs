  using Microsoft.Extensions.Logging;
  using Microsoft.JSInterop;

  namespace ProjectManagement.Services.Desktop;

  /// <summary>
  /// Service for detecting and managing desktop mode via Tauri.
  /// </summary>
  public sealed class DesktopConfigService : IAsyncDisposable
  {
      private readonly IJSRuntime _jsRuntime;
      private readonly ILogger<DesktopConfigService> _logger;
      private DesktopConfig? _cachedConfig;
      private IJSObjectReference? _interopModule;

      public DesktopConfigService(
          IJSRuntime jsRuntime,
          ILogger<DesktopConfigService> logger)
      {
          _jsRuntime = jsRuntime;
          _logger = logger;
      }

      /// <summary>
      /// Get desktop configuration. Returns null if not in desktop mode.
      /// </summary>
      public async Task<DesktopConfig?> GetConfigAsync()
      {
          if (_cachedConfig != null)
              return _cachedConfig;

          try
          {
              // Read window.PM_CONFIG set by index.html
              _cachedConfig = await _jsRuntime.InvokeAsync<DesktopConfig>(
                  "eval",
                  "window.PM_CONFIG"
              );

              if (_cachedConfig?.IsDesktop == true)
              {
                  _logger.LogInformation("Desktop mode detected");
                  return _cachedConfig;
              }

              _logger.LogInformation("Web mode detected");
              return null;
          }
          catch (Exception ex)
          {
              _logger.LogWarning(ex, "Failed to detect desktop mode, assuming web mode");
              return null;
          }
      }

      /// <summary>
      /// Check if running in desktop mode.
      /// </summary>
      public async Task<bool> IsDesktopModeAsync()
      {
          var config = await GetConfigAsync();
          return config?.IsDesktop == true;
      }

      /// <summary>
      /// Get server status from Tauri (desktop mode only).
      /// </summary>
      public async Task<ServerStatus?> GetServerStatusAsync()
      {
          if (!await IsDesktopModeAsync())
          {
              throw new InvalidOperationException("Not in desktop mode");
          }

          try
          {
              var module = await GetInteropModuleAsync();
              return await module.InvokeAsync<ServerStatus>("getServerStatus");
          }
          catch (Exception ex)
          {
              _logger.LogError(ex, "Failed to get server status from Tauri");
              throw;
          }
      }

      /// <summary>
      /// Poll server until it's running (desktop mode only).
      /// </summary>
      public async Task<string> WaitForServerAsync(
          TimeSpan timeout,
          CancellationToken ct = default)
      {
          if (!await IsDesktopModeAsync())
          {
              throw new InvalidOperationException("Not in desktop mode");
          }

          var stopwatch = System.Diagnostics.Stopwatch.StartNew();
          var pollInterval = TimeSpan.FromMilliseconds(500);

          while (stopwatch.Elapsed < timeout && !ct.IsCancellationRequested)
          {
              try
              {
                  var status = await GetServerStatusAsync();

                  if (status?.State == "running" && !string.IsNullOrEmpty(status.WebsocketUrl))
                  {
                      _logger.LogInformation(
                          "Server ready at {Url} after {Elapsed}ms",
                          status.WebsocketUrl,
                          stopwatch.ElapsedMilliseconds);
                      return status.WebsocketUrl;
                  }

                  if (status?.State == "failed")
                  {
                      throw new InvalidOperationException(
                          $"Server failed to start: {status.Error}");
                  }

                  _logger.LogDebug(
                      "Server state: {State}, waiting... ({Elapsed}s)",
                      status?.State,
                      (int)stopwatch.Elapsed.TotalSeconds);
              }
              catch (Exception ex) when (ex is not InvalidOperationException)
              {
                  _logger.LogDebug(ex, "Server status check failed, retrying...");
              }

              await Task.Delay(pollInterval, ct);
          }

          throw new TimeoutException(
              $"Server did not start within {timeout.TotalSeconds}s");
      }

      /// <summary>
      /// Subscribe to server state change events (desktop mode only).
      /// </summary>
      public async Task SubscribeToServerStateChangesAsync( Func<string, Task> callback)
      {
          if (!await IsDesktopModeAsync())
          {
              return; // No-op in web mode
          }

          try
          {
              var module = await GetInteropModuleAsync();
              var dotNetRef = DotNetObjectReference.Create(
                  new ServerStateChangeHandler(callback, _logger));

              await module.InvokeVoidAsync(
                  "onServerStateChanged",
                  dotNetRef);

              _logger.LogInformation("Subscribed to server state changes");
          }
          catch (Exception ex)
          {
              _logger.LogError(ex, "Failed to subscribe to server state changes");
          }
      }

      private async Task<IJSObjectReference> GetInteropModuleAsync()
      {
          if (_interopModule != null)
              return _interopModule;

          _interopModule = await _jsRuntime.InvokeAsync<IJSObjectReference>(
              "eval",
              "window.DesktopInterop"
          );

          return _interopModule;
      }

      public async ValueTask DisposeAsync()
      {
          if (_interopModule != null)
          {
              await _interopModule.DisposeAsync();
          }
      }
  }