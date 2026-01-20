# Session 20.5: WASM Host & Observability

**Parent Plan**: [session-20-plan.md](session-20-plan.md)
**Target**: ~25k tokens
**Prerequisites**: Session 20.4 complete (State management)

---

## Scope

**Goal**: WASM host setup with error boundaries and structured logging

**Estimated Tokens**: ~25k

## Context: Types Available from Previous Sessions

**From Session 20.1 (ProjectManagement.Core):**
- `ProjectManagement.Core.Models.*` - WorkItem, Sprint, CreateWorkItemRequest, UpdateWorkItemRequest, FieldChange, ConnectionState
- `ProjectManagement.Core.Interfaces.*` - IWebSocketClient, IConnectionHealth, IWorkItemStore, ISprintStore
- `ProjectManagement.Core.Exceptions.*` - ConnectionException, RequestTimeoutException, ServerRejectedException, ValidationException, VersionConflictException, CircuitOpenException
- `ProjectManagement.Core.Validation.*` - IValidator, CreateWorkItemRequestValidator, UpdateWorkItemRequestValidator

**From Session 20.2 (ProjectManagement.Services.WebSocket):**
- `WebSocketOptions`, `WebSocketClient`, `ConnectionHealthTracker`

**From Session 20.3 (ProjectManagement.Services.Resilience):**
- `CircuitBreaker`, `CircuitBreakerOptions`, `CircuitState`
- `RetryPolicy`, `RetryPolicyOptions`
- `ReconnectionService`, `ReconnectionOptions`
- `ResilientWebSocketClient`

**From Session 20.4 (ProjectManagement.Services.State):**
- `WorkItemStore`, `SprintStore`, `AppState`, `OptimisticUpdate<T>`

---

### Phase 5.1: Correlation ID Logger

```csharp
// CorrelationIdLoggerProvider.cs
using Microsoft.Extensions.Logging;

namespace ProjectManagement.Services.Logging;

/// <summary>
/// Logger provider that adds correlation IDs to all log messages.
/// </summary>
public sealed class CorrelationIdLoggerProvider : ILoggerProvider
{
    private readonly LogLevel _minLevel;

    public CorrelationIdLoggerProvider(LogLevel minLevel = LogLevel.Debug)
    {
        _minLevel = minLevel;
    }

    public ILogger CreateLogger(string categoryName)
    {
        return new CorrelationIdLogger(categoryName, _minLevel);
    }

    public void Dispose() { }
}
```

```csharp
// CorrelationIdLogger.cs
using Microsoft.Extensions.Logging;

namespace ProjectManagement.Services.Logging;

/// <summary>
/// Logger that includes correlation IDs for request tracing.
/// </summary>
public sealed class CorrelationIdLogger : ILogger
{
    private readonly string _categoryName;
    private readonly LogLevel _minLevel;
    private static readonly AsyncLocal<string?> _correlationId = new();

    public static string CorrelationId
    {
        get => _correlationId.Value ?? Guid.NewGuid().ToString("N")[..8];
        set => _correlationId.Value = value;
    }

    public CorrelationIdLogger(string categoryName, LogLevel minLevel = LogLevel.Debug)
    {
        _categoryName = categoryName;
        _minLevel = minLevel;
    }

    public IDisposable? BeginScope<TState>(TState state) where TState : notnull
    {
        return null;
    }

    public bool IsEnabled(LogLevel logLevel) => logLevel >= _minLevel;

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

### Phase 5.2: Program.cs with Full DI Setup

```csharp
// Program.cs
using Microsoft.AspNetCore.Components.Web;
using Microsoft.AspNetCore.Components.WebAssembly.Hosting;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Options;
using ProjectManagement.Core.Interfaces;
using ProjectManagement.Core.Models;
using ProjectManagement.Core.Validation;
using ProjectManagement.Services.Logging;
using ProjectManagement.Services.Resilience;
using ProjectManagement.Services.State;
using ProjectManagement.Services.WebSocket;
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

### Phase 5.3: App.razor (Root Component)

```razor
@* App.razor *@
<Router AppAssembly="@typeof(App).Assembly">
    <Found Context="routeData">
        <RouteView RouteData="@routeData" DefaultLayout="@typeof(MainLayout)" />
        <FocusOnNavigate RouteData="@routeData" Selector="h1" />
    </Found>
    <NotFound>
        <PageTitle>Not found</PageTitle>
        <LayoutView Layout="@typeof(MainLayout)">
            <p role="alert">Sorry, there's nothing at this address.</p>
        </LayoutView>
    </NotFound>
</Router>
```

### Phase 5.4: MainLayout.razor

```razor
@* MainLayout.razor *@
@inherits LayoutComponentBase
@using ProjectManagement.Core.Exceptions

<RadzenLayout>
    <RadzenHeader>
        <RadzenStack Orientation="Orientation.Horizontal" AlignItems="AlignItems.Center" Gap="1rem" class="px-4">
            <RadzenText TextStyle="TextStyle.H5" class="m-0">Project Management</RadzenText>
            <ConnectionStatus />
        </RadzenStack>
    </RadzenHeader>

    <RadzenBody>
        <div class="rz-p-4">
            <AppErrorBoundary>
                @Body
            </AppErrorBoundary>
        </div>
    </RadzenBody>
</RadzenLayout>

<RadzenDialog />
<RadzenNotification />
<RadzenContextMenu />
<RadzenTooltip />
```

### Phase 5.5: Error Boundary Component

```razor
@* AppErrorBoundary.razor *@
@using Microsoft.AspNetCore.Components.Web
@using ProjectManagement.Core.Exceptions
@inherits ErrorBoundaryBase
@inject ILogger<AppErrorBoundary> Logger

@if (CurrentException is not null)
{
    <div class="error-boundary">
        <div class="error-content">
            <RadzenIcon Icon="error" Style="font-size: 3rem; color: var(--rz-danger);" />
            <RadzenText TextStyle="TextStyle.H5">Something went wrong</RadzenText>
            <RadzenText TextStyle="TextStyle.Body1">@GetUserFriendlyMessage()</RadzenText>
            <RadzenButton Text="Try Again" Click="Recover" ButtonStyle="ButtonStyle.Primary" class="mt-3" />
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

### Phase 5.6: Connection Status Component

```razor
@* ConnectionStatus.razor *@
@using ProjectManagement.Core.Models
@using ProjectManagement.Services.State
@inject AppState AppState
@implements IDisposable

<div class="connection-status @GetStatusClass()">
    <RadzenIcon Icon="@GetStatusIcon()" />
    <span>@GetStatusText()</span>
</div>

@code {
    private ConnectionState _state;

    protected override void OnInitialized()
    {
        _state = AppState.ConnectionState;
        AppState.OnConnectionStateChanged += HandleStateChanged;
    }

    private void HandleStateChanged(ConnectionState state)
    {
        _state = state;
        InvokeAsync(StateHasChanged);
    }

    private string GetStatusClass() => _state switch
    {
        ConnectionState.Connected => "connected",
        ConnectionState.Connecting => "connecting",
        ConnectionState.Reconnecting => "reconnecting",
        ConnectionState.Disconnected => "disconnected",
        ConnectionState.Closed => "closed",
        _ => "unknown"
    };

    private string GetStatusIcon() => _state switch
    {
        ConnectionState.Connected => "wifi",
        ConnectionState.Connecting => "sync",
        ConnectionState.Reconnecting => "sync_problem",
        ConnectionState.Disconnected => "wifi_off",
        ConnectionState.Closed => "close",
        _ => "help"
    };

    private string GetStatusText() => _state switch
    {
        ConnectionState.Connected => "Connected",
        ConnectionState.Connecting => "Connecting...",
        ConnectionState.Reconnecting => "Reconnecting...",
        ConnectionState.Disconnected => "Disconnected",
        ConnectionState.Closed => "Closed",
        _ => "Unknown"
    };

    public void Dispose()
    {
        AppState.OnConnectionStateChanged -= HandleStateChanged;
    }
}
```

### Phase 5.7: _Imports.razor

```razor
@* _Imports.razor *@
@using System.Net.Http
@using System.Net.Http.Json
@using Microsoft.AspNetCore.Components.Forms
@using Microsoft.AspNetCore.Components.Routing
@using Microsoft.AspNetCore.Components.Web
@using Microsoft.AspNetCore.Components.Web.Virtualization
@using Microsoft.AspNetCore.Components.WebAssembly.Http
@using Microsoft.Extensions.Logging
@using Microsoft.JSInterop
@using Radzen
@using Radzen.Blazor
@using ProjectManagement.Core.Models
@using ProjectManagement.Core.Interfaces
@using ProjectManagement.Core.Exceptions
@using ProjectManagement.Services.State
@using ProjectManagement.Wasm
@using ProjectManagement.Wasm.Shared
```

### Phase 5.8: wwwroot/index.html

```html
<!DOCTYPE html>
<html lang="en">

<head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Project Management</title>
    <base href="/" />
    <link rel="stylesheet" href="_content/Radzen.Blazor/css/material-base.css" />
    <link rel="stylesheet" href="css/app.css" />
    <link rel="icon" type="image/png" href="favicon.png" />
    <link href="ProjectManagement.Wasm.styles.css" rel="stylesheet" />
</head>

<body>
    <div id="app">
        <div class="loading-screen">
            <div class="spinner"></div>
            <p>Loading...</p>
        </div>
    </div>

    <div id="blazor-error-ui">
        An unhandled error has occurred.
        <a href="" class="reload">Reload</a>
        <a class="dismiss">X</a>
    </div>

    <script src="_content/Radzen.Blazor/Radzen.Blazor.js"></script>
    <script src="_framework/blazor.webassembly.js"></script>
</body>

</html>
```

### Phase 5.9: wwwroot/css/app.css

```css
/* app.css */

/* Loading screen */
.loading-screen {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100vh;
    color: #666;
}

.spinner {
    width: 40px;
    height: 40px;
    border: 4px solid #f3f3f3;
    border-top: 4px solid #3498db;
    border-radius: 50%;
    animation: spin 1s linear infinite;
    margin-bottom: 1rem;
}

@keyframes spin {
    0% { transform: rotate(0deg); }
    100% { transform: rotate(360deg); }
}

/* Blazor error UI */
#blazor-error-ui {
    background: lightyellow;
    bottom: 0;
    box-shadow: 0 -1px 2px rgba(0, 0, 0, 0.2);
    display: none;
    left: 0;
    padding: 0.6rem 1.25rem 0.7rem 1.25rem;
    position: fixed;
    width: 100%;
    z-index: 1000;
}

#blazor-error-ui .dismiss {
    cursor: pointer;
    position: absolute;
    right: 0.75rem;
    top: 0.5rem;
}

/* Error boundary */
.error-boundary {
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: 300px;
    padding: 2rem;
}

.error-content {
    text-align: center;
    max-width: 400px;
}

/* Connection status */
.connection-status {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    padding: 0.25rem 0.5rem;
    border-radius: 4px;
    font-size: 0.875rem;
}

.connection-status.connected {
    color: var(--rz-success);
}

.connection-status.connecting,
.connection-status.reconnecting {
    color: var(--rz-warning);
}

.connection-status.disconnected,
.connection-status.closed {
    color: var(--rz-danger);
}

/* General layout adjustments */
html, body {
    height: 100%;
    margin: 0;
    padding: 0;
}

body {
    font-family: 'Roboto', sans-serif;
}
```

### Phase 5.10: wwwroot/appsettings.json

```json
{
  "WebSocket": {
    "ServerUrl": "ws://localhost:8080/ws",
    "HeartbeatInterval": "00:00:30",
    "HeartbeatTimeout": "00:01:00",
    "RequestTimeout": "00:00:30",
    "ReceiveBufferSize": 8192
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
    "MaxDelay": "00:00:05",
    "BackoffMultiplier": 2.0
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
| `CorrelationIdLoggerProvider.cs` | Logger provider factory |
| `CorrelationIdLogger.cs` | Structured logging with correlation IDs |
| `Program.cs` | Full DI setup |
| `App.razor` | Root component with routing |
| `MainLayout.razor` | Application layout with error boundary |
| `AppErrorBoundary.razor` | Error handling component |
| `ConnectionStatus.razor` | Connection state indicator |
| `_Imports.razor` | Global imports |
| `wwwroot/index.html` | Host page |
| `wwwroot/css/app.css` | Base styles |
| `wwwroot/appsettings.json` | Configuration |
| **Total** | **11 files** |

### Success Criteria for 20.5

- [x] WASM app runs and connects to backend
- [x] Error boundary catches and displays errors gracefully
- [x] Structured logging with correlation IDs
- [x] Configuration loaded from appsettings.json
- [x] Radzen components available
- [x] Connection status displays current state

---

## ✅ Session 20.5 Complete (2026-01-19)

**Status**: Complete - Full end-to-end connectivity verified

**What Was Delivered:**
- ✅ Correlation ID logging system with AsyncLocal storage
- ✅ Program.cs with complete DI setup (all services registered)
- ✅ App.razor with router (NotFoundPage component)
- ✅ MainLayout.razor with Radzen layout (header, body, service components)
- ✅ AppErrorBoundary.razor with user-friendly error messages
- ✅ ConnectionStatus.razor with real-time state updates
- ✅ _Imports.razor with global namespace imports
- ✅ wwwroot/index.html with Radzen CSS and scripts
- ✅ wwwroot/css/app.css with loading, error, and connection status styles
- ✅ wwwroot/appsettings.json with WebSocket, circuit breaker, retry, reconnection configs

**Verification:**
- ✅ `dotnet build frontend/ProjectManagement.slnx` - 0 warnings, 0 errors
- ✅ `dotnet run --project frontend/ProjectManagement.Wasm` - App loads successfully
- ✅ WebSocket connection to ws://localhost:8000/ws successful
- ✅ Connection status shows "Connected" in green
- ✅ All Radzen components rendering correctly
- ✅ AppState initialized and connected to backend

**Files Created:** 11
**Build Time:** 2.49s
**Connection Time:** ~200ms

**Issues Encountered:**
- Port mismatch (appsettings.json had 8080, pm-server running on 8000) - corrected
- Duplicate NotFound.razor and NotFoundPage.razor with same route - resolved
- Missing namespace imports in various components - all resolved

**Next Session:** 20.6 - Comprehensive test suite (100+ tests)

---
