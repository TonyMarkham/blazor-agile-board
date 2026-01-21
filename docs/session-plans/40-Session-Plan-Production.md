# Session 40: Tauri Desktop Integration - Production-Grade Plan

## Production-Grade Score Target: 9.4/10

This session creates a production-ready desktop application using Tauri to wrap the Blazor WASM frontend with an embedded pm-server.

**Key Features:**
- Embedded pm-server as sidecar process
- Health monitoring with circuit breaker pattern
- Crash recovery with exponential backoff
- System tray integration with live status
- Cross-platform build pipeline (macOS, Windows, Linux)
- Comprehensive testing suite

---

## Sub-Session Breakdown

This plan has been split into sub-sessions to fit within ~50k token budgets:

| Session | Scope | Est. Tokens | Actual Tokens | Status |
|---------|-------|-------------|---------------|--------|
| **[40.1](40.1-Session-Plan.md)** | Foundation & Error Infrastructure | ~40k | ~95k | ✅ Complete (2026-01-21) |
| **[40.2](40.2-Session-Plan.md)** | Health Monitoring & Lifecycle Management | ~45k | - | Pending |
| **[40.3](40.3-Session-Plan.md)** | Tauri Integration & IPC Commands | ~40k | - | Pending |
| **[40.4](40.4-Session-Plan.md)** | Frontend Integration & Desktop Mode | ~35k | - | Pending |
| **[40.5](40.5-Session-Plan.md)** | Build Pipeline & Testing | ~40k | - | Pending |

---

## Session 40.1: Foundation & Error Infrastructure ✅

**Status**: Complete (2026-01-21)
**Tokens**: ~95k (138% over estimate due to teaching approach)

**Files Created (817 lines):**
- `desktop/src-tauri/Cargo.toml` (44 lines) - Workspace dependencies
- `desktop/src-tauri/build.rs` (3 lines) - Tauri build hook
- `desktop/src-tauri/src/server/mod.rs` (12 lines) - Extended module exports
- `desktop/src-tauri/src/server/error.rs` (155 lines) - `ErrorLocation` pattern with recovery hints
- `desktop/src-tauri/src/server/config.rs` (361 lines) - Versioned config with constants
- `desktop/src-tauri/src/server/port.rs` (75 lines) - Port allocation with constants
- `desktop/src-tauri/src/server/lock.rs` (167 lines) - Single-instance lock with constants

**Enhancements:**
- Uses `ErrorLocation` tracking (follows codebase pattern)
- All magic strings replaced with constants
- Workspace dependency management
- Complete `recovery_hint()` implementation

**Verification:** ✅ `cd desktop/src-tauri && cargo check` (61 warnings, 0 errors)

---

## Session 40.2: Health Monitoring & Lifecycle Management

**Files Created:**
- `desktop/src-tauri/src/server/health.rs` - Health checker with circuit breaker
- `desktop/src-tauri/src/server/lifecycle.rs` - Server process lifecycle

**Files Modified:**
- `desktop/src-tauri/src/server/mod.rs` - Export health and lifecycle modules
- `backend/pm-server/src/main.rs` - Add health endpoints

**Verification:** `cargo check -p pm-server && cd desktop/src-tauri && cargo check`

---

## Session 40.3: Tauri Integration & IPC Commands

**Files Created:**
- `desktop/src-tauri/tauri.conf.json` - Tauri configuration
- `desktop/src-tauri/src/commands.rs` - IPC command handlers
- `desktop/src-tauri/src/tray.rs` - System tray manager
- `desktop/src-tauri/src/logging.rs` - Log rotation setup
- `desktop/src-tauri/src/lib.rs` - Application entry point

**Verification:** `cd desktop/src-tauri && cargo tauri build --debug`

---

## Session 40.4: Frontend Integration & Desktop Mode

**Files Created:**
- `desktop/frontend/index.html` - Desktop frontend with startup UI
- `frontend/ProjectManagement.Wasm/wwwroot/js/desktop-interop.js` - JS interop
- `frontend/ProjectManagement.Services/Desktop/DesktopConfigService.cs` - Desktop detection

**Files Modified:**
- `frontend/ProjectManagement.Wasm/wwwroot/appsettings.json` - WebSocket config
- `frontend/ProjectManagement.Wasm/Program.cs` - Desktop mode detection

**Verification:** `dotnet build frontend/ProjectManagement.Wasm`

---

## Session 40.5: Build Pipeline & Testing

**Files Created:**
- `desktop/scripts/build.sh` - Build script
- `desktop/scripts/dev.sh` - Development script
- `.github/workflows/desktop-build.yml` - CI/CD pipeline
- `desktop/src-tauri/src/server/tests.rs` - Unit tests
- `desktop/src-tauri/tests/integration_tests.rs` - Integration tests
- `desktop/docs/TEST_CHECKLIST.md` - Manual test checklist

**Verification:** `cargo test -p project-manager && ./desktop/scripts/build.sh debug`

---

## Architecture Overview

```
+-------------------------------------------------------------+
|                    Tauri Application                         |
+-------------------------------------------------------------+
|  +-------------+  +-------------+  +---------------------+  |
|  |   Window    |  | System Tray |  |  IPC Commands       |  |
|  |  Manager    |  |   Manager   |  |  (get-status, etc)  |  |
|  +------+------+  +------+------+  +----------+----------+  |
|         |                |                     |             |
|         +----------------+---------------------+             |
|                          |                                   |
|                +---------v---------+                        |
|                |  Server Manager   |                        |
|                |  +-------------+  |                        |
|                |  | Lifecycle   |  |                        |
|                |  | Health Mon. |  |                        |
|                |  | Crash Recov.|  |                        |
|                |  +-------------+  |                        |
|                +---------+---------+                        |
|                          |                                   |
+--------------------------|----------------------------------+
                          | stdin/stdout + signals
                +---------v---------+
                |    pm-server      |<-- Sidecar Process
                |  (SQLite + WS)    |
                +---------+---------+
                          |
                +---------v---------+
                |   .pm/ directory  |
                |  +-- config.toml  |
                |  +-- data.db      |
                |  +-- server.lock  |
                |  +-- logs/        |
                +-------------------+
                          |
                          | WebSocket (127.0.0.1:port)
                          v
+-------------------------------------------------------------+
|                   Blazor WASM Frontend                       |
|  +--------------+  +--------------+  +------------------+   |
|  | Server       |  | Connection   |  | UI Components    |   |
|  | Discovery    |  | Manager      |  | (from Session 30)|   |
|  +--------------+  +--------------+  +------------------+   |
+-------------------------------------------------------------+
```

---

## Design Principles

1. **Fail gracefully** - Every error has a recovery path or clear user guidance
2. **No data loss** - Database integrity preserved in all scenarios
3. **Observable** - Comprehensive logging for debugging and support
4. **Secure by default** - Minimal attack surface, defense in depth
5. **Resilient** - Auto-recovery from transient failures
6. **Testable** - Every component has automated tests

---

## Pre-Implementation Checklist

Before starting **any** sub-session:

- [ ] `cargo test --workspace` passes
- [ ] `dotnet build frontend/ProjectManagement.sln` succeeds
- [ ] Backend server runs: `cargo run -p pm-server`
- [ ] Tauri CLI installed: `cargo install tauri-cli`

---

## Implementation Order (Dependency Graph)

Files must be implemented in this order. Files at the same layer can be implemented in parallel.

```
Layer 0: Prerequisites
+-- backend/pm-server: Add /admin/checkpoint endpoint

Layer 1: Build Configuration
+-- desktop/src-tauri/Cargo.toml

Layer 2: Error Foundation
+-- src/server/error.rs

Layer 3: Core Utilities (parallel, all depend on error.rs only)
+-- src/server/config.rs
+-- src/server/port.rs
+-- src/server/lock.rs

Layer 4: Health Monitoring
+-- src/server/health.rs

Layer 5: Lifecycle Management
+-- src/server/lifecycle.rs
    (depends on: error, config, port, lock, health)

Layer 6: Module Export
+-- src/server/mod.rs
    (declares and re-exports all server/*.rs)

Layer 7: Tauri Configuration
+-- desktop/src-tauri/tauri.conf.json
    (required before Tauri app can run)

Layer 8: Tauri Commands (parallel, all depend on server module)
+-- src/commands.rs
+-- src/tray.rs
+-- src/logging.rs

Layer 9: Application Entry
+-- src/lib.rs (ties everything together)

Layer 10: Frontend Integration (parallel)
+-- desktop/frontend/index.html
+-- frontend/.../desktop-interop.js
+-- frontend/.../DesktopConfigService.cs
+-- frontend/.../appsettings.json
+-- frontend/.../Program.cs

Layer 11: Build Infrastructure (parallel)
+-- scripts/build.sh
+-- scripts/dev.sh
+-- .github/workflows/desktop-build.yml

Layer 12: Testing (parallel)
+-- src/server/tests.rs
+-- tests/integration_tests.rs
+-- docs/TEST_CHECKLIST.md
```

---

## Production-Grade Scoring

| Category | Score | Justification |
|----------|-------|---------------|
| Error Handling | 9.5/10 | Comprehensive error types, recovery hints, transient detection |
| Security | 9/10 | Lock file, local-only binding, config sanitization |
| Logging & Observability | 9.5/10 | Structured logging with rotation, JSON format, diagnostics export |
| Resource Management | 9.5/10 | HTTP + OS signal graceful shutdown, checkpoint, lock cleanup |
| Cross-platform | 9.5/10 | Full Windows support with CTRL_BREAK, HTTP shutdown fallback |
| Testing | 9.5/10 | Unit, integration, manual checklist |
| User Experience | 9.5/10 | Progress UI, live tray status, reconnection overlay, retry |
| Configuration | 9.5/10 | Versioned, migrated, validated, atomic writes |
| Upgrade Path | 9/10 | Config versioning, migration support |
| Edge Cases | 9.5/10 | Single instance, port conflicts, crash recovery |

**Overall: 9.4/10**

---

## Final Verification

After all five sub-sessions are complete:

```bash
# Run all tests
cargo test --workspace
dotnet test frontend/ProjectManagement.sln

# Build desktop app (debug)
./desktop/scripts/dev.sh

# Build desktop app (release)
./desktop/scripts/build.sh release

# Test artifacts
ls desktop/src-tauri/target/release/bundle/
```
