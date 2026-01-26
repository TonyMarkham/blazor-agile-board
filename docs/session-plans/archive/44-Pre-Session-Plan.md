# Session 44: Server Shutdown & Logging Infrastructure

**Prerequisite**: Session 42.5 completed
**Branch**: `feature/fix-server-shutdown` (to be rewritten cleanly on main)
**Status**: Pre-session planning

---

## Problem Statement

The current `feature/fix-server-shutdown` branch contains a working fix for pm-server shutdown issues, but with several code quality problems:

1. **PM_LOG_FILE hack** - Log file path passed via magic environment variable, bypassing config.toml
2. **CARGO_MANIFEST_DIR binary discovery** - Only works in development, breaks production bundles
3. **Unconditional idle shutdown** - Always-on 60s timeout, not configurable
4. **Broken restart handler** - Restart command handler stubbed out with warning message
5. **Sidecar removed** - Changed to standalone process due to macOS Tahoe issues (this part is correct)

This session rewrites the fix properly with config-driven settings and production-grade code.

---

## Key Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Process spawning | **Standalone process** | Required due to macOS Tahoe issues with Tauri sidecar |
| Log file configuration | **config.toml driven** | Proper configuration, not magic env vars |
| Idle shutdown | **Config-driven with WASM ping coordination** | Prevent false shutdowns, configurable per deployment |
| Dual config strategy | **Tauri config for desktop, pm-config for web** | Flexibility for future web deployment |

---

## Architecture

### Config Relationship

```
Desktop Mode:
┌─────────────────────────────────────────────────────────────┐
│ Tauri App                                                   │
│  └── reads: .pm/config.toml (Tauri's ServerConfig)          │
│       └── passes relevant values to pm-server via env vars  │
│            PM_CONFIG_DIR, PM_SERVER_PORT, PM_LOG_FILE, etc  │
└─────────────────────────────────────────────────────────────┘
         │
         ▼ spawns standalone process
┌─────────────────────────────────────────────────────────────┐
│ pm-server                                                   │
│  └── reads: PM_CONFIG_DIR/.pm/config.toml (pm-config)       │
│       └── env vars override config values                   │
└─────────────────────────────────────────────────────────────┘

Future Web Mode:
┌─────────────────────────────────────────────────────────────┐
│ pm-server (standalone)                                      │
│  └── reads: .pm/config.toml (pm-config) directly            │
│       └── no Tauri, no env var overrides needed             │
└─────────────────────────────────────────────────────────────┘
```

### Ping/Pong & Idle Shutdown Coordination

```
WASM Client                    pm-server
     │                              │
     │──── ping ───────────────────>│  (every ping_interval_secs)
     │<─── pong ────────────────────│
     │                              │
     │                              │  idle_shutdown_secs timer resets
     │                              │  on any WebSocket activity
     │                              │
     │  (disconnect)                │
     │                              │  idle_shutdown_secs countdown starts
     │                              │
     │                              │  (shutdown after timeout)
```

**Validation Rule**: `idle_shutdown_secs` MUST be > `ping_interval_secs` to prevent false shutdowns.

---

## Implementation Phases

### Phase 1: pm-config Enhancements

**Goal**: Add file logging and idle shutdown to pm-server's configuration system.

#### 1.1 Update LoggingConfig

**File**: `backend/crates/pm-config/src/logging_config.rs`

Add optional log file field:

```rust
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct LoggingConfig {
    pub level: LogLevel,
    pub dir: String,
    pub file: Option<String>,  // NEW: None = stdout, Some("name.log") = file
    pub colored: bool,
}
```

- Default: `file: None` (stdout logging, backward compatible)
- When set: logs written to `{config_dir}/{dir}/{file}`

#### 1.2 Update ServerConfig

**File**: `backend/crates/pm-config/src/server_config.rs`

Add idle shutdown configuration:

```rust
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub max_connections: u32,
    pub idle_shutdown_secs: u64,  // NEW: 0 = disabled, >0 = auto-shutdown timeout
}
```

- Default: `idle_shutdown_secs: 0` (disabled for standalone/web mode)
- Desktop mode sets via `PM_IDLE_SHUTDOWN_SECS` env var

#### 1.3 Add Environment Variable Overrides

**File**: `backend/crates/pm-config/src/config.rs`

Add to `apply_env_overrides()`:

```rust
// Logging file (full path or filename)
Self::apply_env_option_string("PM_LOG_FILE", &mut self.logging.file);

// Idle shutdown
Self::apply_env_parse("PM_IDLE_SHUTDOWN_SECS", &mut self.server.idle_shutdown_secs);
```

#### 1.4 Update Config Example

**File**: `backend/config.example.toml`

```toml
[server]
host = "127.0.0.1"
port = 8000
max_connections = 10000
# Auto-shutdown when no connections for N seconds (0 = disabled)
# Desktop mode typically sets this via PM_IDLE_SHUTDOWN_SECS env var
idle_shutdown_secs = 0

[logging]
level = "info"
dir = "log"
# Log file name (optional, omit or set to empty for stdout logging)
# Desktop mode typically sets this via PM_LOG_FILE env var
# file = "pm-server.log"
colored = false
```

---

### Phase 2: pm-server Logger Enhancement

**Goal**: Support file-based logging when configured.

#### 2.1 Update Logger Signature

**File**: `backend/pm-server/src/logger.rs`

Change initialization to accept optional file path:

```rust
/// Initialize logger with fern
///
/// # Arguments
/// * `log_level` - Log level filter
/// * `log_file` - Optional path to log file. None = stdout, Some = file output
/// * `colored` - Enable colored output (ignored when logging to file)
#[track_caller]
pub fn initialize(
    log_level: pm_config::LogLevel,
    log_file: Option<PathBuf>,
    colored: bool,
) -> ServerErrorResult<()>
```

Logic:
- `log_file.is_some()` -> write to file, no colors, append mode
- `log_file.is_none()` -> use stdout with optional colors (existing behavior)

#### 2.2 Update main.rs Logger Initialization

**File**: `backend/pm-server/src/main.rs`

```rust
// Construct log file path if configured
let log_file_path = if let Some(ref filename) = config.logging.file {
    let config_dir = pm_config::Config::config_dir()?;
    let log_dir = config_dir.join(&config.logging.dir);

    // Ensure log directory exists
    std::fs::create_dir_all(&log_dir)?;

    Some(log_dir.join(filename))
} else {
    None
};

// Initialize logger
logger::initialize(config.logging.level, log_file_path, config.logging.colored)?;
```

#### 2.3 Add Idle Shutdown Task

**File**: `backend/pm-server/src/main.rs`

```rust
// Idle shutdown (when configured)
if config.server.idle_shutdown_secs > 0 {
    let idle_timeout = config.server.idle_shutdown_secs;
    let registry_for_idle = registry.clone();
    let shutdown_for_idle = shutdown.clone();

    info!("Idle shutdown enabled: {}s timeout", idle_timeout);

    tokio::spawn(async move {
        // Grace period on startup (allow initial connection)
        let grace_period = idle_timeout.min(60);
        info!("Idle shutdown grace period: {}s", grace_period);
        tokio::time::sleep(Duration::from_secs(grace_period)).await;

        let check_interval = (idle_timeout / 2).max(10);

        loop {
            tokio::time::sleep(Duration::from_secs(check_interval)).await;

            if registry_for_idle.total_count().await == 0 {
                info!("No active connections, checking again in {}s...", check_interval);

                // Double-check after another interval
                tokio::time::sleep(Duration::from_secs(check_interval)).await;

                if registry_for_idle.total_count().await == 0 {
                    warn!("No connections for {}s, initiating auto-shutdown", idle_timeout);
                    shutdown_for_idle.shutdown();
                    break;
                } else {
                    info!("Connection established, continuing...");
                }
            }
        }
    });
}
```

---

### Phase 3: Tauri Config Enhancements

**Goal**: Add WASM ping/pong configuration and coordinate with idle shutdown.

#### 3.1 Add ConnectionSettings

**File**: `desktop/src-tauri/src/server/config.rs`

Add new configuration section:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionSettings {
    /// WASM WebSocket ping interval in seconds
    #[serde(default = "default_ping_interval")]
    pub ping_interval_secs: u64,

    /// Server idle shutdown timeout in seconds (0 = disabled)
    /// Must be > ping_interval_secs to avoid false shutdowns
    #[serde(default = "default_idle_shutdown")]
    pub idle_shutdown_secs: u64,
}

fn default_ping_interval() -> u64 { 30 }
fn default_idle_shutdown() -> u64 { 120 }  // 2 minutes > 30s ping

impl Default for ConnectionSettings {
    fn default() -> Self {
        Self {
            ping_interval_secs: default_ping_interval(),
            idle_shutdown_secs: default_idle_shutdown(),
        }
    }
}
```

Add to `ServerConfig`:

```rust
pub struct ServerConfig {
    // ... existing fields ...

    /// Connection and idle settings
    #[serde(default)]
    pub connection: ConnectionSettings,
}
```

#### 3.2 Add Validation

**File**: `desktop/src-tauri/src/server/config.rs`

Add to `validate()`:

```rust
// Idle shutdown must be greater than ping interval to avoid race conditions
if self.connection.idle_shutdown_secs > 0
    && self.connection.idle_shutdown_secs <= self.connection.ping_interval_secs * 2 {
    return Err(ServerError::ConfigInvalid {
        message: format!(
            "idle_shutdown_secs ({}) should be at least 2x ping_interval_secs ({}) to avoid false shutdowns",
            self.connection.idle_shutdown_secs,
            self.connection.ping_interval_secs
        ),
        location: ErrorLocation::from(Location::caller()),
    });
}
```

#### 3.3 Pass Idle Shutdown to pm-server

**File**: `desktop/src-tauri/src/server/lifecycle.rs`

When spawning pm-server, add environment variable:

```rust
cmd.env("PM_IDLE_SHUTDOWN_SECS", self.config.connection.idle_shutdown_secs.to_string())
```

---

### Phase 4: Lifecycle.rs Cleanup

**Goal**: Clean up standalone process spawning while keeping the working parts.

#### 4.1 Binary Discovery

**File**: `desktop/src-tauri/src/server/lifecycle.rs`

Replace `CARGO_MANIFEST_DIR` hack with proper discovery:

```rust
fn find_server_binary(&self) -> ServerResult<PathBuf> {
    // 1. Development override via environment variable
    if let Ok(path) = std::env::var("PM_SERVER_BIN") {
        let path = PathBuf::from(path);
        if path.exists() {
            info!("Using pm-server from PM_SERVER_BIN: {}", path.display());
            return Ok(path);
        }
        warn!("PM_SERVER_BIN set but path doesn't exist: {}", path.display());
    }

    // 2. Bundled location (production) - next to the Tauri executable
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            let bundled = exe_dir.join("pm-server");
            if bundled.exists() {
                info!("Using bundled pm-server: {}", bundled.display());
                return Ok(bundled);
            }

            // macOS .app bundle: Contents/MacOS/pm-server
            #[cfg(target_os = "macos")]
            {
                // Already in MacOS directory if running from .app
                info!("Checked bundled path (not found): {}", bundled.display());
            }
        }
    }

    // 3. Development: target/release or target/debug
    if let Ok(exe) = std::env::current_exe() {
        // Walk up to find workspace root (contains Cargo.toml with [workspace])
        let mut current = exe.parent();
        while let Some(dir) = current {
            let cargo_toml = dir.join("Cargo.toml");
            if cargo_toml.exists() {
                if let Ok(content) = std::fs::read_to_string(&cargo_toml) {
                    if content.contains("[workspace]") {
                        // Found workspace root
                        for profile in ["release", "debug"] {
                            let bin = dir.join("target").join(profile).join("pm-server");
                            if bin.exists() {
                                info!("Using development pm-server: {}", bin.display());
                                return Ok(bin);
                            }
                        }
                    }
                }
            }
            current = dir.parent();
        }
    }

    // 4. System PATH (last resort)
    if let Ok(output) = std::process::Command::new("which").arg("pm-server").output() {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                info!("Using pm-server from PATH: {}", path);
                return Ok(PathBuf::from(path));
            }
        }
    }

    Err(ServerError::ProcessSpawn {
        source: std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "pm-server binary not found. Set PM_SERVER_BIN or ensure it's built.",
        ).into(),
        location: ErrorLocation::from(Location::caller()),
    })
}
```

#### 4.2 Keep Working Parts from Branch

Retain these improvements from the branch:

- `/ready` endpoint polling for startup detection (cleaner than stdout parsing)
- PID tracking instead of process handle (works with detached processes)
- SIGTERM → wait → SIGKILL shutdown sequence
- Detached process spawning with `setsid()`

#### 4.3 Fix Restart Handler

Implement actual restart in command handler instead of stub.

---

### Phase 5: Signal Handling & Shutdown (Keep from Branch)

**Goal**: Keep the working shutdown improvements from the branch.

#### 5.1 Signal Handlers (lib.rs)

Keep the Unix signal handler that intercepts SIGINT/SIGTERM:

```rust
#[cfg(unix)]
{
    let app_handle = app.handle().clone();
    std::thread::spawn(move || {
        use signal_hook::consts::{SIGINT, SIGTERM};
        use signal_hook::iterator::Signals;

        let mut signals = Signals::new(&[SIGINT, SIGTERM])
            .expect("Failed to register signal handlers");

        for sig in signals.forever() {
            info!("Received signal {}, shutting down...", sig);

            if let Some(manager) = app_handle.try_state::<Arc<ServerManager>>() {
                tauri::async_runtime::block_on(async {
                    manager.stop().await.ok();
                });
            }

            std::process::exit(0);
        }
    });
}
```

#### 5.2 ExitRequested Handler (lib.rs)

Keep the `.build().run()` pattern with ExitRequested handler for proper cleanup.

#### 5.3 Blocking Quit (tray.rs)

Keep `block_on` instead of `spawn` for quit to ensure server stops before exit.

#### 5.4 WASM Ready Re-emit (commands.rs)

Keep the fix that re-emits `server-ready` when WASM calls `wasm_ready` and server is already running.

---

### Phase 6: Bundling Configuration

**Goal**: Ensure pm-server is properly bundled in production builds.

#### 6.1 Update tauri.conf.json

Add pm-server to resources (since we're not using sidecar):

```json
{
  "bundle": {
    "resources": {
      "../../target/release/pm-server": "pm-server"
    }
  }
}
```

**Note**: This copies pm-server to the bundle's resources. The binary discovery in Phase 4 will find it.

#### 6.2 Build Coordination

Document the build order requirement:

```bash
# 1. Build pm-server first
cargo build --release -p pm-server

# 2. Build Tauri app (bundles pm-server)
cd desktop && cargo tauri build
```

Consider adding to `justfile` if not already present.

---

## Files to Modify

| File | Changes |
|------|---------|
| **pm-config crate** | |
| `backend/crates/pm-config/src/logging_config.rs` | Add `file: Option<String>` field |
| `backend/crates/pm-config/src/server_config.rs` | Add `idle_shutdown_secs: u64` field |
| `backend/crates/pm-config/src/config.rs` | Add env overrides for new fields |
| `backend/crates/pm-config/src/lib.rs` | Export new defaults if needed |
| **pm-server** | |
| `backend/pm-server/src/logger.rs` | Accept optional file path, implement file output |
| `backend/pm-server/src/main.rs` | Construct log path, add idle shutdown task |
| **Tauri app** | |
| `desktop/src-tauri/src/server/config.rs` | Add `ConnectionSettings` struct |
| `desktop/src-tauri/src/server/lifecycle.rs` | Clean up binary discovery, pass env vars |
| `desktop/src-tauri/src/lib.rs` | Keep signal handlers from branch |
| `desktop/src-tauri/src/tray.rs` | Keep blocking quit from branch |
| `desktop/src-tauri/src/commands.rs` | Keep wasm_ready re-emit fix from branch |
| **Config files** | |
| `backend/config.example.toml` | Document new options |
| `.pm/config.toml` | Add connection settings |
| `desktop/src-tauri/tauri.conf.json` | Add pm-server to resources |

---

## Testing Checklist

### Development Mode

- [ ] `cargo run -p pm-server` starts and logs to stdout
- [ ] `PM_LOG_FILE=test.log cargo run -p pm-server` logs to file
- [ ] `cargo tauri dev` spawns pm-server with file logging
- [ ] Server responds to `/health` and `/ready` endpoints
- [ ] WebSocket connections work in dev mode

### Shutdown Behavior

- [ ] Tray menu "Quit" cleanly stops pm-server
- [ ] Cmd+Q (macOS) cleanly stops pm-server
- [ ] Window close cleanly stops pm-server
- [ ] `kill -TERM <tauri_pid>` cleanly stops pm-server
- [ ] No orphaned pm-server processes after any exit method

### Idle Shutdown

- [ ] Server stays running while WASM is connected
- [ ] WASM ping keeps connection alive
- [ ] Server shuts down after configured idle timeout with no connections
- [ ] Grace period prevents shutdown during initial startup

### Production Bundle

- [ ] `cargo tauri build` includes pm-server in bundle
- [ ] Bundled app starts pm-server correctly
- [ ] Bundled app shuts down cleanly
- [ ] Universal binary works on both Intel and Apple Silicon (macOS)

---

## Rollback Plan

If issues arise during implementation:

1. **Keep branch available**: Don't delete `feature/fix-server-shutdown` until verified
2. **Incremental commits**: Commit after each phase for easy bisection
3. **Feature flags**: Idle shutdown can be disabled via config (set to 0)
4. **Env var override**: `PM_LOG_FILE` override still works as fallback

---

## Success Criteria

Session is complete when:

1. All items in Testing Checklist pass
2. No orphaned pm-server processes after any shutdown method
3. Log file configuration works via config.toml
4. Idle shutdown is configurable and respects WASM ping
5. Code follows existing project patterns (no hacks or magic values)
6. Production bundle works correctly

---

## Estimated Scope

- **Files modified**: ~15
- **New code**: ~300-400 lines
- **Deleted code**: ~200 lines (branch hacks)
- **Net change**: ~100-200 lines added

---

## Future Considerations

1. **Web deployment**: pm-server config works standalone without Tauri
2. **Log rotation**: Currently not implemented, add if log files grow large
3. **Metrics**: Idle shutdown could emit metrics before shutdown
4. **Windows testing**: Signal handling differs, needs verification
