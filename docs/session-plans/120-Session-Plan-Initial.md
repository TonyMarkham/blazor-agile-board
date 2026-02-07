# Plan: Per-Repo Dynamic Port Discovery

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

## Changes

### 1. Allow port 0 in validation
**File:** `backend/crates/pm-config/src/server_config.rs`

Update `validate()` to accept port 0 (OS auto-assign):
```rust
if self.port != 0 && self.port < MIN_PORT {
    // port 0 = auto-assign, otherwise must be >= 1024
}
```

### 2. Change default port to 0
**File:** `backend/crates/pm-config/src/lib.rs` (line 47)

Change `DEFAULT_PORT` from `8000` to `0`. Existing installations with explicit `port = 8000` in config.toml are unaffected. Only new installations with no config file get auto-assign behavior.

### 3. Add `port_file` module to pm-config
**New file:** `backend/crates/pm-config/src/port_file.rs`

Provides `PortFileInfo` struct with `write()`, `read_live()`, `remove()`. Lives in pm-config because both server and CLI need it.

- JSON format: `{ "pid": 12345, "port": 49152, "host": "127.0.0.1", "started_at": "...", "version": "..." }`
- `read_live()` checks PID liveness via `libc::kill(pid, 0)` on Unix; auto-removes stale files
- File location: `<config_dir>/server.json` (same dir as `config.toml` and `data.db`)

**File:** `backend/crates/pm-config/src/lib.rs` — add `mod port_file` and `pub use`

**File:** `backend/crates/pm-config/Cargo.toml` — add `serde_json`, `chrono`, `libc` (all already in workspace deps)

### 4. Server writes port file after bind
**File:** `backend/pm-server/src/main.rs`

After `TcpListener::bind()` (line 187):
- Call `listener.local_addr()` to get the actual bound port
- Call `PortFileInfo::write(actual_port, &config.server.host)` (non-fatal on error)
- Log the actual address

After `axum::serve(...).await?` returns (line 251):
- Call `PortFileInfo::remove()` for cleanup

### 5. CLI auto-discovers server from port file
**File:** `backend/crates/pm-cli/src/main.rs`

Replace lines 43-46 (hardcoded default) with:
- If `--server` provided → use it
- Else → call `PortFileInfo::read_live()`
  - If found → construct URL from `host` and `port`
  - If not found → print helpful error ("No running pm-server found. Start one with `pm-server`, or use `--server <url>`") and exit

**File:** `backend/crates/pm-cli/src/cli.rs` — update help text for `--server` flag

### 6. Update config example and existing config
**File:** `backend/config.example.toml` — update port comment to document `0 = auto-assign`

**File:** `.server/config.toml` — change `port = 8000` to `port = 0`

### 7. Tests
**File:** `backend/crates/pm-config/src/tests/port_file.rs` (new)

- Write and read back, verify fields
- Read non-existent returns None
- `read_live()` with current PID returns Some
- `read_live()` with dead PID returns None and cleans up stale file
- Remove non-existent succeeds

**File:** `backend/crates/pm-config/src/tests/server.rs` — add test that port 0 passes validation

## Implementation Order

1. `pm-config/Cargo.toml` — add deps
2. `pm-config/src/port_file.rs` — new module
3. `pm-config/src/lib.rs` — export + change DEFAULT_PORT
4. `pm-config/src/server_config.rs` — allow port 0
5. `pm-config/src/tests/` — port file + validation tests
6. `pm-server/src/main.rs` — write/cleanup port file, use local_addr()
7. `pm-cli/src/main.rs` + `cli.rs` — auto-discovery
8. Config files — update docs/comments
9. `just check-backend && just test-backend` to verify

## Verification

1. `just test-backend` — all existing + new tests pass
2. `just clippy-backend` — no warnings
3. Manual: Start server from repo root, verify `server.json` appears in config dir with correct port
4. Manual: Run `just run-cli project list --pretty` without `--server` — should auto-discover
5. Manual: Stop server, verify `server.json` is cleaned up
6. Manual: Run CLI with no server running — should get helpful error message
