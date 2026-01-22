# Session 42: Desktop Enhancements - User Identity & Persistent Storage

## Production-Grade Score Target: 9.3/10

This session implements persistent user identity for desktop mode, eliminating the bug where users lose access to their projects after restarting the app.

---

## The Problem

When users close and reopen the desktop app, they lose access to all their projects. This happens because the app generates a new random user ID on every launch instead of remembering who you are.

## The Solution

Store a persistent user identity locally on the device. On first launch, show a simple registration screen. On subsequent launches, load the saved identity and reconnect with the same user ID.

---

## Sub-Session Breakdown

This plan has been split into sub-sessions to fit within token budgets:

| Session | Scope | Est. Tokens | Status |
|---------|-------|-------------|--------|
| **[42.1](42.1-Session-Plan.md)** | Core Identity Models & Backend | ~88k | ✅ Complete |
| **[42.2](42.2-Session-Plan.md)** | Identity Service & App State Machine | ~40-45k | Pending |
| **[42.3](42.3-Session-Plan.md)** | Tauri Service (JS Elimination) | ~35-40k | Pending |
| **[42.4](42.4-Session-Plan.md)** | Startup UI Components | ~40-45k | Pending |
| **[42.5](42.5-Session-Plan.md)** | Build Scripts, CI/CD & Testing | ~35-40k | Pending |

---

## Session 42.1: Core Identity Models & Backend

**Teaching Focus:** Data modeling, schema versioning, Rust/Tauri integration

**Files Created (10):**
- `frontend/ProjectManagement.Core/Models/UserIdentity.cs` - Identity model with schema versioning
- `frontend/ProjectManagement.Core/Validation/RegistrationValidator.cs` - Validation logic (Note: ValidationResult.cs already existed)
- `desktop/src-tauri/src/identity/error.rs` - Production-grade error types
- `desktop/src-tauri/src/identity/load_result.rs` - Three-state load result
- `desktop/src-tauri/src/identity/user_identity.rs` - Rust identity struct
- `desktop/src-tauri/src/identity/mod.rs` - Load/save/backup with atomic writes
- `desktop/src-tauri/src/tests/mod.rs` - Test module declaration
- `desktop/src-tauri/src/tests/identity.rs` - 10 unit tests
- `backend/crates/pm-ws/src/handlers/connection.rs` - Backend security validation
- `backend/crates/pm-ws/src/tests/connection.rs` - 8 unit tests with RAII pattern

**Files Modified (4):**
- `desktop/src-tauri/src/commands.rs` - Added 3 identity commands
- `desktop/src-tauri/src/lib.rs` - Registered commands in invoke_handler
- `backend/crates/pm-ws/src/handlers/mod.rs` - Exported connection module
- `backend/crates/pm-ws/src/tests/mod.rs` - Declared connection tests

**Verification:** ✅ All tests passing (203 total), all builds clean

---

## Session 42.2: Identity Service & App State Machine

**Teaching Focus:** Service architecture, retry patterns, state machines, async disposal

**Files Created:**
- `frontend/ProjectManagement.Services/Desktop/UserIdentityService.cs` - Service with retry logic
- `frontend/ProjectManagement.Core/State/AppStartupState.cs` - State machine

**Files Modified:**
- `frontend/ProjectManagement.Services/State/AppState.cs` - Add user context
- `frontend/ProjectManagement.Services/WebSocket/WebSocketClient.cs` - Add user_id param

**Verification:** `dotnet build frontend/ProjectManagement.sln`

---

## Session 42.3: Tauri Service (JS Elimination)

**Teaching Focus:** C#/JS interop, resource management, event subscriptions

**Files Created:**
- `frontend/ProjectManagement.Services/Desktop/TauriService.cs` - Replaces desktop-interop.js
- `frontend/ProjectManagement.Services/Desktop/DesktopConfigService.cs` - Server waiting logic

**Files Deleted:**
- `desktop/frontend/wwwroot/js/desktop-interop.js`

**Verification:** `dotnet build && tauri dev` (no JS errors in console)

---

## Session 42.4: Startup UI Components

**Teaching Focus:** Blazor components, CSS styling, accessibility, state-driven UI

**Files Created:**
- `StartupScreen.razor` + CSS - Progress indication
- `UserRegistrationScreen.razor` + CSS - Form with validation
- `ErrorScreen.razor` + CSS - Error recovery

**Files Modified:**
- `frontend/ProjectManagement.Wasm/App.razor` - State machine integration

**Verification:** Visual inspection, keyboard navigation test

---

## Session 42.5: Build Scripts, CI/CD & Testing

**Teaching Focus:** Cross-platform automation, CI/CD best practices, comprehensive testing

**Files Created:**
- Build scripts: `dev.sh`, `dev.ps1`, `build.sh`, `build.ps1`
- `.github/workflows/desktop-build.yml` - Matrix build
- Unit tests for UserIdentityService and validation
- `desktop/TESTING.md` - Manual test checklist

**Verification:** `dotnet test && ./dev.sh`

---

## Quality Standards Applied

| Standard | Implementation |
|----------|---------------|
| **Retry with backoff** | Transient failures retry 3 times with 100ms, 500ms, 1s delays |
| **Thread-safe locks** | SemaphoreSlim prevents concurrent identity operations |
| **Double-click protection** | Buttons disabled during async operations |
| **Schema versioning** | Identity file includes version for future migrations |
| **Graceful degradation** | If Tauri APIs fail, assume web mode and continue |
| **IAsyncDisposable** | All services properly clean up resources |

---

## Files Summary

### Create (22 files)

| File | Purpose |
|------|---------|
| `UserIdentity.cs` | Persistent identity model |
| `ValidationResult.cs` | Validation helpers |
| `UserIdentityService.cs` | Identity CRUD with retry |
| `TauriService.cs` | C# Tauri interop |
| `DesktopConfigService.cs` | Server lifecycle |
| `AppStartupState.cs` | State machine |
| `StartupScreen.razor` + CSS | Loading UI |
| `UserRegistrationScreen.razor` + CSS | Registration form |
| `ErrorScreen.razor` + CSS | Error recovery |
| `commands.rs` (additions) | Tauri commands |
| `connection.rs` | Security validation |
| `dev.sh`, `dev.ps1` | Dev build scripts |
| `build.sh`, `build.ps1` | Prod build scripts |
| `desktop-build.yml` | CI workflow |
| Tests + `TESTING.md` | Test coverage |

### Modify (6 files)

| File | Change |
|------|--------|
| `App.razor` | State machine integration |
| `AppState.cs` | User context |
| `WebSocketClient.cs` | User ID parameter |
| `src-tauri/src/lib.rs` | Register commands |
| `index.html` | Remove JS script |

### Delete (1 file)

| File | Reason |
|------|--------|
| `desktop-interop.js` | Replaced by TauriService.cs |

---

## Success Criteria

After implementation, these must all be true:
1. User creates project → closes app → reopens → project still accessible
2. Zero JavaScript files (except Blazor bootstrap)
3. Startup shows progress within 100ms
4. Invalid email shows inline error message
5. Server crash shows error screen with Retry button
6. All unit tests pass
7. Manual checklist 100% complete

---

## Estimated Effort

| Phase | Hours |
|-------|-------|
| 42.1 Core Models & Backend | 3-4 |
| 42.2 Identity Service & State | 3-4 |
| 42.3 Tauri Service (JS Elimination) | 2-3 |
| 42.4 Startup UI Components | 3-4 |
| 42.5 Build Scripts & Testing | 4-5 |
| **Total** | **15-20** |

---

## Pre-Implementation Checklist

Before starting **any** sub-session:

- [ ] `dotnet build frontend/ProjectManagement.sln` passes
- [ ] `cargo check --workspace` passes
- [ ] Desktop app launches: `cd desktop && cargo tauri dev`

---

## Final Verification

After all five sub-sessions are complete:

```bash
# Backend check
cargo check --workspace
cargo test --workspace

# Frontend check
dotnet build frontend/ProjectManagement.sln
dotnet test

# Desktop build
cd desktop && cargo tauri build

# Manual testing
# Follow desktop/TESTING.md checklist
```
