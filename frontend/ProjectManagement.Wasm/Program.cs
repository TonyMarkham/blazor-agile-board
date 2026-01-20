using Microsoft.AspNetCore.Components.Web;
using Microsoft.AspNetCore.Components.WebAssembly.Hosting;
using ProjectManagement.Core.Interfaces;
using ProjectManagement.Core.Models;
using ProjectManagement.Core.Validation;
using ProjectManagement.Core.ViewModels;
using ProjectManagement.Services.Logging;
using ProjectManagement.Services.Resilience;
using ProjectManagement.Services.State;
using ProjectManagement.Services.WebSocket;
using ProjectManagement.Wasm;
using Radzen;

var builder = WebAssemblyHostBuilder.CreateDefault(args);
builder.RootComponents.Add<App>("#app");
builder.RootComponents.Add<HeadOutlet>("head::after");

// Configuration
builder.Services.Configure<WebSocketOptions>(
    builder.Configuration.GetSection("WebSocket"));
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
    var logger = sp.GetRequiredService<ILogger<ResilientWebSocketClient>>();
    return new ResilientWebSocketClient(inner, circuitBreaker, retryPolicy, logger);
});

// State Management
builder.Services.AddSingleton<IWorkItemStore, WorkItemStore>();
builder.Services.AddSingleton<ISprintStore, SprintStore>();
builder.Services.AddSingleton<AppState>();

// ViewModels
builder.Services.AddScoped<ViewModelFactory>();

// Radzen
builder.Services.AddRadzenComponents();

var app = builder.Build();

// Initialize state
var appState = app.Services.GetRequiredService<AppState>();
var logger = app.Services.GetRequiredService<ILogger<Program>>();

try
{
    await appState.InitializeAsync();
}
catch (Exception ex)
{
    logger.LogError(ex, "Failed to initialize application state");
    // App will show connection error UI
}

await app.RunAsync();
