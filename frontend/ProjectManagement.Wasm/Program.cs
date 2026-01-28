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

// === Configuration ===
// WebSocket options - URL will be set dynamically in desktop mode by App.razor
builder.Services.Configure<WebSocketOptions>(options =>
{
    var config = builder.Configuration.GetSection("WebSocket");
    
    // Default URL for web mode (desktop mode overrides this dynamically)
    options.ServerUrl = config["ServerUrl"] ?? "ws://localhost:8000/ws";
    options.HeartbeatInterval = config.GetValue<TimeSpan?>("HeartbeatInterval") ?? TimeSpan.FromSeconds(30);
    options.HeartbeatTimeout = config.GetValue<TimeSpan?>("HeartbeatTimeout") ?? TimeSpan.FromSeconds(60);
    options.RequestTimeout = config.GetValue<TimeSpan?>("RequestTimeout") ?? TimeSpan.FromSeconds(30);
    options.ReceiveBufferSize = config.GetValue<int?>("ReceiveBufferSize") ?? 8192;
});

builder.Services.Configure<CircuitBreakerOptions>(builder.Configuration.GetSection("CircuitBreaker"));
builder.Services.Configure<RetryPolicyOptions>(builder.Configuration.GetSection("Retry"));
builder.Services.Configure<ReconnectionOptions>(builder.Configuration.GetSection("Reconnection"));

// === Logging ===
builder.Services.AddLogging(logging =>
{
    logging.SetMinimumLevel(LogLevel.Debug);
    logging.AddProvider(new CorrelationIdLoggerProvider());
});

// === Desktop Services ===
builder.Services.AddScoped<TauriService>();
builder.Services.AddScoped<ITauriService>(sp => sp.GetRequiredService<TauriService>());
builder.Services.AddScoped<IDesktopConfigService, DesktopConfigService>();
builder.Services.AddScoped<UserIdentityService>();

// === Validators ===
builder.Services.AddSingleton<IValidator<CreateWorkItemRequest>, CreateWorkItemRequestValidator>();
builder.Services.AddSingleton<IValidator<UpdateWorkItemRequest>, UpdateWorkItemRequestValidator>();

// === Resilience ===
builder.Services.AddSingleton<CircuitBreaker>();
builder.Services.AddSingleton<RetryPolicy>();
builder.Services.AddSingleton<ReconnectionService>();

// === WebSocket ===
builder.Services.AddSingleton<WebSocketClient>();
builder.Services.AddSingleton<IWebSocketClient>(sp =>
{
    var inner = sp.GetRequiredService<WebSocketClient>();
    var circuitBreaker = sp.GetRequiredService<CircuitBreaker>();
    var retryPolicy = sp.GetRequiredService<RetryPolicy>();
    var wsLogger = sp.GetRequiredService<ILogger<ResilientWebSocketClient>>();
    
    return new ResilientWebSocketClient(inner, circuitBreaker, retryPolicy, wsLogger);
});

// === State Management ===
builder.Services.AddSingleton<IWorkItemStore, WorkItemStore>();
builder.Services.AddSingleton<ISprintStore, SprintStore>();
builder.Services.AddSingleton<IProjectStore, ProjectStore>();
builder.Services.AddSingleton<ICommentStore, CommentStore>();
builder.Services.AddSingleton<ITimeEntryStore, TimeEntryStore>();
builder.Services.AddSingleton<IDependencyStore, DependencyStore>();
builder.Services.AddSingleton<AppState>();

// === ViewModels ===
builder.Services.AddScoped<ViewModelFactory>();

// === Radzen ===
builder.Services.AddRadzenComponents();

// === Run ===
var app = builder.Build();
await app.RunAsync();