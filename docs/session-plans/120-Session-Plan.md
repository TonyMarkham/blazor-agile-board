# Session 120: Per-Repo Dynamic Port Discovery

## Context

This project is designed to run as a per-repo tool where the human uses the Tauri/Blazor UI and the LLM uses the CLI. If installed in multiple repos, each repo runs its own `pm-server` instance. The problem: all instances default to port 8000, so the second server fails to bind, and the CLI has no way to discover which port belongs to which repo.

**What already works:** Tauri desktop mode dynamically finds an available port, spawns the server on it, and tells the frontend via IPC. This is fully solved for the desktop flow.

**The gap:** When running the server directly (not via Tauri) + CLI, there's no dynamic port assignment and no discovery mechanism. This is the primary use case for LLM integration.

## Approach: Port File Discovery

The server writes a `server.json` file after binding. The CLI reads it to discover the server URL. This is the same pattern used by LSP servers, Jupyter, webpack-dev-server, etc.

**Flow:**
1. Server starts with `port = 0` (default), OS assigns an available port
2. After `TcpListener::bind()`, server writes `{pid, port, host}` to `<config_dir>/server.json`
3. CLI reads `server.json` to discover server URL (unless `--server` is explicit)
4. CLI validates PID is alive (stale file detection)
5. Server deletes `server.json` on graceful shutdown

## Config Directory Location

The port file is written to `<config_dir>/server.json` where `<config_dir>` is determined by `Config::config_dir()` (`pm-config/src/config.rs:80-90`):

1. `PM_CONFIG_DIR` environment variable if set
2. Otherwise: `<current_working_directory>/.pm/`

**Default location**: `.pm/server.json`

**Note**: The `.server/` directory in the repo root holds a working copy of `config.toml`. It is **not** the config directory used by `Config::config_dir()`. These are separate concerns:
- `.pm/` — runtime data: database (`data.db`), port file (`server.json`)
- `.server/` — configuration template: `config.toml`
- `backend/config.example.toml` — reference copy of config.toml

**Verification**: Run `echo $PM_CONFIG_DIR` to check if the env var is set. If unset, the default `.pm/` is used.

---

## Sub-Session Breakdown

| Session | Scope | Est. Tokens | Status |
|---------|-------|-------------|--------|
| **[120.1](120.1-Session-Plan.md)** | pm-config: port_file module, deps, validation, DEFAULT_PORT, tests | ~25-35k | Pending |
| **[120.2](120.2-Session-Plan.md)** | pm-server + pm-cli: write/read port file, config file updates | ~25-35k | Pending |

---

## Session 120.1: Port File Module & Config Changes (pm-config)

**Files Created:**
- `pm-config/src/port_file.rs` - Port file write/read/cleanup with cross-platform PID liveness check
- `pm-config/src/tests/port_file.rs` - Tests for port file operations

**Files Modified:**
- `pm-config/Cargo.toml` - Add `serde_json`, `chrono`, `libc` workspace deps
- `pm-config/src/lib.rs` - Module declaration, re-export, change DEFAULT_PORT to 0
- `pm-config/src/server_config.rs` - Allow port 0 in validation
- `pm-config/src/tests/mod.rs` - Add port_file test module
- `pm-config/src/tests/server.rs` - Add port 0 validation test

**Verification:** `just check-rs-config && just test-rs-config`

---

## Session 120.2: Server & CLI Integration

**Files Modified:**
- `pm-server/src/main.rs` - Write port file after bind, cleanup on shutdown
- `pm-cli/src/main.rs` - Auto-discover server URL from port file
- `pm-cli/src/cli.rs` - Update --server help text
- `backend/config.example.toml` - Document port 0 auto-assign
- `.server/config.toml` - Change port from 8000 to 0

**Verification:** `just check-backend && just test-backend`

---

## Pre-Implementation Checklist

Before starting **any** sub-session:

- [ ] `just check-backend` passes
- [ ] `just test-backend` passes
- [ ] Database is current (server starts without migration errors)

---

## Files Summary

### Create (2 files)

| File | Purpose |
|------|---------|
| `pm-config/src/port_file.rs` | Port file write/read/remove with cross-platform PID liveness |
| `pm-config/src/tests/port_file.rs` | Unit tests for port file operations |

### Modify (9 files)

| File | Change |
|------|--------|
| `pm-config/Cargo.toml` | Add serde_json, chrono, libc deps |
| `pm-config/src/lib.rs` | Add module + re-export, DEFAULT_PORT -> 0 |
| `pm-config/src/server_config.rs` | Allow port 0 in validation |
| `pm-config/src/tests/mod.rs` | Add port_file module |
| `pm-config/src/tests/server.rs` | Add port 0 test |
| `pm-server/src/main.rs` | Write port file after bind, cleanup on shutdown |
| `pm-cli/src/main.rs` | Auto-discover from port file |
| `pm-cli/src/cli.rs` | Update help text |
| `backend/config.example.toml` | Document port 0 |
| `.server/config.toml` | Change port to 0 |

---

## Final Verification

After both sub-sessions are complete:

```bash
just check-backend
just test-backend
just clippy-backend

# Manual test: start server, verify port file, run CLI
cargo run -p pm-server &
SERVER_PID=$!
sleep 3

# Verify port file was created
echo "=== Port file contents ==="
cat .pm/server.json

# Test CLI auto-discovery
echo "=== CLI auto-discovery ==="
cargo run -p pm-cli -- project list --pretty

# Stop server, verify cleanup
echo "=== Stopping server ==="
kill -TERM $SERVER_PID
wait $SERVER_PID 2>/dev/null
sleep 1

echo "=== After shutdown ==="
if [ ! -f ".pm/server.json" ]; then
    echo "OK: port file removed"
else
    echo "FAIL: port file still exists"
    cat .pm/server.json
fi

# Test CLI with no server running
echo "=== CLI with no server ==="
cargo run -p pm-cli -- project list 2>&1 && echo "FAIL: should have errored" || echo "OK: failed as expected"
```
