# Session 20.5: WASM Host & Observability

**Parent Plan**: [session-20-plan.md](session-20-plan.md)
**Target**: ~25k tokens
**Prerequisites**: Session 20.4 complete (State management)

---

## Scope

**Goal**: WASM host setup with error boundaries and structured logging

**Estimated Tokens**: ~25k

### Phase 5.1: Program.cs with Full DI Setup

```csharp
// Program.cs
using Microsoft.AspNetCore.Components.Web;
using Microsoft.AspNetCore.Components.WebAssembly.Hosting;
using ProjectManagement.Services.WebSocket;
using ProjectManagement.Services.State;
using ProjectManagement.Services.Resilience;
using ProjectManagement.Services.Logging;
using ProjectManagement.Wasm;

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
```

### Phase 5.2: Error Boundary Component

```razor
@* ErrorBoundary.razor *@
@using Microsoft.AspNetCore.Components.Web
@inherits ErrorBoundaryBase

@if (CurrentException is not null)
{
    <div class="error-boundary">
        <div class="error-content">
            <h3>Something went wrong</h3>
            <p>@GetUserFriendlyMessage()</p>
            <button class="btn btn-primary" @onclick="Recover">Try Again</button>
        </div>
    </div>
}
else
{
    @ChildContent
}

@code {
    [Parameter]
    public RenderFragment? ChildContent { get; set; }

    [Inject]
    private ILogger<ErrorBoundary> Logger { get; set; } = default!;

    protected override Task OnErrorAsync(Exception exception)
    {
        Logger.LogError(exception, "Unhandled error in component tree");
        return Task.CompletedTask;
    }

    private string GetUserFriendlyMessage()
    {
        return CurrentException switch
        {
            ConnectionException => "Unable to connect to the server. Please check your connection.",
            RequestTimeoutException => "The request timed out. Please try again.",
            CircuitOpenException => "Service temporarily unavailable. Please wait a moment.",
            ValidationException ve => ve.UserMessage,
            ServerRejectedException sre => sre.UserMessage,
            _ => "An unexpected error occurred. Please try again."
        };
    }
}
```

### Phase 5.3: Correlation ID Logger

```csharp
// CorrelationIdLogger.cs
namespace ProjectManagement.Services.Logging;

public sealed class CorrelationIdLoggerProvider : ILoggerProvider
{
    public ILogger CreateLogger(string categoryName)
    {
        return new CorrelationIdLogger(categoryName);
    }

    public void Dispose() { }
}

public sealed class CorrelationIdLogger : ILogger
{
    private readonly string _categoryName;
    private static readonly AsyncLocal<string?> _correlationId = new();

    public static string CorrelationId
    {
        get => _correlationId.Value ?? Guid.NewGuid().ToString("N")[..8];
        set => _correlationId.Value = value;
    }

    public CorrelationIdLogger(string categoryName)
    {
        _categoryName = categoryName;
    }

    public IDisposable? BeginScope<TState>(TState state) where TState : notnull
    {
        return null;
    }

    public bool IsEnabled(LogLevel logLevel) => true;

    public void Log<TState>(
        LogLevel logLevel,
        EventId eventId,
        TState state,
        Exception? exception,
        Func<TState, Exception?, string> formatter)
    {
        if (!IsEnabled(logLevel))
            return;

        var message = formatter(state, exception);
        var timestamp = DateTime.UtcNow.ToString("HH:mm:ss.fff");
        var level = logLevel.ToString()[..3].ToUpper();
        var category = _categoryName.Split('.').LastOrDefault() ?? _categoryName;

        Console.WriteLine($"[{timestamp}] [{level}] [{CorrelationId}] {category}: {message}");

        if (exception != null)
        {
            Console.WriteLine($"  Exception: {exception.GetType().Name}: {exception.Message}");
        }
    }
}
```

### Phase 5.4: appsettings.json

```json
{
  "WebSocket": {
    "ServerUrl": "ws://localhost:8080/ws",
    "HeartbeatInterval": "00:00:30",
    "HeartbeatTimeout": "00:01:00",
    "RequestTimeout": "00:00:30"
  },
  "CircuitBreaker": {
    "FailureThreshold": 5,
    "OpenDuration": "00:00:30",
    "HalfOpenSuccessThreshold": 3,
    "FailureWindow": "00:01:00"
  },
  "Retry": {
    "MaxAttempts": 3,
    "InitialDelay": "00:00:00.100",
    "MaxDelay": "00:00:05"
  },
  "Reconnection": {
    "MaxAttempts": 10,
    "InitialDelay": "00:00:01",
    "MaxDelay": "00:00:30"
  }
}
```

### Files Summary for Sub-Session 20.5

| File | Purpose |
|------|---------|
| `Program.cs` | Full DI setup |
| `App.razor` | Root component |
| `_Imports.razor` | Global imports |
| `ErrorBoundary.razor` | Error handling |
| `CorrelationIdLogger.cs` | Structured logging |
| `wwwroot/index.html` | Host page |
| `wwwroot/css/app.css` | Base styles |
| `wwwroot/appsettings.json` | Configuration |
| **Total** | **8 files** |

### Success Criteria for 20.5

- [ ] WASM app runs and connects to backend
- [ ] Error boundary catches and displays errors gracefully
- [ ] Structured logging with correlation IDs
- [ ] Configuration loaded from appsettings.json
- [ ] Radzen components available

---

