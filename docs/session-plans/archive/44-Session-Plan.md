# Session 44: Server Shutdown & Logging Infrastructure

**Prerequisites**: Session 42.5 completed, `cargo check --workspace` passes
**Source**: `stash@{0}` contains working code with quality issues to fix
**Target**: ~50-60k tokens
**Status**: Implementation Complete - Testing/Verification Phase

---

## Progress Tracking

| Step | Description | Status |
|------|-------------|--------|
| 1 | Add signal-hook dependency | ✅ Done |
| 2 | Update LoggingConfig with file field | ✅ Done |
| 3 | Update ServerConfig with idle_shutdown_secs | ✅ Done |
| 4 | Add environment variable overrides | ✅ Done |
| 5 | Update config.example.toml | ✅ Done |
| 6 | Update logger signature | ✅ Done |
| 7 | Update main.rs logger initialization | ✅ Done |
| 8 | Add configurable idle shutdown | ✅ Done |
| 9 | Add ConnectionSettings to Tauri config | ✅ Done |
| 10 | Complete lifecycle.rs refactor | ✅ Done |
| 11 | Update lib.rs directory setup | ✅ Done |
| 12 | Add signal handlers to lib.rs | ✅ Done |
| 13 | Add ExitRequested handler | ✅ Done |
| 14 | Update tray.rs quit handler | ✅ Done |
| 15 | Update wasm_ready command | ✅ Done |
| 16 | Add quit_app command | ✅ Done |
| 17 | Update bundling config | ✅ Done |

**Phases Completed**: 1-8 (All implementation phases)
**Current Phase**: Testing & Verification

---

## Issues Discovered During Testing

### Issue 1: WASM TypeLoadException on Startup

**Symptom**: Blazor WASM times out waiting for server on first launch. Clicking "Retry" works.

**Console Error**:
```
[Error] Unhandled Promise Rejection: Error: System.TypeLoadException: Could not resolve type
with token 0100001d from typeref (expected class 'System.Diagnostics.DebuggerStepThroughAttribute'
in assembly 'System.Runtime, Version=10.0.0.0...
```

**Root Cause**: Stale .NET build artifacts. The `server-state-changed` event IS received by JS,
but the callback into .NET fails due to incompatible cached assemblies.

**Solution**: Clean rebuild
```bash
just clean
just build-dev
```

**References**:
- [Telerik KB: TypeLoadException](https://www.telerik.com/blazor-ui/documentation/knowledge-base/common-could-not-resolve-type-with-token)
- [HAVIT KB: WASM AggregateException](https://knowledge-base.havit.eu/2025/05/28/wasm-aggregateexception_ctor_defaultmessage-could-not-resolve-type-with-token/)

### Issue 2: Config Validation Failure

**Symptom**: App crashes on startup with panic:
```
Config error: Configuration invalid: idle_shutdown_secs (60) must be > 2x ping_interval_secs (30)
to avoid false shutdowns
```

**Root Cause**: Existing config file created before validation was added. The validation requires
`idle_shutdown_secs > 2 * ping_interval_secs`, so 60 is not > 60.

**Solutions**:
1. Lower `ping_interval_secs` to 29 (then 60 > 58 ✓)
2. Increase `idle_shutdown_secs` to 61+
3. Delete config to regenerate with valid defaults (120)

**Config Location**: `~/Library/Application Support/com.projectmanager.app/.tauri/config.toml`

---

## Scope

This session cleans up and properly integrates server shutdown fixes from the stash:

1. **Directory restructure**: Separate `.server/` and `.tauri/` directories
2. **pm-config**: Add `file` to LoggingConfig, `idle_shutdown_secs` to ServerConfig
3. **pm-server logger**: Config-driven file logging (not env var hack)
4. **pm-server main**: Configurable idle shutdown (not hardcoded 60s)
5. **Tauri config**: Add ConnectionSettings with validation
6. **lifecycle.rs**: Fix binary discovery for production, keep standalone process spawning
7. **lib.rs**: Signal handlers, ExitRequested handler
8. **tray.rs**: Blocking quit
9. **commands.rs**: wasm_ready re-emit, quit_app command
10. **Bundling**: Add pm-server to resources

---

## Directory Structure

**Before** (confusing - shared config file):
```
~/Library/Application Support/com.projectmanager.app/
├── .pm/
│   ├── config.toml      ← Both trying to use this!
│   ├── data.db
│   ├── logs/
│   └── server.lock
└── user.json
```

**After** (clean separation):
```
~/Library/Application Support/com.projectmanager.app/
├── .server/                    ← pm-server (backend)
│   ├── config.toml             ← pm-server config
│   ├── data.db
│   ├── logs/
│   │   └── pm-server.log
│   └── server.lock
├── .tauri/                     ← Tauri desktop app
│   ├── config.toml             ← Tauri's ServerConfig
│   └── logs/
│       └── pm-desktop.YYYY-MM-DD.log
└── user.json                   ← Tauri internal
```

---

## Implementation Order

The steps are organized into phases. Each phase can be verified independently without intermediate compilation failures.

---

## Phase 1: Dependencies & Setup

### Step 1: Add signal-hook Dependency

**File**: `Cargo.toml` (workspace root)

Add to `[workspace.dependencies]`:
```toml
signal-hook = { version = "0.3.18" }
```

**File**: `desktop/src-tauri/Cargo.toml`

Add under `[target.'cfg(unix)'.dependencies]`:
```toml
signal-hook = { workspace = true }
```

**Verification**: `cargo check -p project-manager`

---

## Phase 2: pm-config Changes (Backend)

These changes are independent and can be verified after each step.

### Step 2: Update LoggingConfig

**File**: `backend/crates/pm-config/src/logging_config.rs`

Add `file` field to struct:
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

Update `Default` impl:
```rust
impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: LogLevel::default(),
            dir: DEFAULT_LOG_DIRECTORY.to_string(),
            file: None,  // NEW
            colored: DEFAULT_LOG_COLORED,
        }
    }
}
```

**Verification**: `cargo check -p pm-config`

---

### Step 3: Update ServerConfig

**File**: `backend/crates/pm-config/src/server_config.rs`

Add `idle_shutdown_secs` field:
```rust
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub max_connections: usize,
    pub idle_shutdown_secs: u64,  // NEW: 0 = disabled
}
```

Update `Default` impl:
```rust
impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: DEFAULT_SERVER_HOST.to_string(),
            port: DEFAULT_SERVER_PORT,
            max_connections: DEFAULT_MAX_CONNECTIONS,
            idle_shutdown_secs: 0,  // NEW: disabled by default
        }
    }
}
```

**Verification**: `cargo check -p pm-config`

---

### Step 4: Add Environment Variable Overrides

**File**: `backend/crates/pm-config/src/config.rs`

In `apply_env_overrides()` method, add after existing overrides:
```rust
// Logging file (optional filename)
Self::apply_env_option_string("PM_LOG_FILE", &mut self.logging.file);

// Server idle shutdown
Self::apply_env_parse("PM_IDLE_SHUTDOWN_SECS", &mut self.server.idle_shutdown_secs);
```

**Verification**: `cargo check -p pm-config && cargo test -p pm-config`

---

### Step 5: Update config.example.toml

**File**: `backend/config.example.toml`

Add under `[server]`:
```toml
# Auto-shutdown when no connections for N seconds (0 = disabled)
# Desktop mode sets this via PM_IDLE_SHUTDOWN_SECS env var
idle_shutdown_secs = 0
```

Add under `[logging]`:
```toml
# Optional log file name (omit for stdout logging)
# Desktop mode sets this via PM_LOG_FILE env var
# file = "pm-server.log"
```

**Verification**: N/A (documentation only)

---

## Phase 3: pm-server Changes (Backend)

Depends on Phase 2 (pm-config changes).

### Step 6: Update Logger Signature

**File**: `backend/pm-server/src/logger.rs`

Change signature to accept optional file path:
```rust
use std::path::PathBuf;

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
) -> ServerErrorResult<()> {
    let level_filter = log_level.0;
    let base_dispatch = Dispatch::new().level(level_filter);

    let dispatch = if let Some(ref log_path) = log_file {
        // File output (no colors, plain format)
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)
            .map_err(|e| ServerError::EnvVar {
                message: format!("Failed to open log file {}: {}", log_path.display(), e),
            })?;

        Dispatch::new()
            .format(|out, message, record| {
                out.finish(format_args!(
                    "[{date} - {level}] {message} [{file}:{line}]",
                    date = humantime::format_rfc3339(SystemTime::now()),
                    level = record.level(),
                    message = message,
                    file = record.file().unwrap_or("unknown"),
                    line = record.line().unwrap_or(0),
                ))
            })
            .chain(file)
    } else if colored {
        // Colored stdout (existing code)
        // ... keep existing colored implementation
    } else {
        // Plain stdout (existing code)
        // ... keep existing plain implementation
    };

    // ... rest of existing function
}
```

**Note**: Keep the existing colored and plain stdout implementations, just add the file branch.

**Verification**: `cargo check -p pm-server`

---

### Step 7: Update main.rs Logger Initialization

**File**: `backend/pm-server/src/main.rs`

Replace logger initialization with config-driven path construction:
```rust
// Construct log file path if configured
let log_file_path: Option<std::path::PathBuf> = if let Some(ref filename) = config.logging.file {
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

**Verification**: `cargo check -p pm-server`

---

### Step 8: Add Configurable Idle Shutdown

**File**: `backend/pm-server/src/main.rs`

Add after the existing signal handler setup, before server start:
```rust
// Clone registry for idle monitoring before moving into AppState
let registry_for_idle = registry.clone();

// ... existing AppState creation ...

// Idle shutdown (when configured)
if config.server.idle_shutdown_secs > 0 {
    let idle_timeout = config.server.idle_shutdown_secs;
    let shutdown_for_idle = shutdown.clone();

    info!("Idle shutdown enabled: {}s timeout", idle_timeout);

    tokio::spawn(async move {
        // Grace period on startup (allow initial connection)
        let grace_period = idle_timeout.min(60);
        info!("Idle shutdown grace period: {}s", grace_period);
        tokio::time::sleep(std::time::Duration::from_secs(grace_period)).await;

        let check_interval = (idle_timeout / 2).max(10);

        loop {
            tokio::time::sleep(std::time::Duration::from_secs(check_interval)).await;

            if registry_for_idle.total_count().await == 0 {
                info!("No active connections, checking again in {}s...", check_interval);

                tokio::time::sleep(std::time::Duration::from_secs(check_interval)).await;

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

**Verification**: `cargo check -p pm-server && cargo test -p pm-server`

---

## Phase 4: Tauri Config

Independent of backend changes.

### Step 9: Add ConnectionSettings to Tauri Config

**File**: `desktop/src-tauri/src/server/config.rs`

Add new struct after existing structs:
```rust
/// Connection and idle shutdown settings
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
fn default_idle_shutdown() -> u64 { 120 }

impl Default for ConnectionSettings {
    fn default() -> Self {
        Self {
            ping_interval_secs: default_ping_interval(),
            idle_shutdown_secs: default_idle_shutdown(),
        }
    }
}
```

Add field to `ServerConfig`:
```rust
pub struct ServerConfig {
    // ... existing fields ...

    /// Connection and idle settings
    #[serde(default)]
    pub connection: ConnectionSettings,
}
```

Add validation in `validate()`:
```rust
// Idle shutdown must be > 2x ping interval to avoid race conditions
if self.connection.idle_shutdown_secs > 0
    && self.connection.idle_shutdown_secs <= self.connection.ping_interval_secs * 2
{
    return Err(ServerError::ConfigInvalid {
        message: format!(
            "idle_shutdown_secs ({}) must be > 2x ping_interval_secs ({}) to avoid false shutdowns",
            self.connection.idle_shutdown_secs,
            self.connection.ping_interval_secs
        ),
        location: ErrorLocation::from(Location::caller()),
    });
}
```

**Verification**: `cargo check -p project-manager`

---

## Phase 5: Lifecycle Refactor (Atomic)

**IMPORTANT**: All changes in this phase must be applied together. Do NOT try to verify after individual sub-steps - the code will not compile until all sub-steps are complete.

### Step 10: Complete lifecycle.rs Refactor

**File**: `desktop/src-tauri/src/server/lifecycle.rs`

Apply ALL of the following changes together:

#### 10a: Update Imports and Struct Fields

Remove imports and change struct fields:
```rust
// REMOVE these imports:
// use tauri_plugin_shell::ShellExt;
// use tauri_plugin_shell::process::CommandChild;

// In ServerManager struct, change:
pub struct ServerManager {
    config: ServerConfig,
    server_dir: PathBuf,    // .server/ - for pm-server
    tauri_dir: PathBuf,     // .tauri/ - for Tauri config/logs (ADD THIS)
    server_pid: Arc<Mutex<Option<u32>>>,  // Changed FROM: process: Arc<Mutex<Option<CommandChild>>>
    // ... other fields ...
    // REMOVE: ready_signal_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<()>>>>,
}
```

#### 10b: Update Constructor

```rust
impl ServerManager {
    pub fn new(server_dir: PathBuf, tauri_dir: PathBuf, config: ServerConfig) -> Self {
        Self {
            config,
            server_dir,
            tauri_dir,
            server_pid: Arc::new(Mutex::new(None)),  // Changed from process
            // ... other fields ...
            // REMOVE: ready_signal_tx: Arc::new(Mutex::new(None)),
        }
    }
}
```

#### 10c: Add Binary Discovery Function

```rust
/// Find the pm-server binary in development or bundled locations.
fn find_server_binary(&self) -> ServerResult<PathBuf> {
    // 1. Environment variable override (development/testing)
    if let Ok(path) = std::env::var("PM_SERVER_BIN") {
        let path = PathBuf::from(path);
        if path.exists() {
            info!("Using pm-server from PM_SERVER_BIN: {}", path.display());
            return Ok(path);
        }
        warn!("PM_SERVER_BIN set but path doesn't exist: {}", path.display());
    }

    // 2. Bundled location (production) - next to Tauri executable
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            let bundled = exe_dir.join("pm-server");
            if bundled.exists() {
                info!("Using bundled pm-server: {}", bundled.display());
                return Ok(bundled);
            }
        }
    }

    // 3. Development: walk up to find workspace root
    if let Ok(exe) = std::env::current_exe() {
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

    // 4. System PATH fallback
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
        location: error_location::ErrorLocation::from(Location::caller()),
    })
}
```

#### 10d: Replace spawn_process Method

```rust
/// Spawn pm-server as a standalone detached process.
async fn spawn_process(
    &self,
    _app: &tauri::AppHandle,
    port: u16,
) -> ServerResult<tokio::sync::oneshot::Receiver<()>> {
    info!(
        "Spawning standalone pm-server with PM_CONFIG_DIR={}",
        self.server_dir.display()
    );

    // Find pm-server binary
    let server_binary = self.find_server_binary()?;
    info!("Using pm-server at: {}", server_binary.display());

    // Prepare log file path (in .server/logs/)
    let log_file = self.server_dir
        .join(&self.config.logging.directory)
        .join("pm-server.log");

    // Ensure logs directory exists
    if let Some(log_dir) = log_file.parent() {
        std::fs::create_dir_all(log_dir).map_err(|e| ServerError::ProcessSpawn {
            source: std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to create logs directory: {}", e),
            ).into(),
            location: error_location::ErrorLocation::from(Location::caller()),
        })?;
    }

    // Spawn as detached process
    let mut cmd = std::process::Command::new(&server_binary);
    cmd.env("PM_CONFIG_DIR", self.server_dir.to_str().unwrap())
        .env("PM_SERVER_PORT", port.to_string())
        .env("PM_SERVER_HOST", &self.config.server.host)
        .env("PM_LOG_LEVEL", &self.config.logging.level)
        .env("PM_LOG_FILE", log_file.to_str().unwrap())
        .env("PM_IDLE_SHUTDOWN_SECS", self.config.connection.idle_shutdown_secs.to_string())
        .env("PM_AUTH_ENABLED", "false"); // Desktop mode = no auth

    // Detach on Unix
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        unsafe {
            cmd.pre_exec(|| {
                libc::setsid();
                Ok(())
            });
        }
    }

    // Close stdio - server logs to file via PM_LOG_FILE
    cmd.stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());

    let child = cmd.spawn().map_err(|e| ServerError::ProcessSpawn {
        source: std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to spawn pm-server: {}", e),
        ).into(),
        location: error_location::ErrorLocation::from(Location::caller()),
    })?;

    let pid = child.id();
    info!("Spawned standalone pm-server with PID: {}", pid);

    // Store PID for tracking
    *self.server_pid.lock().await = Some(pid);

    // Don't store child handle - it's detached
    drop(child);

    // Create channel and poll /ready endpoint
    let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
    let ready_port = port;

    tokio::spawn(async move {
        let timeout = Duration::from_secs(30);
        let start = Instant::now();

        while start.elapsed() < timeout {
            tokio::time::sleep(Duration::from_millis(200)).await;

            if let Ok(client) = reqwest::Client::builder()
                .timeout(Duration::from_millis(1000))
                .build()
            {
                let url = format!("http://127.0.0.1:{}/ready", ready_port);
                if let Ok(response) = client.get(&url).send().await {
                    if response.status().is_success() {
                        info!("Server readiness check passed");
                        let _ = ready_tx.send(());
                        return;
                    }
                }
            }
        }
        warn!("Server readiness check timed out after 30s");
    });

    Ok(ready_rx)
}
```

#### 10e: Update server_pid Method

```rust
/// Get server process PID (if running).
pub async fn server_pid(&self) -> Option<u32> {
    *self.server_pid.lock().await
}
```

#### 10f: Replace stop Method

```rust
/// Stop the server gracefully.
pub async fn stop(&self) -> ServerResult<()> {
    self.shutdown_requested.store(true, Ordering::SeqCst);
    self.set_state(ServerState::ShuttingDown);

    // Update health status
    if let Some(ref hc) = *self.health_checker.lock().await {
        hc.set_status(HealthStatus::ShuttingDown).await;
    }

    // Checkpoint database before shutdown
    if self.config.database.checkpoint_on_shutdown {
        if let Err(e) = self.checkpoint_database().await {
            warn!("Failed to checkpoint database: {e}");
        }
    }

    // Kill server process if we have a PID
    let pid_guard = self.server_pid.lock().await;
    if let Some(pid) = *pid_guard {
        drop(pid_guard); // Release lock before async operations

        let timeout = Duration::from_secs(self.config.resilience.shutdown_timeout_secs);
        let port = self.actual_port.lock().await.unwrap_or(self.config.server.port);

        // First, try HTTP shutdown endpoint
        let shutdown_success = self.request_graceful_shutdown(port).await;

        if !shutdown_success {
            // Fallback to OS-level signals
            #[cfg(unix)]
            {
                use nix::sys::signal::{Signal, kill};
                use nix::unistd::Pid;

                info!("Sending SIGTERM to pid {pid}");
                kill(Pid::from_raw(pid as i32), Signal::SIGTERM).ok();
            }

            #[cfg(windows)]
            {
                use windows_sys::Win32::System::Console::{
                    CTRL_BREAK_EVENT, GenerateConsoleCtrlEvent,
                };

                info!("Sending CTRL_BREAK to pid {pid}");
                unsafe {
                    GenerateConsoleCtrlEvent(CTRL_BREAK_EVENT, pid);
                }
            }
        }

        // Wait for process to exit with timeout
        let start = Instant::now();
        let poll_interval = Duration::from_millis(100);

        while start.elapsed() < timeout {
            if let Ok(client) = reqwest::Client::builder()
                .timeout(Duration::from_millis(500))
                .build()
            {
                let url = format!("http://127.0.0.1:{}/health", port);
                if client.get(&url).send().await.is_err() {
                    info!("Server stopped responding, shutdown complete");
                    break;
                }
            }
            tokio::time::sleep(poll_interval).await;
        }

        // Force kill if still running after timeout
        info!("Force killing server process (PID: {})", pid);

        #[cfg(unix)]
        {
            use nix::sys::signal::{Signal, kill};
            use nix::unistd::Pid;

            kill(Pid::from_raw(pid as i32), Signal::SIGKILL).ok();
        }

        #[cfg(windows)]
        {
            std::process::Command::new("taskkill")
                .args(["/F", "/PID", &pid.to_string()])
                .output()
                .ok();
        }

        // Clear stored PID
        *self.server_pid.lock().await = None;
    }

    // Release lock file
    if let Some(mut lock) = self.lock_file.lock().await.take() {
        lock.release();
    }

    self.set_state(ServerState::Stopped);
    info!("Server stopped");

    Ok(())
}
```

**Verification** (after ALL sub-steps complete): `cargo check -p project-manager`

---

## Phase 6: lib.rs App Handlers

### Step 11: Update lib.rs Directory Setup

**File**: `desktop/src-tauri/src/lib.rs`

Update constants and directory setup in `setup` closure:
```rust
// Constants for directory names
const SERVER_DATA_DIR: &str = ".server";
const TAURI_DATA_DIR: &str = ".tauri";

// In setup closure:
let app_data_dir = app.path().app_data_dir()?;

// Server data directory (.server/)
let server_dir = app_data_dir.join(SERVER_DATA_DIR);
std::fs::create_dir_all(&server_dir)?;

// Tauri data directory (.tauri/)
let tauri_dir = app_data_dir.join(TAURI_DATA_DIR);
std::fs::create_dir_all(&tauri_dir)?;

// Initialize Tauri logging to .tauri/logs/
setup_logging(&tauri_dir)?;

// Load Tauri's config from .tauri/
let config = ServerConfig::load_or_create(&tauri_dir)
    .map_err(|e| format!("Config error: {}", e))?;

// ServerManager uses server_dir for pm-server, tauri_dir for reference
let manager = Arc::new(ServerManager::new(server_dir.clone(), tauri_dir.clone(), config));
```

**Verification**: `cargo check -p project-manager`

---

### Step 12: Add Signal Handlers to lib.rs

**File**: `desktop/src-tauri/src/lib.rs`

Add at the start of `setup` closure, after directory setup:
```rust
// Setup signal handlers for graceful shutdown on Unix
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
                    match manager.stop().await {
                        Ok(()) => info!("Server stopped due to signal {}", sig),
                        Err(e) => error!("Failed to stop server on signal: {}", e),
                    }
                });
            }

            std::process::exit(0);
        }
    });
}
```

**Verification**: `cargo check -p project-manager`

---

### Step 13: Add ExitRequested Handler

**File**: `desktop/src-tauri/src/lib.rs`

Replace `.run(tauri::generate_context!())` with `.build().run()` pattern:
```rust
// REPLACE this:
// .run(tauri::generate_context!())
// .expect("error while running tauri application");

// WITH this:
.build(tauri::generate_context!())
.expect("error while building tauri application")
.run(|app_handle, event| {
    use tauri::RunEvent;

    match event {
        RunEvent::ExitRequested { api, code, .. } => {
            info!("Exit requested (code: {:?})", code);
            api.prevent_exit();

            let app_handle_clone = app_handle.clone();
            tauri::async_runtime::block_on(async move {
                if let Some(manager) = app_handle_clone.try_state::<Arc<ServerManager>>() {
                    info!("Stopping server before exit...");
                    match manager.stop().await {
                        Ok(()) => info!("Server stopped successfully"),
                        Err(e) => error!("Failed to stop server: {}", e),
                    }
                }
            });

            std::process::exit(code.unwrap_or(0));
        }
        _ => {}
    }
});
```

**Verification**: `cargo check -p project-manager`

---

## Phase 7: Commands & Tray

### Step 14: Update tray.rs Quit Handler

**File**: `desktop/src-tauri/src/tray.rs`

Change quit handler from `spawn` to `block_on`:
```rust
"quit" => {
    tracing::info!("Tray quit clicked");
    let app_handle = app.clone();
    // Use block_on to ensure server stops BEFORE app exits
    tauri::async_runtime::block_on(async move {
        if let Some(manager) = app_handle.try_state::<Arc<ServerManager>>() {
            match manager.stop().await {
                Ok(()) => tracing::info!("Server stopped successfully"),
                Err(e) => tracing::error!("Failed to stop server: {}", e),
            }
        }
        app_handle.exit(0);
    });
}
```

**Verification**: `cargo check -p project-manager`

---

### Step 15: Update wasm_ready Command

**File**: `desktop/src-tauri/src/commands.rs`

Add `Emitter` import and update function:
```rust
use tauri::{Emitter, Manager, State};  // Add Emitter

#[tauri::command]
pub async fn wasm_ready(
    app: tauri::AppHandle,  // NEW parameter
    manager: State<'_, Arc<ServerManager>>,
) -> Result<ServerStatus, String> {
    tracing::info!("WASM ready notification received");

    let state = manager.state().await;
    let port = manager.port().await;
    let ws_url = manager.websocket_url().await;
    let health = manager.health().await;
    let pid = manager.server_pid().await;

    let status = build_server_status(&state, port, ws_url, health.as_ref(), pid);

    // Re-emit server-ready if already running (handles race condition)
    if matches!(state, ServerState::Running { .. }) && port.is_some() {
        tracing::info!("Server already running, re-emitting server-ready event");
        app.emit("server-ready", &status).ok();
    }

    Ok(status)
}
```

**Verification**: `cargo check -p project-manager`

---

### Step 16: Add quit_app Command

**File**: `desktop/src-tauri/src/commands.rs`

Add new command:
```rust
/// Quit the application with proper server shutdown.
#[tauri::command]
pub async fn quit_app(
    app: tauri::AppHandle,
    manager: State<'_, Arc<ServerManager>>,
) -> Result<(), String> {
    tracing::info!("quit_app command called");

    match manager.stop().await {
        Ok(()) => tracing::info!("Server stopped via quit_app"),
        Err(e) => tracing::error!("Failed to stop server via quit_app: {}", e),
    }

    app.exit(0);
    Ok(())
}
```

Register in `invoke_handler`:
```rust
.invoke_handler(tauri::generate_handler![
    // ... existing commands ...
    commands::quit_app,  // NEW
])
```

**Verification**: `cargo check -p project-manager`

---

## Phase 8: Bundling

### Step 17: Update tauri.conf.json Bundling

**File**: `desktop/src-tauri/tauri.conf.json`

Remove `externalBin` section if present, update `resources`:
```json
{
  "bundle": {
    "resources": {
      "../../.pm/config.toml": ".pm/config.toml",
      "../../target/release/pm-server": "pm-server"
    }
  }
}
```

**Note**: The binary discovery in lifecycle.rs will find pm-server in the bundle directory.

**Verification**: `cargo check -p project-manager`

---

## Session 44 Completion Checklist

After completing all steps:

- [x] All 17 implementation steps complete
- [ ] `just clean && just build-dev` (clean build required)
- [ ] `cargo check --workspace` passes
- [ ] `cargo test --workspace` passes
- [ ] `cargo clippy --workspace -- -D warnings` passes
- [ ] Fix any stale `.tauri/config.toml` validation errors
- [ ] Verify WASM connects to server on first launch (no retry needed)

### Files Modified (14 files, 17 steps organized in 8 phases)

| File | Change |
|------|--------|
| `Cargo.toml` | Add signal-hook workspace dependency |
| `desktop/src-tauri/Cargo.toml` | Add signal-hook Unix dependency |
| `backend/crates/pm-config/src/logging_config.rs` | Add `file` field |
| `backend/crates/pm-config/src/server_config.rs` | Add `idle_shutdown_secs` field |
| `backend/crates/pm-config/src/config.rs` | Add PM_LOG_FILE, PM_IDLE_SHUTDOWN_SECS overrides |
| `backend/config.example.toml` | Document new options |
| `backend/pm-server/src/logger.rs` | Accept optional file path parameter |
| `backend/pm-server/src/main.rs` | Config-driven log path, configurable idle shutdown |
| `desktop/src-tauri/src/server/config.rs` | Add ConnectionSettings struct |
| `desktop/src-tauri/src/server/lifecycle.rs` | `.server/` + `.tauri/` dirs, PID tracking, binary discovery |
| `desktop/src-tauri/src/lib.rs` | Directory restructure, signal handlers, ExitRequested |
| `desktop/src-tauri/src/tray.rs` | Blocking quit handler |
| `desktop/src-tauri/src/commands.rs` | wasm_ready re-emit, quit_app command |
| `desktop/src-tauri/tauri.conf.json` | pm-server bundling |

---

## Final Verification

```bash
# IMPORTANT: Clean build required after SDK/dependency changes
just clean
just build-dev

# Or manually:
# rm -rf frontend/**/bin frontend/**/obj
# cargo clean

# Build and test
cargo check --workspace
cargo test --workspace
cargo clippy --workspace -- -D warnings

# Test pm-server standalone
cargo run -p pm-server
# Should log to stdout

PM_LOG_FILE=test.log cargo run -p pm-server
# Should create .pm/log/test.log

# Test Tauri dev
cargo tauri dev
# Should find and spawn pm-server

# Verify directory structure
ls -la ~/Library/Application\ Support/com.projectmanager.app/
# Should show:
#   .server/   ← pm-server data
#   .tauri/    ← Tauri config/logs
#   user.json

ls -la ~/Library/Application\ Support/com.projectmanager.app/.server/
# Should show: config.toml, data.db, logs/, server.lock

ls -la ~/Library/Application\ Support/com.projectmanager.app/.tauri/
# Should show: config.toml, logs/

# Test shutdown (in separate terminal)
pgrep pm-server  # Should show PID
# Quit from tray or Cmd+Q
pgrep pm-server  # Should show nothing (no orphan)
```

---

## Rollback

The stash `stash@{0}` remains available as backup:
```bash
git stash show -p stash@{0}  # View changes
git stash pop                 # Restore if needed
```
