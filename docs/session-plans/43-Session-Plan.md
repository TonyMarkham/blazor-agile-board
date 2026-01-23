# Session 43: Port Signalling & Program.cs Simplification

**Parent Plan**: N/A (standalone session)
**Target**: ~30-35k tokens
**Prerequisites**: Session 42.4 complete, `dotnet build frontend/ProjectManagement.slnx && cargo check --workspace` passes

---

## Teaching Focus

This session teaches:
- **Race condition elimination** - Event subscription + acknowledgment pattern
- **Startup simplification** - Single-host architecture vs temp-host dance
- **Tauri IPC patterns** - Commands vs events, when to use each
- **JavaScript interop pitfalls** - Why `eval` causes TypeLoadException in Blazor
- **Constants over magic strings** - Maintainable, searchable, refactor-safe code
- **Test-driven changes** - Adding tests alongside production code

---

## The Problem

When the Tauri desktop app starts, two things happen concurrently:

1. **Tauri** spawns pm-server as a sidecar process
2. **WASM** loads in the WebView and needs to connect to pm-server

The WASM needs to know which port pm-server is running on. Currently there's a race condition:

```
Timeline A: Server starts BEFORE WASM subscribes to events
─────────────────────────────────────────────────────────
Tauri: [spawns pm-server] → [server ready on port 54321] → [emits "server-ready" ()]
WASM:                                                            [subscribes] → waits forever...

Timeline B: Server starts AFTER WASM subscribes
───────────────────────────────────────────────
Tauri: [spawns pm-server] ─────────────────────────────→ [server ready] → [emits event]
WASM:        [subscribes to events] → [waiting...] ─────────────────────→ [receives event] ✓
```

Additionally, `Program.cs` has a convoluted "temp host" pattern:
1. Build temporary host just to detect desktop mode
2. Wait for server
3. Dispose temp host
4. Build final host with discovered URL
5. Call `InitializeAsync()` which fails (no user set yet)

There's also an `eval` usage issue in both `TauriService.cs` and `TauriEventSubscription.cs` that can cause `TypeLoadException` in Blazor WASM.

---

## The Solution

**Two-phase handshake** eliminates the race:

```
WASM                              Tauri
  │                                 │
  │── subscribe to events ────────>│
  │                                 │
  │── "wasm_ready" command ───────>│
  │                                 │
  │<── current ServerStatus ───────│  (port may be null if still starting)
  │                                 │
  │     ... time passes ...         │
  │                                 │
  │<── "server-ready" event ───────│  (if server starts after ping)
  │                                 │
```

**Key insight**: WASM sends a ping *after* subscribing. Tauri responds with current state. Either:
- The response has the port (server already running) → done
- The response has no port → wait for `server-ready` event

**Program.cs simplification**:
- Build host once with placeholder WebSocket URL
- Let `App.razor` handle the entire startup sequence
- Update WebSocket URL dynamically when discovered

**eval elimination**:
- Add named JS functions for all interop calls
- Replace all `eval` usage with direct function calls

---

## Scope

1. **index.html** - Add `checkTauriAvailable()` and `unlistenTauri()` JS functions
2. **ITauriService.cs** - New interface for testability
3. **TauriService.cs** - Implement interface, add `NotifyReadyAsync()`, fix `eval` detection, add constants
4. **TauriEventSubscription.cs** - Replace `eval` with named function
5. **DesktopConfigService.cs** - Depend on `ITauriService`, rewrite `WaitForServerAsync()` with handshake pattern
6. **ServerStatus.cs** - Add `Pid` field
7. **ServerStateEvent.cs** - Add `Pid` field for consistency
8. **Tauri Backend** - Emit full `ServerStatus` in `server-ready` event, add `wasm_ready` command, add `server_pid()` method
9. **Program.cs** - Remove temp-host dance, single build, register `ITauriService`
10. **Tests** - Add unit tests with proper mocking for C# and Rust

---

## Prerequisites Check

Before starting, verify the codebase compiles:

```bash
# Frontend
dotnet build frontend/ProjectManagement.slnx

# Backend
cargo check --workspace

# Desktop
cd desktop && cargo check

# Run existing tests
dotnet test frontend/ProjectManagement.slnx
cd desktop && cargo test
```

All commands should complete with no errors.

---

## Implementation Order

### Step 1: Extract JavaScript to External File

**The Problem**: Multiple places in the codebase use `eval` for JS interop, which causes `TypeLoadException` in some Blazor WASM scenarios. The inline script in index.html is growing and harder to maintain.

**The Solution**: Extract to a separate JS file with named functions that can be called directly.

**Step 1a: Create** `frontend/ProjectManagement.Wasm/wwwroot/js/desktop-detection.js`

```javascript
// Desktop mode detection and Tauri IPC helpers
// Runs before Blazor loads to detect Tauri environment

(function() {
    'use strict';

    // Detect Tauri desktop mode
    window.PM_CONFIG = {
        isDesktop: !!(window.__TAURI__),
        serverUrl: null  // Will be set by C# after server discovery
    };

    if (window.PM_CONFIG.isDesktop) {
        console.log('[Desktop Mode] Tauri detected');
    } else {
        console.log('[Web Mode] Running in browser');
    }

    // Used by TauriService.IsDesktopAsync() - avoids eval() which causes
    // TypeLoadException in some Blazor WASM scenarios
    window.checkTauriAvailable = function() {
        return typeof window.__TAURI__ !== 'undefined' &&
               typeof window.__TAURI__.core !== 'undefined';
    };

    // Used by TauriEventSubscription.DisposeAsync() - avoids eval()
    window.unlistenTauri = function(subscriptionId) {
        var unlisteners = window.__PM_UNLISTENERS__;
        if (unlisteners && typeof unlisteners[subscriptionId] === 'function') {
            unlisteners[subscriptionId]();
            delete unlisteners[subscriptionId];
            return true;
        }
        return false;
    };

    // Sets up a Tauri event listener and stores the unlisten function
    // Called from TauriService.SubscribeToServerStateAsync()
    window.setupTauriListener = async function(dotNetHelper, subscriptionId, eventName) {
        console.log('[Tauri Setup] Setting up listener for:', eventName, 'subscriptionId:', subscriptionId);
        var unlisten = await window.__TAURI__.event.listen(
            eventName,
            async function(event) {
                console.log('[Tauri Event] Received:', eventName, 'Payload:', JSON.stringify(event.payload));
                await dotNetHelper.invokeMethodAsync('HandleEventAsync', event.payload);
            }
        );

        window.__PM_UNLISTENERS__ = window.__PM_UNLISTENERS__ || {};
        window.__PM_UNLISTENERS__[subscriptionId] = unlisten;
        console.log('[Tauri Setup] Listener registered successfully');
    };
})();
```

**Step 1b: Update** `frontend/ProjectManagement.Wasm/wwwroot/index.html`

Replace the inline `<script>` block (lines 34-62) with a script reference:

```html
        <!-- Desktop mode detection (runs before Blazor) -->
        <script src="js/desktop-detection.js"></script>

        <script src="_framework/blazor.webassembly.js"></script>
```

**Why extract to external file?**
- Separates concerns (HTML structure vs JS behavior)
- Easier to test and debug
- Can be minified/bundled separately
- Cleaner index.html

**Why named functions instead of eval?**
- `eval` in Blazor WASM can trigger unexpected type loading issues
- Named functions are faster (no parsing overhead)
- Easier to debug in browser devtools
- Can be tested independently
- Single source of truth for JS interop

**Verification**: Open browser devtools console and run:
```javascript
checkTauriAvailable()  // Should return true in Tauri, false in browser
unlistenTauri('nonexistent')  // Should return false
```

---

### Step 2: Update TauriService Constants and Detection

**File**: `frontend/ProjectManagement.Services/Desktop/TauriService.cs`

**Step 2a**: Update the constants section (replace lines 21-37):

```csharp
// Tauri API paths
private const string TauriInvokePath = "__TAURI__.core.invoke";

// Tauri command names (must match Rust command function names)
private const string CommandGetServerStatus = "get_server_status";
private const string CommandGetWebSocketUrl = "get_websocket_url";
private const string CommandWasmReady = "wasm_ready";
private const string CommandRestartServer = "restart_server";
private const string CommandExportDiagnostics = "export_diagnostics";

// Event names (must match Rust event names in lib.rs)
private const string EventServerStateChanged = "server-state-changed";

// JS interop function names (must match index.html)
private const string JsCheckTauriAvailable = "checkTauriAvailable";
private const string JsSetupTauriListener = "setupTauriListener";
private const string JsUnlistenTauri = "unlistenTauri";
```

**Step 2b**: Replace the `IsDesktopAsync` method (around line 49):

```csharp
/// <summary>
/// Checks if running in Tauri desktop environment.
/// Result is cached. Returns false on any error (graceful degradation).
/// </summary>
public async Task<bool> IsDesktopAsync()
{
    if (_isDesktopCached.HasValue)
        return _isDesktopCached.Value;

    await _initLock.WaitAsync();
    try
    {
        if (_isDesktopCached.HasValue)
            return _isDesktopCached.Value;

        // Use named function instead of eval to avoid TypeLoadException
        var exists = await _js.InvokeAsync<bool>(JsCheckTauriAvailable);

        _isDesktopCached = exists;
        _logger.LogInformation("Desktop mode detected: {IsDesktop}", exists);
        return exists;
    }
    catch (Exception ex)
    {
        _logger.LogDebug(ex, "Tauri detection failed, assuming web mode");
        _isDesktopCached = false;
        return false;
    }
    finally
    {
        _initLock.Release();
    }
}
```

**Step 2c**: Update `SubscribeToServerStateAsync` to use constants (around line 120):

```csharp
await _js.InvokeVoidAsync(
    JsSetupTauriListener,
    ct,
    dotNetRef,
    subscriptionId,
    EventServerStateChanged
);
```

**Step 2d**: Add the `NotifyReadyAsync` method (after `GetWebSocketUrlAsync`, around line 101):

```csharp
/// <summary>
/// Notifies Tauri that WASM is ready and requests current server status.
/// This is the second part of the handshake - called AFTER subscribing to events.
/// </summary>
/// <remarks>
/// The handshake protocol:
/// 1. WASM subscribes to server-state-changed events
/// 2. WASM calls NotifyReadyAsync (this method)
/// 3. Tauri responds with current ServerStatus
/// 4. If server already running, WASM has the port
/// 5. If server still starting, WASM waits for event
///
/// This eliminates the race condition where the server-ready event
/// fires before WASM subscribes.
/// </remarks>
public async Task<ServerStatus> NotifyReadyAsync(CancellationToken ct = default)
{
    ThrowIfDisposed();
    await EnsureDesktopAsync();

    _logger.LogDebug("Sending wasm_ready notification to Tauri");
    return await InvokeTauriAsync<ServerStatus>(CommandWasmReady, ct);
}
```

**Step 2e**: Add a public constant for the unlisten function (for use by TauriEventSubscription):

```csharp
/// <summary>
/// JS function name for unlistening to Tauri events.
/// Public for use by TauriEventSubscription.
/// </summary>
public const string JsUnlistenFunction = JsUnlistenTauri;
```

---

### Step 3: Update TauriEventSubscription to Avoid eval

**File**: `frontend/ProjectManagement.Services/Desktop/TauriEventSubscription.cs`

Replace the entire file:

```csharp
using Microsoft.JSInterop;

namespace ProjectManagement.Services.Desktop;

/// <summary>
/// Manages cleanup of Tauri event subscription.
/// </summary>
internal sealed class TauriEventSubscription : IAsyncDisposable
{
    // JS function name - must match index.html
    private const string JsUnlistenTauri = "unlistenTauri";

    private readonly string _subscriptionId;
    private readonly IJSRuntime _js;
    private readonly Action _onDispose;
    private readonly IDisposable _dotNetRef;
    private bool _disposed;

    public TauriEventSubscription(
        string subscriptionId,
        IJSRuntime js,
        Action onDispose,
        IDisposable dotNetRef)
    {
        _subscriptionId = subscriptionId;
        _js = js;
        _onDispose = onDispose;
        _dotNetRef = dotNetRef;
    }

    public async ValueTask DisposeAsync()
    {
        if (_disposed) return;
        _disposed = true;

        // Use named function instead of eval to avoid TypeLoadException
        try
        {
            await _js.InvokeAsync<bool>(JsUnlistenTauri, _subscriptionId);
        }
        catch
        {
            // Best effort cleanup - subscription may already be gone
        }

        _dotNetRef.Dispose();
        _onDispose();
    }
}
```

**Why this change?**
- The original used `eval` with string interpolation: `$"window.__PM_UNLISTENERS__?.['{_subscriptionId}']?.()"`
- This triggered `TypeLoadException` in some Blazor scenarios
- Named function is cleaner, testable, and avoids the issue

---

### Step 4: Add `Pid` Field to ServerStatus and ServerStateEvent

**File**: `frontend/ProjectManagement.Services/Desktop/ServerStatus.cs`

Add the `Pid` property (after `Port`, around line 44):

```csharp
/// <summary>
/// Server process ID (for debugging orphan processes).
/// </summary>
[JsonPropertyName("pid")]
public uint? Pid { get; init; }
```

**File**: `frontend/ProjectManagement.Services/Desktop/ServerStateEvent.cs`

Add the `Pid` property for consistency:

```csharp
namespace ProjectManagement.Services.Desktop;

/// <summary>
/// Server state event from Tauri backend.
/// </summary>
public sealed record ServerStateEvent
{
    public required string State { get; init; }
    public int? Port { get; init; }
    public string? Error { get; init; }
    public string? WebsocketUrl { get; init; }
    public HealthInfo? Health { get; init; }
    public string? RecoveryHint { get; init; }
    public uint? Pid { get; init; }
    public DateTime Timestamp { get; init; } = DateTime.UtcNow;
}
```

**Why add Pid everywhere?**
- Consistency between commands and events
- Enables debugging orphan process issues
- Can be used to verify the correct server is running
- Useful for diagnostics export

---

### Step 5: Rewrite DesktopConfigService.WaitForServerAsync

**File**: `frontend/ProjectManagement.Services/Desktop/DesktopConfigService.cs`

Replace the entire `WaitForServerAsync` method:

```csharp
public async Task<string> WaitForServerAsync(TimeSpan timeout, CancellationToken ct = default)
{
    using var timeoutCts = new CancellationTokenSource(timeout);
    using var linkedCts = CancellationTokenSource.CreateLinkedTokenSource(ct, timeoutCts.Token);

    _serverReadyTcs = new TaskCompletionSource<bool>(
        TaskCreationOptions.RunContinuationsAsynchronously);

    try
    {
        // STEP 1: Subscribe to events FIRST (before any status check)
        // This ensures we don't miss the server-ready event
        _serverStateSubscriptionId = await _tauriService.SubscribeToServerStateAsync(
            OnServerStateChangedAsync,
            linkedCts.Token);

        _logger.LogDebug("Subscribed to server state events");

        // STEP 2: Send "I'm ready" ping - get current status
        // This eliminates the race condition: we're already subscribed,
        // so if server starts between now and the response, we'll get the event
        var status = await _tauriService.NotifyReadyAsync(linkedCts.Token);
        _logger.LogDebug(
            "Received initial status: State={State}, Port={Port}, Pid={Pid}",
            status.State, status.Port, status.Pid);

        // STEP 3: Check if server is already running
        if (status.State == ServerStateRunning && status.IsHealthy && status.WebsocketUrl != null)
        {
            _logger.LogInformation(
                "Server already running on port {Port} (pid={Pid})",
                status.Port, status.Pid);
            return status.WebsocketUrl;
        }

        // STEP 4: Check for error state
        if (status.State == ServerStateFailed)
        {
            throw new InvalidOperationException(
                status.Error ?? "Server failed to start");
        }

        // STEP 5: Wait for server-ready event
        _logger.LogDebug("Server not ready yet (state={State}), waiting for event...", status.State);

        using (linkedCts.Token.Register(() =>
        {
            if (timeoutCts.IsCancellationRequested)
                _serverReadyTcs.TrySetException(new TimeoutException("Server startup timed out"));
            else
                _serverReadyTcs.TrySetCanceled(ct);
        }))
        {
            await _serverReadyTcs.Task;

            // Get the WebSocket URL from final status
            var finalStatus = await _tauriService.GetServerStatusAsync(linkedCts.Token);

            if (finalStatus.WebsocketUrl == null)
            {
                throw new InvalidOperationException("Server ready but WebSocket URL is null");
            }

            _logger.LogInformation(
                "Server ready on port {Port} (pid={Pid})",
                finalStatus.Port, finalStatus.Pid);
            return finalStatus.WebsocketUrl;
        }
    }
    finally
    {
        // Cleanup subscription
        if (_serverStateSubscriptionId != null)
        {
            await _tauriService.UnsubscribeAsync(_serverStateSubscriptionId);
            _serverStateSubscriptionId = null;
        }
    }
}
```

**Why this order matters:**
1. **Subscribe FIRST** - ensures no events are missed
2. **Ping SECOND** - gets current state while already subscribed
3. **Check response** - if server running, done immediately
4. **Wait for event** - only if server not ready yet

**The key insight**: Between subscribing and the ping response, we can't miss the `server-ready` event because we're already listening.

---

### Step 6: Add Event Name Constants to lib.rs

**File**: `desktop/src-tauri/src/lib.rs`

Add constants after the existing ones (around line 21):

```rust
const PM_SERVER_CONFIG_DIRECTORY_NAME: &str = ".pm";
const PM_SERVER_CONFIG_FILENAME: &str = "config.toml";

// Tauri event names (must match frontend TauriService constants)
const EVENT_SERVER_READY: &str = "server-ready";
const EVENT_SERVER_ERROR: &str = "server-error";
const EVENT_SERVER_STATE_CHANGED: &str = "server-state-changed";
```

**Why constants?**
- Typos in event names cause silent failures (events never arrive)
- Constants are searchable and refactor-safe
- Single source of truth for event names

---

### Step 7: Add `server_pid` Method to ServerManager

**File**: `desktop/src-tauri/src/server/lifecycle.rs`

Add this method to the `impl ServerManager` block (after `health()`, around line 452):

```rust
/// Get server process PID (if running).
pub async fn server_pid(&self) -> Option<u32> {
    self.process
        .lock()
        .await
        .as_ref()
        .map(|child| child.pid())
}
```

**Why expose PID?**
- Debugging: identify orphan processes
- Diagnostics: include in error reports
- Verification: ensure correct process is running

---

### Step 8: Update ServerStatus Struct and build_server_status

**File**: `desktop/src-tauri/src/commands.rs`

**Step 8a**: Update the `ServerStatus` struct (around line 15):

```rust
/// Server status returned to frontend.
#[derive(Debug, Clone, Serialize)]
pub struct ServerStatus {
    pub state: String,
    pub port: Option<u16>,
    pub websocket_url: Option<String>,
    pub health: Option<HealthInfo>,
    pub error: Option<String>,
    pub recovery_hint: Option<String>,
    pub is_healthy: bool,
    pub pid: Option<u32>,
}
```

**Step 8b**: Update `build_server_status` signature and implementation (around line 243):

```rust
/// Converts internal server state to frontend-facing status.
///
/// Shared by `get_server_status` command, `wasm_ready` command, and state change events.
/// Health and pid parameters are optional since state events don't always include them.
pub fn build_server_status(
    state: &ServerState,
    port: Option<u16>,
    ws_url: Option<String>,
    health: Option<&HealthStatus>,
    pid: Option<u32>,
) -> ServerStatus {
    let (state_str, error, recovery_hint) = match state {
        ServerState::Stopped => ("stopped".into(), None, None),
        ServerState::Starting => ("starting".into(), None, None),
        ServerState::Running { .. } => ("running".into(), None, None),
        ServerState::Restarting { attempt } => {
            (format!("restarting (attempt {})", attempt), None, None)
        }
        ServerState::ShuttingDown => ("shutting_down".into(), None, None),
        ServerState::Failed { error } => (
            "failed".into(),
            Some(error.clone()),
            Some("Please check the logs or restart the application.".into()),
        ),
    };

    let is_healthy = state_str == "running"
        && health.map_or(false, |h| matches!(h, HealthStatus::Healthy { .. }));

    ServerStatus {
        state: state_str,
        port,
        websocket_url: ws_url,
        health: health.map(|h| h.into()),
        error,
        recovery_hint,
        is_healthy,
        pid,
    }
}
```

**Step 8c**: Update `get_server_status` to pass pid (around line 78):

```rust
#[tauri::command]
pub async fn get_server_status(
    manager: State<'_, Arc<ServerManager>>,
) -> Result<ServerStatus, String> {
    let state = manager.state().await;
    let port = manager.port().await;
    let ws_url = manager.websocket_url().await;
    let health = manager.health().await;
    let pid = manager.server_pid().await;

    Ok(build_server_status(&state, port, ws_url, health.as_ref(), pid))
}
```

---

### Step 9: Add `wasm_ready` Tauri Command

**File**: `desktop/src-tauri/src/commands.rs`

Add the new command (after `get_websocket_url`, around line 97):

```rust
/// Called by WASM after it subscribes to events.
/// Returns current server status - enables race-free startup handshake.
///
/// The handshake protocol:
/// 1. WASM subscribes to server-state-changed events
/// 2. WASM calls wasm_ready (this command)
/// 3. Tauri responds with current ServerStatus
/// 4. If server already running, WASM has the port
/// 5. If server still starting, WASM waits for event
#[tauri::command]
pub async fn wasm_ready(
    manager: State<'_, Arc<ServerManager>>,
) -> Result<ServerStatus, String> {
    tracing::info!("WASM ready notification received");

    let state = manager.state().await;
    let port = manager.port().await;
    let ws_url = manager.websocket_url().await;
    let health = manager.health().await;
    let pid = manager.server_pid().await;

    Ok(build_server_status(&state, port, ws_url, health.as_ref(), pid))
}
```

---

### Step 10: Update Server Events to Include Full Status

**File**: `desktop/src-tauri/src/lib.rs`

**Step 10a**: Update the server startup block (around line 76):

```rust
// Start server in background
let app_handle = app.handle().clone();
let manager_clone = manager.clone();
tauri::async_runtime::spawn(async move {
    match manager_clone.start(&app_handle).await {
        Ok(()) => {
            info!("Server started successfully");

            // Build full status for frontend
            let state = manager_clone.state().await;
            let port = manager_clone.port().await;
            let ws_url = manager_clone.websocket_url().await;
            let health = manager_clone.health().await;
            let pid = manager_clone.server_pid().await;

            let status = commands::build_server_status(
                &state,
                port,
                ws_url,
                health.as_ref(),
                pid,
            );

            info!(
                "Emitting {} event: port={:?}, pid={:?}",
                EVENT_SERVER_READY, port, pid
            );
            app_handle.emit(EVENT_SERVER_READY, status).ok();
        }
        Err(e) => {
            error!("Failed to start server: {}", e);
            app_handle.emit(EVENT_SERVER_ERROR, e.to_string()).ok();
        }
    }
});
```

**Step 10b**: Update the state subscription block (around line 89):

```rust
// Subscribe to state changes for tray updates
let app_handle = app.handle().clone();
let manager_for_events = manager.clone();
let mut state_rx = manager.subscribe();
tauri::async_runtime::spawn(async move {
    info!("State subscription task started");
    while state_rx.changed().await.is_ok() {
        let state = state_rx.borrow().clone();
        info!("State change detected: {:?}", state);

        // Update tray via TrayManager
        if let Some(tray_mgr) = app_handle.try_state::<Arc<TrayManager>>() {
            tray_mgr.update_status(&app_handle, &state);
        }

        // Emit to frontend - extract data from state to avoid lock contention
        let port = match &state {
            server::ServerState::Running { port } => Some(*port),
            _ => None,
        };
        let ws_url = port.map(|p| format!("ws://127.0.0.1:{}/ws", p));

        // Get PID for state events
        let pid = manager_for_events.server_pid().await;

        let status = commands::build_server_status(
            &state,
            port,
            ws_url,
            None, // Health check happens separately, not in state events
            pid,
        );

        info!("Emitting {}: state={}", EVENT_SERVER_STATE_CHANGED, status.state);
        app_handle.emit(EVENT_SERVER_STATE_CHANGED, status).ok();
    }
});
```

---

### Step 11: Register `wasm_ready` Command

**File**: `desktop/src-tauri/src/lib.rs`

Update the `invoke_handler` (around line 133):

```rust
.invoke_handler(tauri::generate_handler![
    commands::get_server_status,
    commands::get_websocket_url,
    commands::wasm_ready,
    commands::load_user_identity,
    commands::save_user_identity,
    commands::backup_corrupted_user_identity,
    commands::restart_server,
    commands::export_diagnostics,
    commands::get_recent_logs,
])
```

---

### Step 12: Simplify Program.cs

**File**: `frontend/ProjectManagement.Wasm/Program.cs`

Replace the entire file:

```csharp
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
    options.HeartbeatInterval = config.GetValue<TimeSpan?>("HeartbeatInterval")
        ?? TimeSpan.FromSeconds(30);
    options.HeartbeatTimeout = config.GetValue<TimeSpan?>("HeartbeatTimeout")
        ?? TimeSpan.FromSeconds(60);
    options.RequestTimeout = config.GetValue<TimeSpan?>("RequestTimeout")
        ?? TimeSpan.FromSeconds(30);
    options.ReceiveBufferSize = config.GetValue<int?>("ReceiveBufferSize")
        ?? 8192;
});

builder.Services.Configure<CircuitBreakerOptions>(
    builder.Configuration.GetSection("CircuitBreaker"));
builder.Services.Configure<RetryPolicyOptions>(
    builder.Configuration.GetSection("Retry"));
builder.Services.Configure<ReconnectionOptions>(
    builder.Configuration.GetSection("Reconnection"));

// === Logging ===

builder.Services.AddLogging(logging =>
{
    logging.SetMinimumLevel(LogLevel.Debug);
    logging.AddProvider(new CorrelationIdLoggerProvider());
});

// === Desktop Services ===

builder.Services.AddScoped<TauriService>();
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
builder.Services.AddSingleton<AppState>();

// === ViewModels ===

builder.Services.AddScoped<ViewModelFactory>();

// === Radzen ===

builder.Services.AddRadzenComponents();

// === Run ===

var app = builder.Build();
await app.RunAsync();
```

**What changed from the original:**
- Removed Phase 1 temp-host dance (lines 20-59)
- Removed Phase 2 rebuild with discovered URL
- Removed broken `InitializeAsync()` call that failed without user set
- Single host build with clean DI registration
- App.razor handles all startup orchestration via its state machine

**Why this is better:**
- Single host instance (no wasteful temp build/dispose cycle)
- No duplicated server wait logic
- No broken initialization call
- App.razor state machine is the single source of truth for startup

---

### Step 13: Extract ITauriService Interface for Testability

**The Problem**: `DesktopConfigService` depends on concrete `TauriService`, which requires `IJSRuntime`. This makes unit testing difficult without mocking the entire JS runtime.

**The Solution**: Extract an interface and depend on the abstraction.

**File**: `frontend/ProjectManagement.Services/Desktop/ITauriService.cs`

Create a new interface file:

```csharp
namespace ProjectManagement.Services.Desktop;

/// <summary>
/// Interface for Tauri IPC operations.
/// Extracted for testability - allows mocking without IJSRuntime.
/// </summary>
public interface ITauriService : IAsyncDisposable
{
    /// <summary>
    /// Checks if running in Tauri desktop environment.
    /// </summary>
    Task<bool> IsDesktopAsync();

    /// <summary>
    /// Gets current server status from Tauri backend.
    /// </summary>
    Task<ServerStatus> GetServerStatusAsync(CancellationToken ct = default);

    /// <summary>
    /// Gets WebSocket URL for connecting to server.
    /// </summary>
    Task<string> GetWebSocketUrlAsync(CancellationToken ct = default);

    /// <summary>
    /// Notifies Tauri that WASM is ready and requests current server status.
    /// Called AFTER subscribing to events to eliminate race condition.
    /// </summary>
    Task<ServerStatus> NotifyReadyAsync(CancellationToken ct = default);

    /// <summary>
    /// Subscribes to server state change events.
    /// Returns subscription ID for unsubscribing.
    /// </summary>
    Task<string> SubscribeToServerStateAsync(
        Func<ServerStateEvent, Task> callback,
        CancellationToken ct = default);

    /// <summary>
    /// Unsubscribes from server state events.
    /// </summary>
    Task UnsubscribeAsync(string subscriptionId);

    /// <summary>
    /// Requests server restart.
    /// </summary>
    Task RestartServerAsync(CancellationToken ct = default);

    /// <summary>
    /// Exports diagnostics bundle and returns file path.
    /// </summary>
    Task<string> ExportDiagnosticsAsync(CancellationToken ct = default);
}
```

**Why extract an interface?**
- Enables unit testing without `IJSRuntime` mocking
- Follows dependency inversion principle
- Matches existing pattern (`IWebSocketConnection`, `IDesktopConfigService`)

---

### Step 14: Update TauriService to Implement Interface

**File**: `frontend/ProjectManagement.Services/Desktop/TauriService.cs`

Update the class declaration (around line 12):

```csharp
/// <summary>
/// C# wrapper for Tauri IPC commands.
/// Replaces desktop-interop.js with type-safe C# calls.
/// Implements proper resource management and graceful degradation.
/// </summary>
public sealed class TauriService : ITauriService
```

No other changes needed - the class already has all required methods.

---

### Step 15: Update DesktopConfigService to Use Interface

**File**: `frontend/ProjectManagement.Services/Desktop/DesktopConfigService.cs`

Update the constructor and field (around line 7):

```csharp
public sealed class DesktopConfigService : IDesktopConfigService, IAsyncDisposable
{
    private readonly ITauriService _tauriService;
    private readonly ILogger<DesktopConfigService> _logger;
    private string? _serverStateSubscriptionId;
    private TaskCompletionSource<bool>? _serverReadyTcs;

    // Server state constants
    private const string ServerStateRunning = "running";
    private const string ServerStateFailed = "failed";

    public DesktopConfigService(
        ITauriService tauriService,
        ILogger<DesktopConfigService> logger)
    {
        _tauriService = tauriService;
        _logger = logger;
    }
```

**Why use the interface in the constructor?**
- Enables mock injection in tests
- Production code still uses `TauriService` via DI
- No runtime behavior change

---

### Step 16: Update DI Registration

**File**: `frontend/ProjectManagement.Wasm/Program.cs`

Update the Desktop Services section:

```csharp
// === Desktop Services ===

builder.Services.AddScoped<TauriService>();
builder.Services.AddScoped<ITauriService>(sp => sp.GetRequiredService<TauriService>());
builder.Services.AddScoped<IDesktopConfigService, DesktopConfigService>();
builder.Services.AddScoped<UserIdentityService>();
```

**Why register both?**
- `TauriService` is registered so it can be resolved directly if needed
- `ITauriService` is registered to point to the same instance
- `DesktopConfigService` receives `ITauriService` via constructor

---

### Step 17: Add Unit Tests

**File**: `frontend/ProjectManagement.Services.Tests/Desktop/DesktopConfigServiceTests.cs`

Create a new test file with proper mocking:

```csharp
using FluentAssertions;
using Microsoft.Extensions.Logging;
using Moq;
using ProjectManagement.Services.Desktop;

namespace ProjectManagement.Services.Tests.Desktop;

public class DesktopConfigServiceTests
{
    private readonly Mock<ITauriService> _mockTauriService;
    private readonly Mock<ILogger<DesktopConfigService>> _mockLogger;
    private readonly DesktopConfigService _sut;

    public DesktopConfigServiceTests()
    {
        _mockTauriService = new Mock<ITauriService>();
        _mockLogger = new Mock<ILogger<DesktopConfigService>>();
        _sut = new DesktopConfigService(_mockTauriService.Object, _mockLogger.Object);
    }

    [Fact]
    public async Task WaitForServerAsync_WhenServerAlreadyRunning_ReturnsUrlImmediately()
    {
        // Arrange
        var expectedUrl = "ws://127.0.0.1:54321/ws";
        var runningStatus = new ServerStatus
        {
            State = "running",
            Port = 54321,
            WebsocketUrl = expectedUrl,
            IsHealthy = true,
            Pid = 12345
        };

        _mockTauriService
            .Setup(x => x.SubscribeToServerStateAsync(
                It.IsAny<Func<ServerStateEvent, Task>>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync("sub-123");

        _mockTauriService
            .Setup(x => x.NotifyReadyAsync(It.IsAny<CancellationToken>()))
            .ReturnsAsync(runningStatus);

        _mockTauriService
            .Setup(x => x.UnsubscribeAsync("sub-123"))
            .Returns(Task.CompletedTask);

        // Act
        var result = await _sut.WaitForServerAsync(TimeSpan.FromSeconds(5));

        // Assert
        result.Should().Be(expectedUrl);
        _mockTauriService.Verify(
            x => x.SubscribeToServerStateAsync(
                It.IsAny<Func<ServerStateEvent, Task>>(),
                It.IsAny<CancellationToken>()),
            Times.Once);
        _mockTauriService.Verify(
            x => x.NotifyReadyAsync(It.IsAny<CancellationToken>()),
            Times.Once);
    }

    [Fact]
    public async Task WaitForServerAsync_WhenServerNotReady_WaitsForEvent()
    {
        // Arrange
        var expectedUrl = "ws://127.0.0.1:54321/ws";
        var startingStatus = new ServerStatus
        {
            State = "starting",
            Port = null,
            WebsocketUrl = null,
            IsHealthy = false,
            Pid = null
        };
        var runningStatus = new ServerStatus
        {
            State = "running",
            Port = 54321,
            WebsocketUrl = expectedUrl,
            IsHealthy = true,
            Pid = 12345
        };

        Func<ServerStateEvent, Task>? capturedCallback = null;

        _mockTauriService
            .Setup(x => x.SubscribeToServerStateAsync(
                It.IsAny<Func<ServerStateEvent, Task>>(),
                It.IsAny<CancellationToken>()))
            .Callback<Func<ServerStateEvent, Task>, CancellationToken>((cb, _) => capturedCallback = cb)
            .ReturnsAsync("sub-123");

        _mockTauriService
            .Setup(x => x.NotifyReadyAsync(It.IsAny<CancellationToken>()))
            .ReturnsAsync(startingStatus);

        _mockTauriService
            .Setup(x => x.GetServerStatusAsync(It.IsAny<CancellationToken>()))
            .ReturnsAsync(runningStatus);

        _mockTauriService
            .Setup(x => x.UnsubscribeAsync("sub-123"))
            .Returns(Task.CompletedTask);

        // Act
        var waitTask = _sut.WaitForServerAsync(TimeSpan.FromSeconds(5));

        // Simulate server becoming ready
        await Task.Delay(50);
        capturedCallback.Should().NotBeNull();
        await capturedCallback!(new ServerStateEvent { State = "running" });

        var result = await waitTask;

        // Assert
        result.Should().Be(expectedUrl);
    }

    [Fact]
    public async Task WaitForServerAsync_WhenTimeout_ThrowsTimeoutException()
    {
        // Arrange
        var startingStatus = new ServerStatus
        {
            State = "starting",
            Port = null,
            WebsocketUrl = null,
            IsHealthy = false,
            Pid = null
        };

        _mockTauriService
            .Setup(x => x.SubscribeToServerStateAsync(
                It.IsAny<Func<ServerStateEvent, Task>>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync("sub-123");

        _mockTauriService
            .Setup(x => x.NotifyReadyAsync(It.IsAny<CancellationToken>()))
            .ReturnsAsync(startingStatus);

        _mockTauriService
            .Setup(x => x.UnsubscribeAsync("sub-123"))
            .Returns(Task.CompletedTask);

        // Act
        var act = () => _sut.WaitForServerAsync(TimeSpan.FromMilliseconds(100));

        // Assert
        await act.Should().ThrowAsync<TimeoutException>()
            .WithMessage("Server startup timed out");
    }

    [Fact]
    public async Task WaitForServerAsync_WhenServerFailed_ThrowsInvalidOperationException()
    {
        // Arrange
        var failedStatus = new ServerStatus
        {
            State = "failed",
            Port = null,
            WebsocketUrl = null,
            IsHealthy = false,
            Pid = null,
            Error = "Port already in use"
        };

        _mockTauriService
            .Setup(x => x.SubscribeToServerStateAsync(
                It.IsAny<Func<ServerStateEvent, Task>>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync("sub-123");

        _mockTauriService
            .Setup(x => x.NotifyReadyAsync(It.IsAny<CancellationToken>()))
            .ReturnsAsync(failedStatus);

        _mockTauriService
            .Setup(x => x.UnsubscribeAsync("sub-123"))
            .Returns(Task.CompletedTask);

        // Act
        var act = () => _sut.WaitForServerAsync(TimeSpan.FromSeconds(5));

        // Assert
        await act.Should().ThrowAsync<InvalidOperationException>()
            .WithMessage("Port already in use");
    }

    [Fact]
    public async Task WaitForServerAsync_AlwaysUnsubscribes_EvenOnError()
    {
        // Arrange
        _mockTauriService
            .Setup(x => x.SubscribeToServerStateAsync(
                It.IsAny<Func<ServerStateEvent, Task>>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync("sub-123");

        _mockTauriService
            .Setup(x => x.NotifyReadyAsync(It.IsAny<CancellationToken>()))
            .ThrowsAsync(new Exception("Network error"));

        _mockTauriService
            .Setup(x => x.UnsubscribeAsync("sub-123"))
            .Returns(Task.CompletedTask);

        // Act
        var act = () => _sut.WaitForServerAsync(TimeSpan.FromSeconds(5));

        // Assert
        await act.Should().ThrowAsync<Exception>();
        _mockTauriService.Verify(x => x.UnsubscribeAsync("sub-123"), Times.Once);
    }
}
```

### Step 18: Add Rust Unit Tests

**File**: `desktop/src-tauri/src/tests/commands.rs`

Create a new test file:

```rust
//! Unit tests for commands module.

use crate::commands::{build_server_status, ServerStatus};
use crate::server::{HealthStatus, ServerState};

#[test]
fn given_running_state_with_pid_when_build_status_then_includes_pid() {
    let state = ServerState::Running { port: 54321 };
    let pid = Some(12345u32);

    let status = build_server_status(
        &state,
        Some(54321),
        Some("ws://127.0.0.1:54321/ws".into()),
        None,
        pid,
    );

    assert_eq!(status.state, "running");
    assert_eq!(status.port, Some(54321));
    assert_eq!(status.pid, Some(12345));
    assert!(status.websocket_url.is_some());
}

#[test]
fn given_starting_state_when_build_status_then_pid_is_none() {
    let state = ServerState::Starting;

    let status = build_server_status(&state, None, None, None, None);

    assert_eq!(status.state, "starting");
    assert_eq!(status.port, None);
    assert_eq!(status.pid, None);
}

#[test]
fn given_failed_state_when_build_status_then_includes_error_and_hint() {
    let state = ServerState::Failed {
        error: "Connection refused".into(),
    };

    let status = build_server_status(&state, None, None, None, None);

    assert_eq!(status.state, "failed");
    assert!(status.error.is_some());
    assert!(status.recovery_hint.is_some());
    assert!(!status.is_healthy);
}

#[test]
fn given_healthy_running_state_when_build_status_then_is_healthy_true() {
    let state = ServerState::Running { port: 54321 };
    let health = HealthStatus::Healthy {
        latency_ms: 5,
        version: "1.0.0".into(),
    };

    let status = build_server_status(
        &state,
        Some(54321),
        Some("ws://127.0.0.1:54321/ws".into()),
        Some(&health),
        Some(12345),
    );

    assert!(status.is_healthy);
    assert!(status.health.is_some());
}

#[test]
fn given_unhealthy_status_when_build_status_then_is_healthy_false() {
    let state = ServerState::Running { port: 54321 };
    let health = HealthStatus::Unhealthy {
        consecutive_failures: 3,
        last_error: "Connection refused".into(),
    };

    let status = build_server_status(
        &state,
        Some(54321),
        Some("ws://127.0.0.1:54321/ws".into()),
        Some(&health),
        Some(12345),
    );

    assert!(!status.is_healthy);
}

#[test]
fn given_restarting_state_when_build_status_then_includes_attempt() {
    let state = ServerState::Restarting { attempt: 2 };

    let status = build_server_status(&state, None, None, None, None);

    assert!(status.state.contains("restarting"));
    assert!(status.state.contains("2"));
}
```

Update `desktop/src-tauri/src/tests/mod.rs`:

```rust
mod commands;
mod identity;
```

---

## Verification

After implementing all steps, run these commands:

```bash
# Step 1: Verify frontend compiles
dotnet build frontend/ProjectManagement.slnx

# Step 2: Run frontend tests
dotnet test frontend/ProjectManagement.slnx

# Step 3: Verify desktop compiles
cd desktop && cargo check

# Step 4: Run desktop tests
cd desktop && cargo test

# Step 5: Build and run the app
just dev
# or
cd desktop && cargo tauri dev
```

**Manual testing checklist:**
1. Launch app - should start without errors
2. Check Tauri logs - should see "WASM ready notification received"
3. Check browser devtools console - should see subscription and status logs
4. Verify WebSocket connects successfully
5. Quit and restart 5 times - should work consistently (no race conditions)
6. Kill pm-server process manually, verify app detects and handles it

---

## Session 43 Completion Checklist

After completing all steps:

- [ ] `dotnet build frontend/ProjectManagement.slnx` passes
- [ ] `dotnet test frontend/ProjectManagement.slnx` passes (including new DesktopConfigServiceTests)
- [ ] `cd desktop && cargo check` passes
- [ ] `cd desktop && cargo test` passes (including new commands tests)
- [ ] `ITauriService` interface exists and `TauriService` implements it
- [ ] `DesktopConfigService` depends on `ITauriService` (not concrete class)
- [ ] App launches and connects to server consistently
- [ ] No race condition on repeated restarts (test 5+ times)
- [ ] "WASM ready notification received" appears in Tauri logs
- [ ] No `eval` usage remains in TauriService.cs or TauriEventSubscription.cs

### Files Modified (11)

**Frontend (C#):**
- `frontend/ProjectManagement.Wasm/wwwroot/index.html` - Replaced inline script with external file reference
- `frontend/ProjectManagement.Services/Desktop/TauriService.cs` - Implements `ITauriService`, fixed detection, added `NotifyReadyAsync()`, added constants
- `frontend/ProjectManagement.Services/Desktop/TauriEventSubscription.cs` - Replaced `eval` with named function
- `frontend/ProjectManagement.Services/Desktop/ServerStatus.cs` - Added `Pid` property
- `frontend/ProjectManagement.Services/Desktop/ServerStateEvent.cs` - Added `Pid` property
- `frontend/ProjectManagement.Services/Desktop/DesktopConfigService.cs` - Depends on `ITauriService`, rewrote `WaitForServerAsync()`
- `frontend/ProjectManagement.Wasm/Program.cs` - Simplified to single host build, registers `ITauriService`
- `desktop/src-tauri/src/tests/mod.rs` - Added `commands` module

**Desktop (Rust):**
- `desktop/src-tauri/src/commands.rs` - Added `wasm_ready`, updated `build_server_status` and `ServerStatus`
- `desktop/src-tauri/src/server/lifecycle.rs` - Added `server_pid()` method
- `desktop/src-tauri/src/lib.rs` - Added event constants, updated events, registered `wasm_ready` command

### Files Created (4)

**Frontend (JS):**
- `frontend/ProjectManagement.Wasm/wwwroot/js/desktop-detection.js` - Extracted Tauri detection and IPC helpers

**Frontend (C#):**
- `frontend/ProjectManagement.Services/Desktop/ITauriService.cs` - Interface for testability

**Tests:**
- `frontend/ProjectManagement.Services.Tests/Desktop/DesktopConfigServiceTests.cs` - Unit tests with proper Moq mocking
- `desktop/src-tauri/src/tests/commands.rs` - Unit tests for `build_server_status`

---

## Key Concepts Learned

1. **Race Condition Elimination** - Subscribe first, then ping for current state. Can't miss events if you're already listening.

2. **Handshake Protocol** - Two-phase startup: (1) subscribe to events, (2) request current state. Covers both "server first" and "WASM first" scenarios.

3. **Event vs Command** - Events are fire-and-forget broadcasts. Commands are request-response. Use events for notifications, commands for queries.

4. **JavaScript Interop Pitfalls** - `eval` in Blazor WASM can cause unexpected type loading issues. Named functions are safer and more debuggable.

5. **Single-Host Architecture** - Building temporary hosts is wasteful and error-prone. Let the app component handle startup orchestration.

6. **Constants Over Magic Strings** - Event names and command names should be constants. Typos in strings cause silent failures; typos in constants cause compile errors.

7. **Idempotent Queries** - `wasm_ready` returns current state regardless of when called. No side effects, safe to retry.

8. **PID Tracking** - Process IDs enable debugging, diagnostics, and orphan detection.

9. **Test-Driven Changes** - Add tests alongside production code to verify behavior and document intent.

---

## Next Session

**Session 44** (if applicable) could implement:
- Proper sidecar cleanup on Cmd+Q (Tauri macOS limitation)
- Tray menu functionality on macOS Tahoe
- Or move on to other features per roadmap

---

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                         WASM (Blazor)                           │
├─────────────────────────────────────────────────────────────────┤
│  App.razor                                                      │
│    │                                                            │
│    ├─► DesktopConfigService.WaitForServerAsync()               │
│    │     │                                                      │
│    │     ├─► TauriService.SubscribeToServerStateAsync()        │
│    │     │     (1. Subscribe FIRST)                            │
│    │     │                                                      │
│    │     ├─► TauriService.NotifyReadyAsync()                   │
│    │     │     (2. Ping SECOND - get current state)            │
│    │     │                                                      │
│    │     └─► Wait for event OR use response                    │
│    │         (3. Race-free: already subscribed)                │
│    │                                                            │
│    └─► WebSocketClient.ConnectAsync(serverUrl)                 │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
                              │
                    Tauri IPC │ (JSON)
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Tauri (Rust)                               │
├─────────────────────────────────────────────────────────────────┤
│  lib.rs                                                         │
│    │                                                            │
│    ├─► ServerManager.start()                                   │
│    │     └─► spawns pm-server sidecar                          │
│    │                                                            │
│    ├─► emit(EVENT_SERVER_READY, ServerStatus)                  │
│    │     (includes port, websocket_url, pid)                   │
│    │                                                            │
│    └─► commands::wasm_ready()                                  │
│          └─► returns current ServerStatus                      │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
                              │
                   WebSocket  │ (Protobuf)
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                     pm-server (Rust)                            │
├─────────────────────────────────────────────────────────────────┤
│  Axum WebSocket server on dynamic port                         │
│  Handles work items, projects, real-time sync                  │
└─────────────────────────────────────────────────────────────────┘
```
