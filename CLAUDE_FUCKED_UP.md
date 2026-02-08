# Session 120.2 Audit Failure: Incomplete Scope

**Date**: 2026-02-07
**Session**: 120.2 - Server & CLI Integration
**Status**: ❌ FAILED AUDIT - Critical Gap Identified

---

## What Went Wrong

The Session 120.2 plan claims to implement "port file discovery" for the project, but the scope is **incomplete and project-scoped incorrectly**.

### The Plan Says
```
Session 120.2: Server & CLI Integration

Scope:
1. Server - Write port file after bind, clean up on shutdown
2. CLI - Auto-discover server URL from port file
3. Config Files - Update example and active config to use `port = 0`
```

### The Reality

The project has **TWO entry points** that spawn pm-server:

1. **Direct CLI mode** (Session 120.2 scope) ✅
   - User runs: `pm-server` directly
   - Server reads `.server/config.toml` with `port = 0`
   - **Status**: Correctly implemented in 120.2

2. **Tauri Desktop mode** (Session 120.2 scope COMPLETELY MISSED) ❌
   - User runs: `cargo tauri dev`
   - Tauri spawns pm-server with hardcoded environment variable
   - **Location**: `desktop/src-tauri/src/server/config.rs:19`
   - **Problem**: `const DEFAULT_PORT: u16 = 8000;` (hardcoded to 8000)
   - **Evidence**: `desktop/src-tauri/src/server/lifecycle.rs:250` passes `PM_SERVER_PORT=8000` to the subprocess

### Concrete Evidence of the Failure

**File**: `desktop/src-tauri/src/server/config.rs`
```rust
19 | const DEFAULT_PORT: u16 = 8000;  // ← HARDCODED, NOT 0!
```

**File**: `desktop/src-tauri/src/server/lifecycle.rs`
```rust
249-250 | cmd.env("PM_CONFIG_DIR", self.server_dir.to_str().unwrap())
        |    .env("PM_SERVER_PORT", port.to_string())  // ← Passes hardcoded port
```

**Result**: When user runs `cargo tauri dev`, the server.json shows:
```json
{
  "pid": 61083,
  "port": 8000,  // ← Still 8000, NOT auto-assigned!
  "host": "127.0.0.1",
  "started_at": "2026-02-07T20:52:01.210932+00:00",
  "version": "0.1.0"
}
```

---

## Why This Matters

The entire point of Session 120 is to solve **"per-repo dynamic port discovery"**. The session plan says:

> "When running the server directly (not via Tauri) + CLI, there's no dynamic port assignment and no discovery mechanism. **This is the primary use case for LLM integration.**"

But this is only HALF the story:
- ✅ Session 120.2 solves the direct server + CLI case
- ❌ Session 120.2 IGNORES the Tauri desktop case (which is equally important for the project)

**Result**: The project is HALF-fixed. Tauri users still get hardcoded port 8000.

---

## The Missing Requirement

**Session 120.2 should have included:**

### Step 6: Update Tauri Desktop Port Configuration

**File**: `desktop/src-tauri/src/server/config.rs:19`

**Current**:
```rust
const DEFAULT_PORT: u16 = 8000;
```

**Required**:
```rust
const DEFAULT_PORT: u16 = 0;
```

This ensures:
- Tauri desktop mode also uses port 0 (auto-assign)
- Server writes port file to discovery location
- Both entry points (CLI + Tauri) work consistently
- No hardcoded port conflicts in multi-repo setups

---

## Root Cause Analysis

### Why the Audit Missed This

1. **Plan scope was ambiguous**: "Server & CLI Integration" sounds project-wide, but only addressed one entry point
2. **Assumed Tauri "already works"**: The Session 120 plan says "Tauri desktop mode dynamically finds an available port... This is fully solved for the desktop flow" — but it's NOT. The code still has hardcoded port 8000.
3. **Focused on files mentioned in plan**: Only looked at `pm-server/src/main.rs` and `pm-cli/src/main.rs`, not the Tauri code that spawns the server
4. **Didn't check all server spawn points**: Should have searched codebase for all places that start pm-server

### Lessons Learned

- **Session scope must be explicit about what it does NOT cover** — If Tauri is excluded, state that clearly
- **Dynamic ports need ALL callers updated** — If the plan changes DEFAULT_PORT to 0, check all places that reference the port constant
- **Two entry points = twice the work** — CLI mode and Tauri mode need matching implementations
- **"Already works" claims need verification** — The Session 120 plan claimed Tauri already handles dynamic ports, but it doesn't

---

## Impact Assessment

### Current State (After 120.2 implementation)
- ✅ CLI mode: Works correctly, uses port 0, auto-discovers
- ❌ Tauri mode: Still hardcoded to 8000, port file discovery broken
- ⚠️ Project inconsistent: Two different port behaviors

### User Experience
- User runs `pm-server` + `pm` CLI → ✅ Works (ports auto-discovered)
- User runs `cargo tauri dev` → ❌ Broken (hardcoded to 8000)
- User expects "dynamic per-repo ports" → ❌ Only works in CLI mode

---

## What Should Have Happened

Session 120.2 plan should have been:

### Proper Scope

```
Session 120.2: Server & CLI Integration for Dynamic Ports

This session wires port 0 auto-discovery into:
1. Direct server mode (pm-server binary)
2. CLI auto-discovery (pm CLI tool)
3. Tauri desktop mode (cargo tauri dev)

All three paths must use port 0 and discover via port file.
```

### Proper Implementation Steps

1. Update pm-server/src/main.rs (port file write/cleanup)
2. Update pm-cli/src/main.rs (auto-discovery)
3. Update pm-cli/src/cli.rs (help text)
4. Update config files (port = 0)
5. **Update desktop/src-tauri/src/server/config.rs (DEFAULT_PORT = 0)**
6. Manual verification (test all three entry points)

---

## How To Fix

Add this step to Session 120.2:

**File**: `desktop/src-tauri/src/server/config.rs`

**Change line 19 from**:
```rust
const DEFAULT_PORT: u16 = 8000;
```

**To**:
```rust
const DEFAULT_PORT: u16 = 0;
```

**Verification**:
```bash
# Rebuild Tauri
cargo tauri dev

# Check that port file has actual assigned port (not 8000)
cat '/Users/tony/Library/Application Support/com.projectmanager.app/.server/server.json'

# Verify port is NOT 8000
```

---

## Session Status

- **120.1**: ✅ Complete
- **120.2 (Scope 1: Direct Server)**: ✅ Complete
- **120.2 (Scope 2: CLI)**: ✅ Complete
- **120.2 (Scope 3: Tauri Desktop)**: ❌ **MISSING - Critical Gap**

Session 120.2 is **INCOMPLETE** until the Tauri code is updated.

---

## Preventing This In Future Sessions

1. **Always search for all callers** of modified constants/functions
2. **List all entry points** for the feature being implemented
3. **Test all entry points** during verification
4. **Don't trust "already works" claims** without verification
5. **Session scope should explicitly state what's IN and OUT**
