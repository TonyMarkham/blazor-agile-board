# Future Enhancements & Technical Debt

This file tracks improvements that are nice-to-have but not required for MVP.

## Desktop UX Enhancements

### Immediate Window Display (Medium Priority) ⚠️
**Anti-Pattern**: Unity Hub shows black screen for 5+ seconds with zero feedback - users think it's frozen and check Task Manager.

**Current Issue**: Tauri window waits for server startup (~600ms) before rendering, creating perception of slowness.

**Psychological Timing Thresholds**:
- 0-100ms: Instant (feels native)
- 100-300ms: Acceptable delay
- 300-1000ms: "Is this working?" (user doubt begins)
- 1000ms+: "Something is broken" (user abandons or force-quits)

**Quick Fix**: Show window immediately, start server in background (non-blocking)
- Open Tauri window on launch (0ms perceived startup)
- Display simple "Starting server..." status message
- Emit Tauri events as server progresses: `server:starting` → `server:ready` → `server:connected`
- Switch to main UI when WebSocket connects successfully

**Implementation**:
- Change `lib.rs` to show window **before** calling `server_manager.start().await`
- Start server in `tauri::async_runtime::spawn()` (non-blocking background task)
- Emit events at each stage using Tauri's event system
- Frontend listens to events and updates status display accordingly
- Show main app only when `server:connected` event fires

**Estimated Effort**: ~1 hour
**Priority**: Medium (core UX issue affecting perceived performance)
**Session**: 40.5 or 41

---

### Eliminate desktop-interop.js (High Priority) 🔥
**Victory Condition**: Zero application JavaScript files. Only Blazor's required bootstrap remains.

**Current Situation**: `desktop-interop.js` (30 lines) is a thin JS wrapper around Tauri APIs:
```
C# (Blazor) → JS (desktop-interop.js) → Tauri API
```

**Better Architecture**: Call Tauri directly from C# via IJSRuntime:
```
C# (Blazor) → Tauri API (direct via __TAURI__ global)
```

**Implementation**:
- Create `Services/TauriService.cs` - C# wrapper for Tauri IPC commands
- Replace `window.DesktopInterop.getServerStatus()` → `await Tauri.GetServerStatus()`
- Replace `window.DesktopInterop.onServerStateChanged()` → `await Tauri.OnServerStateChanged()`
- Replace `window.DesktopInterop.isDesktop()` → `await Tauri.IsDesktop()`
- Use `DotNetObjectReference` for event callbacks (no JS listeners)
- **Delete** `frontend/ProjectManagement.Wasm/wwwroot/js/desktop-interop.js`
- **Delete** `<script src="js/desktop-interop.js"></script>` from index.html

**Result**:
- Zero application JS files in codebase
- Only unavoidable bootstrap: `<script src="_framework/blazor.webassembly.js"></script>`
- All desktop integration logic stays in C#

**Code Example**:
```csharp
// Services/TauriService.cs
public class TauriService
{
    private readonly IJSRuntime _js;

    public async Task<ServerStatus> GetServerStatus()
    {
        return await _js.InvokeAsync<ServerStatus>(
            "__TAURI__.core.invoke",
            "get_server_status"
        );
    }
}

// Component usage
@inject TauriService Tauri

private async Task CheckServer()
{
    var status = await Tauri.GetServerStatus();
    // Pure C# - no JS!
}
```

**Benefits**:
- **Zero JS files to maintain** (just unavoidable Blazor bootstrap)
- Type-safe C# models instead of JS objects
- IntelliSense for all Tauri APIs
- Debuggable C# stack traces (no JS console.log hunting)
- One less file to copy between frontend/desktop directories

**Estimated Effort**: ~30 minutes (simple refactor)
**Priority**: High (eliminates JS, can be done alongside immediate window display)
**Session**: 40.5 or 41 (pair with immediate window display work)

---

### Animated Startup Progress UI (Low Priority)
**Context**: After implementing immediate window display above, enhance with detailed animated progress.

**Proposed**: Replace simple status text with polished Blazor component:
- Animated progress bar with 4 steps (Initialize → Start Server → Check Health → Load UI)
- Error screen with retry button and diagnostics export (all in C#)
- Reconnection overlay when server restarts during session
- Real-time server logs streaming to startup screen
- Graceful degradation if server fails to start

**Benefits**:
- Professional desktop app feel (matches native apps)
- Better debugging for users (see exactly which step failed)
- Handles edge cases gracefully (server crashes, port conflicts, restarts)
- Zero JavaScript (stays true to minimal-JS philosophy)

**Implementation**:
- Create `Components/Desktop/StartupScreen.razor` with state machine
- C# progress tracking with percentage calculations
- CSS animations for smooth transitions
- Tauri commands for retry/diagnostics actions

**Estimated Effort**: ~4 hours
**Priority**: Low (nice-to-have polish after immediate display works)
**Session**: Post-MVP

---

## Known Issues (Safe to Ignore)

### Source Map Warning in Development
**Warning**: `Source Map "http://127.0.0.1:1430/_framework/dotnet.runtime.js.map" has SyntaxError: JSON Parse error: Unrecognized token '<'`
**Cause**: Blazor debug builds + Tauri dev server source map handling
**Impact**: None (cosmetic browser console warning only)
**Fix**: Not required - goes away in Release builds
**Priority**: Ignore

---

## Session 40.5 (Next Up)

- [ ] Build scripts (dev.sh, build.sh)
- [ ] CI/CD pipeline (GitHub Actions)
- [ ] Unit tests for desktop integration
- [ ] Integration tests (end-to-end)
- [ ] Manual test checklist

---

## Future Sessions (Post-MVP)

### Session 50: Sprints & Comments
- Sprint planning UI
- Comment threads
- Real-time collaboration

### Session 60: Time Tracking & Dependencies
- Running timers
- Dependency management
- Circular dependency detection

### Session 70: Activity Logging & Polish
- Activity feed
- Error handling polish
- Loading states
- Documentation

### Session 80+: Advanced Features
- REST API for LLM integration
- Offline support with sync
- Multi-tenant SaaS deployment
- Advanced reporting & analytics
- Import/export (JIRA, CSV)
