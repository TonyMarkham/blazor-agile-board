# Session Recovery - Changes Made Before Reset

**Date:** 2026-01-23
**Problem Being Solved:** pm-server sidecar not being killed when Tauri app quits

## Working Changes That Were Lost

### 1. Port Signalling Fix (IMPORTANT - KEEP THIS)

**Problem:** Frontend couldn't get pm-server port and PID
**Solution:** Backend now emits complete server info in "server-ready" event

#### Backend Changes (desktop/src-tauri/src/lib.rs)

Around line 76-101, the server startup emits full connection info:

```rust
tauri::async_runtime::spawn(async move {
    match manager_clone.start(&app_handle).await {
        Ok(()) => {
            info!("Server started successfully");

            // Get full connection info
            let port = manager_clone.port().await;
            let ws_url = manager_clone.websocket_url().await;
            let pid = manager_clone.server_pid().await;
            let state = manager_clone.state().await;
            let health = manager_clone.health().await;

            // Build complete server status
            let server_info = commands::build_server_status(
                &state,
                port,
                ws_url,
                health.as_ref(),
                pid,
            );

            // Log before emitting
            info!("Emitting server-ready event with port={:?}, pid={:?}", port, pid);

            // Emit with full connection info
            app_handle.emit("server-ready", server_info).ok();
        }
        Err(e) => {
            error!("Failed to start server: {}", e);
            app_handle.emit("server-error", e.to_string()).ok();
        }
    }
});
```

#### Frontend Changes

**File:** `frontend/ProjectManagement.Services/Desktop/ServerStatus.cs`

Added PID field:
```csharp
public class ServerStatus
{
    public string Status { get; set; } = "unknown";
    public int? Port { get; set; }
    public string? WebSocketUrl { get; set; }
    public string? Error { get; set; }
    public int? Pid { get; set; }  // <-- ADDED THIS
}
```

**File:** `frontend/ProjectManagement.Wasm/wwwroot/index.html`

Added JavaScript function for Tauri detection (around line 20):
```javascript
window.checkTauriAvailable = function() {
    return typeof window.__TAURI__ !== 'undefined' &&
           typeof window.__TAURI__.core !== 'undefined';
};
```

**File:** `frontend/ProjectManagement.Services/Desktop/TauriService.cs`

Changed eval detection to use the new JavaScript function:
```csharp
public async Task<bool> IsTauriAvailableAsync()
{
    try
    {
        var result = await _jsRuntime.InvokeAsync<bool>("checkTauriAvailable");
        return result;
    }
    catch
    {
        return false;
    }
}
```

This fixed the `TypeLoadException` error about `IProducerConsumerCollection`.

### 2. Sidecar Bundling Fix (justfile)

**File:** `justfile`

Around line 17, added copy command for sidecar binary with target triple:
```bash
build-release:
    just setup-config
    cargo build -p pm-server --release & \
    dotnet publish frontend/ProjectManagement.Wasm/ProjectManagement.Wasm.csproj -c Release & \
    wait
    # Copy sidecar with target triple suffix for Tauri bundling
    cp target/release/pm-server "desktop/src-tauri/binaries/pm-server-$(rustc --print host-tuple)"
```

**File:** `desktop/src-tauri/tauri.conf.json`

Added externalBin section:
```json
"bundle": {
  "active": true,
  "icon": [...],
  "externalBin": [
    "binaries/pm-server"
  ],
  "resources": {
    "../../.pm/config.toml": ".pm/config.toml"
  },
  ...
}
```

## Failed Attempts (DO NOT RESTORE)

### Exit Handler Attempts

**Problem:** Cmd+Q doesn't kill pm-server sidecar, leaves orphan process

**What we learned:**
1. Ctrl+C works perfectly (sends SIGTERM, Tauri handles cleanup)
2. Cmd+Q does NOT work (macOS Apple event, Tauri bugs #9198, #12978)
3. `RunEvent::ExitRequested` doesn't fire on macOS for Cmd+Q
4. Window close prevention (`api.prevent_close()`) blocks ALL exits
5. macOS Tahoe has tray menu bugs - custom menus don't show

**Attempted fixes (all failed):**

#### Attempt 1: RunEvent::ExitRequested handler
Added to lib.rs around line 163-175:
```rust
.run(|app, event| {
    if let tauri::RunEvent::ExitRequested { api, .. } = event {
        api.prevent_exit();

        if let Some(manager) = app.try_state::<Arc<ServerManager>>() {
            eprintln!("DEBUG: ExitRequested - killing server");
            manager.kill_child_sync();
            eprintln!("DEBUG: Server killed, now exiting");
        }

        app.exit(0);
    }
});
```
**Result:** Handler never executes on Cmd+Q

#### Attempt 2: kill_child_sync() method
Added to `desktop/src-tauri/src/server/lifecycle.rs` around line 552:
```rust
/// Immediately kill the child process (synchronous, for shutdown).
pub fn kill_child_sync(&self) {
    if let Ok(mut process_guard) = self.process.try_lock() {
        if let Some(mut child) = process_guard.take() {
            tracing::info!("Synchronously killing pm-server PID {}", child.pid());
            let _ = child.kill();
        }
    }
}
```
**Result:** Method is correct, but handlers that call it don't execute

#### Attempt 3: Tray menu quit handler
Modified `desktop/src-tauri/src/tray.rs` around line 85-98:
```rust
"app_exit" => {
    eprintln!("DEBUG: Quit menu clicked");

    if let Some(manager) = app.try_state::<Arc<ServerManager>>() {
        eprintln!("DEBUG: Got manager, calling kill_child_sync");
        manager.kill_child_sync();
        eprintln!("DEBUG: kill_child_sync returned");
    }

    eprintln!("DEBUG: Calling app.exit(0)");
    app.exit(0);
}
```
**Result:** Tray menu doesn't show on macOS Tahoe (shows Dock menu instead)

## Known Issues

### macOS Tahoe Tray Menu Bug
- Custom tray menus don't appear
- Shows default Dock menu instead
- Possibly related to macOS Sequoia/Tahoe "Liquid Glass" design
- May require app entitlements or permissions

### Tauri v2 macOS Limitations
- Issue #9198: ExitRequested doesn't fire for Cmd+Q on macOS
- Issue #12978: Related macOS quit handling
- Issue #2464: Sidecars not cleaned up when using prevent_exit + app.exit()

### What Actually Works
- **Ctrl+C:** Kills both app and pm-server ✓
- **Close window button:** Closes window only (by design, hides to tray)
- **Cmd+Q:** Kills app but orphans pm-server ✗

## Recommendations for Next Session

1. **Keep the port signalling changes** - This is good work that enables PID discovery

2. **Don't try to fix Cmd+Q** - It's a Tauri framework bug, not our code
   - Document that Ctrl+C is the proper way to quit during development
   - Consider it a known limitation of Tauri v2 on macOS

3. **Alternative approaches to explore:**
   - Register Unix signal handlers directly (SIGTERM) using `nix` crate
   - Use `atexit` handlers for cleanup
   - macOS specific: NSApplicationDelegate integration
   - Wait for Tauri v3 or framework fixes

4. **Simplify the design:**
   - Remove window close prevention (let windows close normally)
   - Accept that "hide to tray" might not be needed for v1
   - Focus on making the app work well, not trying to be too clever

## Files Modified This Session

- `desktop/src-tauri/src/lib.rs` - Server startup, exit handlers
- `desktop/src-tauri/src/commands.rs` - build_server_status helper
- `desktop/src-tauri/src/server/lifecycle.rs` - kill_child_sync method
- `desktop/src-tauri/src/tray.rs` - Tray menu quit handler
- `desktop/src-tauri/tauri.conf.json` - externalBin, removed trayIcon section
- `frontend/ProjectManagement.Services/Desktop/ServerStatus.cs` - Added Pid field
- `frontend/ProjectManagement.Services/Desktop/TauriService.cs` - Fixed IsTauriAvailable
- `frontend/ProjectManagement.Wasm/wwwroot/index.html` - Added checkTauriAvailable JS
- `justfile` - Added sidecar binary copy with target triple

## Context from Git

Last good commit: `5f2a0cc feat: Session 42.4 - Startup UI Components`

All changes were uncommitted when reset occurred.

Use `git reflog` to potentially recover the work, then cherry-pick the good changes.
