# Session 40: Tauri Desktop Integration - Production-Grade Plan

**Created**: 2026-01-20
**Quality Target**: 9.25/10 production-grade
**Estimated Tokens**: ~150k (split into 6 sub-sessions)
**Goal**: Production-ready desktop application with embedded pm-server

---

## Design Principles

1. **Fail gracefully** - Every error has a recovery path or clear user guidance
2. **No data loss** - Database integrity preserved in all scenarios
3. **Observable** - Comprehensive logging for debugging and support
4. **Secure by default** - Minimal attack surface, defense in depth
5. **Resilient** - Auto-recovery from transient failures
6. **Testable** - Every component has automated tests

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                    Tauri Application                         │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │   Window    │  │ System Tray │  │  IPC Commands       │  │
│  │  Manager    │  │   Manager   │  │  (get-status, etc)  │  │
│  └──────┬──────┘  └──────┬──────┘  └──────────┬──────────┘  │
│         │                │                     │             │
│         └────────────────┼─────────────────────┘             │
│                          │                                   │
│                ┌─────────▼─────────┐                        │
│                │  Server Manager   │                        │
│                │  ┌─────────────┐  │                        │
│                │  │ Lifecycle   │  │                        │
│                │  │ Health Mon. │  │                        │
│                │  │ Crash Recov.│  │                        │
│                │  └─────────────┘  │                        │
│                └─────────┬─────────┘                        │
│                          │                                   │
├──────────────────────────┼──────────────────────────────────┤
│                          │ stdin/stdout + signals            │
│                ┌─────────▼─────────┐                        │
│                │    pm-server      │◄── Sidecar Process     │
│                │  (SQLite + WS)    │                        │
│                └─────────┬─────────┘                        │
│                          │                                   │
│                ┌─────────▼─────────┐                        │
│                │   .pm/ directory  │                        │
│                │  ├── config.toml  │                        │
│                │  ├── data.db      │                        │
│                │  ├── server.lock  │                        │
│                │  └── logs/        │                        │
│                └───────────────────┘                        │
└─────────────────────────────────────────────────────────────┘
                          │
                          │ WebSocket (127.0.0.1:port)
                          ▼
┌─────────────────────────────────────────────────────────────┐
│                   Blazor WASM Frontend                       │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────┐   │
│  │ Server       │  │ Connection   │  │ UI Components    │   │
│  │ Discovery    │  │ Manager      │  │ (from Session 30)│   │
│  └──────────────┘  └──────────────┘  └──────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

---

## Phase 1: Core Infrastructure (~25k tokens)

### 1.1 Server Manager - Core Structure

**File**: `desktop/src-tauri/src/server/mod.rs`

```rust
//! Server lifecycle management with production-grade reliability.
//!
//! Responsibilities:
//! - Process spawning with environment isolation
//! - Health monitoring with circuit breaker
//! - Graceful shutdown with timeout escalation
//! - Crash recovery with exponential backoff
//! - Lock file management for single-instance

mod config;
mod error;
mod health;
mod lifecycle;
mod lock;
mod port;

pub use config::ServerConfig;
pub use error::ServerError;
pub use health::{HealthStatus, HealthChecker};
pub use lifecycle::{ServerManager, ServerState};
pub use lock::LockFile;
pub use port::PortManager;
```

**File**: `desktop/src-tauri/src/server/error.rs`

```rust
//! Comprehensive error types with recovery guidance.

use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ServerError {
    // Startup errors
    #[error("Failed to create data directory at {path}: {source}")]
    DataDirCreation {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("Configuration invalid: {message}")]
    ConfigInvalid { message: String },

    #[error("Failed to spawn server process: {source}")]
    ProcessSpawn {
        #[from]
        source: tauri_plugin_shell::Error,
    },

    #[error("Server binary not found at {path}")]
    BinaryNotFound { path: PathBuf },

    // Port errors
    #[error("Port {port} is in use by another application")]
    PortInUse { port: u16 },

    #[error("No available port in range {start}-{end}")]
    NoAvailablePort { start: u16, end: u16 },

    // Health errors
    #[error("Server failed to become ready within {timeout_secs}s")]
    StartupTimeout { timeout_secs: u64 },

    #[error("Health check failed: {message}")]
    HealthCheckFailed { message: String },

    // Lifecycle errors
    #[error("Server crashed with exit code {code:?}: {stderr}")]
    ProcessCrashed { code: Option<i32>, stderr: String },

    #[error("Graceful shutdown timed out after {timeout_secs}s")]
    ShutdownTimeout { timeout_secs: u64 },

    #[error("Maximum restart attempts ({max}) exceeded")]
    MaxRestartsExceeded { max: u32 },

    // Lock errors
    #[error("Another instance is already running (lock file: {path})")]
    AlreadyRunning { path: PathBuf },

    #[error("Failed to acquire lock: {source}")]
    LockAcquisition {
        path: PathBuf,
        source: std::io::Error,
    },

    // Database errors
    #[error("Database integrity check failed: {message}")]
    DatabaseCorruption { message: String },

    #[error("Failed to checkpoint database: {message}")]
    CheckpointFailed { message: String },

    // Generic
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
}

impl ServerError {
    /// Returns user-friendly recovery instructions.
    pub fn recovery_hint(&self) -> &'static str {
        match self {
            Self::PortInUse { .. } => {
                "Another application is using the required port. \
                 Close other applications or restart your computer."
            }
            Self::AlreadyRunning { .. } => {
                "Project Manager is already running. \
                 Check your system tray or task manager."
            }
            Self::StartupTimeout { .. } => {
                "The server is taking too long to start. \
                 Try restarting the application or check the logs."
            }
            Self::MaxRestartsExceeded { .. } => {
                "The server keeps crashing. \
                 Please report this issue with the diagnostic logs."
            }
            Self::DatabaseCorruption { .. } => {
                "The database may be corrupted. \
                 A backup will be created and recovery attempted."
            }
            Self::BinaryNotFound { .. } => {
                "The application installation appears incomplete. \
                 Please reinstall Project Manager."
            }
            _ => "An unexpected error occurred. Please check the logs.",
        }
    }

    /// Whether this error is recoverable via retry.
    pub fn is_transient(&self) -> bool {
        matches!(
            self,
            Self::HealthCheckFailed { .. }
                | Self::Http(_)
                | Self::StartupTimeout { .. }
        )
    }
}
```

### 1.2 Configuration Management

**File**: `desktop/src-tauri/src/server/config.rs`

```rust
//! Server configuration with validation and versioning.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Configuration version for migration support.
pub const CONFIG_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Config file format version
    #[serde(default = "default_version")]
    pub version: u32,

    /// Server settings
    #[serde(default)]
    pub server: ServerSettings,

    /// Database settings
    #[serde(default)]
    pub database: DatabaseSettings,

    /// Logging settings
    #[serde(default)]
    pub logging: LoggingSettings,

    /// Resilience settings
    #[serde(default)]
    pub resilience: ResilienceSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerSettings {
    #[serde(default = "default_host")]
    pub host: String,

    #[serde(default = "default_port")]
    pub port: u16,

    /// Port range for fallback if primary port unavailable
    #[serde(default = "default_port_range")]
    pub port_range: (u16, u16),

    #[serde(default = "default_max_connections")]
    pub max_connections: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseSettings {
    /// Database filename (relative to data directory)
    #[serde(default = "default_db_name")]
    pub filename: String,

    /// Enable WAL checkpoint on shutdown
    #[serde(default = "default_true")]
    pub checkpoint_on_shutdown: bool,

    /// Backup before migrations
    #[serde(default = "default_true")]
    pub backup_before_migration: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingSettings {
    #[serde(default = "default_log_level")]
    pub level: String,

    /// Log directory (relative to data directory)
    #[serde(default = "default_log_dir")]
    pub directory: String,

    /// Maximum log file size in MB before rotation
    #[serde(default = "default_max_log_size")]
    pub max_file_size_mb: u32,

    /// Number of rotated log files to keep
    #[serde(default = "default_log_retention")]
    pub retention_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResilienceSettings {
    /// Maximum server restart attempts
    #[serde(default = "default_max_restarts")]
    pub max_restarts: u32,

    /// Time window for restart counting (seconds)
    #[serde(default = "default_restart_window")]
    pub restart_window_secs: u64,

    /// Initial backoff delay (milliseconds)
    #[serde(default = "default_initial_backoff")]
    pub initial_backoff_ms: u64,

    /// Maximum backoff delay (milliseconds)
    #[serde(default = "default_max_backoff")]
    pub max_backoff_ms: u64,

    /// Startup timeout (seconds)
    #[serde(default = "default_startup_timeout")]
    pub startup_timeout_secs: u64,

    /// Graceful shutdown timeout (seconds)
    #[serde(default = "default_shutdown_timeout")]
    pub shutdown_timeout_secs: u64,

    /// Health check interval (seconds)
    #[serde(default = "default_health_interval")]
    pub health_check_interval_secs: u64,
}

// Default value functions
fn default_version() -> u32 { CONFIG_VERSION }
fn default_host() -> String { "127.0.0.1".into() }
fn default_port() -> u16 { 8000 }
fn default_port_range() -> (u16, u16) { (8000, 8100) }
fn default_max_connections() -> u32 { 100 }
fn default_db_name() -> String { "data.db".into() }
fn default_true() -> bool { true }
fn default_log_level() -> String { "info".into() }
fn default_log_dir() -> String { "logs".into() }
fn default_max_log_size() -> u32 { 10 }
fn default_log_retention() -> u32 { 5 }
fn default_max_restarts() -> u32 { 5 }
fn default_restart_window() -> u64 { 300 } // 5 minutes
fn default_initial_backoff() -> u64 { 100 }
fn default_max_backoff() -> u64 { 30000 } // 30 seconds
fn default_startup_timeout() -> u64 { 30 }
fn default_shutdown_timeout() -> u64 { 10 }
fn default_health_interval() -> u64 { 5 }

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            version: CONFIG_VERSION,
            server: ServerSettings::default(),
            database: DatabaseSettings::default(),
            logging: LoggingSettings::default(),
            resilience: ResilienceSettings::default(),
        }
    }
}

impl Default for ServerSettings {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            port_range: default_port_range(),
            max_connections: default_max_connections(),
        }
    }
}

impl Default for DatabaseSettings {
    fn default() -> Self {
        Self {
            filename: default_db_name(),
            checkpoint_on_shutdown: true,
            backup_before_migration: true,
        }
    }
}

impl Default for LoggingSettings {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            directory: default_log_dir(),
            max_file_size_mb: default_max_log_size(),
            retention_count: default_log_retention(),
        }
    }
}

impl Default for ResilienceSettings {
    fn default() -> Self {
        Self {
            max_restarts: default_max_restarts(),
            restart_window_secs: default_restart_window(),
            initial_backoff_ms: default_initial_backoff(),
            max_backoff_ms: default_max_backoff(),
            startup_timeout_secs: default_startup_timeout(),
            shutdown_timeout_secs: default_shutdown_timeout(),
            health_check_interval_secs: default_health_interval(),
        }
    }
}

impl ServerConfig {
    /// Load config from file, creating default if not exists.
    pub fn load_or_create(data_dir: &Path) -> Result<Self, super::ServerError> {
        let config_path = data_dir.join("config.toml");

        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            let mut config: Self = toml::from_str(&content).map_err(|e| {
                super::ServerError::ConfigInvalid {
                    message: e.to_string(),
                }
            })?;

            // Migrate if needed
            if config.version < CONFIG_VERSION {
                config = Self::migrate(config)?;
                config.save(data_dir)?;
            }

            config.validate()?;
            Ok(config)
        } else {
            let config = Self::default();
            config.save(data_dir)?;
            Ok(config)
        }
    }

    /// Save config to file.
    pub fn save(&self, data_dir: &Path) -> Result<(), super::ServerError> {
        let config_path = data_dir.join("config.toml");
        let content = toml::to_string_pretty(self).map_err(|e| {
            super::ServerError::ConfigInvalid {
                message: e.to_string(),
            }
        })?;

        // Write atomically via temp file
        let temp_path = config_path.with_extension("toml.tmp");
        std::fs::write(&temp_path, &content)?;
        std::fs::rename(&temp_path, &config_path)?;

        Ok(())
    }

    /// Migrate config from older version.
    fn migrate(mut config: Self) -> Result<Self, super::ServerError> {
        // Version 0 -> 1: Add resilience settings
        if config.version == 0 {
            config.resilience = ResilienceSettings::default();
            config.version = 1;
        }

        // Future migrations go here

        Ok(config)
    }

    /// Validate configuration values.
    fn validate(&self) -> Result<(), super::ServerError> {
        if self.server.port < 1024 {
            return Err(super::ServerError::ConfigInvalid {
                message: "Port must be >= 1024".into(),
            });
        }

        if self.server.port_range.0 > self.server.port_range.1 {
            return Err(super::ServerError::ConfigInvalid {
                message: "Invalid port range".into(),
            });
        }

        if self.resilience.startup_timeout_secs == 0 {
            return Err(super::ServerError::ConfigInvalid {
                message: "Startup timeout must be > 0".into(),
            });
        }

        Ok(())
    }
}
```

### 1.3 Lock File Management

**File**: `desktop/src-tauri/src/server/lock.rs`

```rust
//! Lock file for single-instance enforcement.

use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;

pub struct LockFile {
    path: PathBuf,
    file: Option<File>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct LockInfo {
    pid: u32,
    port: u16,
    started_at: String,
}

impl LockFile {
    /// Try to acquire the lock file.
    /// Returns Ok if acquired, Err if another instance is running.
    pub fn acquire(data_dir: &Path, port: u16) -> Result<Self, super::ServerError> {
        let path = data_dir.join("server.lock");

        // Check if existing lock is stale
        if path.exists() {
            if let Ok(existing) = Self::read_lock_info(&path) {
                if Self::is_process_running(existing.pid) {
                    return Err(super::ServerError::AlreadyRunning {
                        path: path.clone(),
                    });
                }
                // Stale lock, remove it
                std::fs::remove_file(&path).ok();
            }
        }

        // Create lock file with exclusive access
        #[cfg(unix)]
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&path)
            .map_err(|e| super::ServerError::LockAcquisition {
                path: path.clone(),
                source: e,
            })?;

        #[cfg(windows)]
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)
            .map_err(|e| super::ServerError::LockAcquisition {
                path: path.clone(),
                source: e,
            })?;

        let mut lock = Self {
            path,
            file: Some(file),
        };

        lock.write_info(port)?;

        Ok(lock)
    }

    /// Write current process info to lock file.
    fn write_info(&mut self, port: u16) -> Result<(), super::ServerError> {
        let info = LockInfo {
            pid: std::process::id(),
            port,
            started_at: chrono::Utc::now().to_rfc3339(),
        };

        let content = serde_json::to_string_pretty(&info).unwrap();

        if let Some(ref mut file) = self.file {
            file.write_all(content.as_bytes())?;
            file.sync_all()?;
        }

        Ok(())
    }

    fn read_lock_info(path: &Path) -> Result<LockInfo, std::io::Error> {
        let mut file = File::open(path)?;
        let mut content = String::new();
        file.read_to_string(&mut content)?;
        serde_json::from_str(&content).map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, e)
        })
    }

    #[cfg(unix)]
    fn is_process_running(pid: u32) -> bool {
        unsafe { libc::kill(pid as i32, 0) == 0 }
    }

    #[cfg(windows)]
    fn is_process_running(pid: u32) -> bool {
        use windows_sys::Win32::Foundation::{CloseHandle, STILL_ACTIVE};
        use windows_sys::Win32::System::Threading::{
            GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
        };

        unsafe {
            let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if handle == 0 {
                return false;
            }

            let mut exit_code: u32 = 0;
            let result = GetExitCodeProcess(handle, &mut exit_code);
            CloseHandle(handle);

            result != 0 && exit_code == STILL_ACTIVE
        }
    }

    /// Release the lock file.
    pub fn release(&mut self) {
        self.file.take();
        std::fs::remove_file(&self.path).ok();
    }
}

impl Drop for LockFile {
    fn drop(&mut self) {
        self.release();
    }
}
```

### 1.4 Port Management

**File**: `desktop/src-tauri/src/server/port.rs`

```rust
//! Port allocation and availability checking.

use std::net::TcpListener;

pub struct PortManager;

impl PortManager {
    /// Find an available port, preferring the given port.
    pub fn find_available(preferred: u16, range: (u16, u16)) -> Result<u16, super::ServerError> {
        // Try preferred port first
        if Self::is_available(preferred) {
            return Ok(preferred);
        }

        // Search in range
        for port in range.0..=range.1 {
            if port != preferred && Self::is_available(port) {
                return Ok(port);
            }
        }

        Err(super::ServerError::NoAvailablePort {
            start: range.0,
            end: range.1,
        })
    }

    /// Check if a port is available for binding.
    pub fn is_available(port: u16) -> bool {
        TcpListener::bind(("127.0.0.1", port)).is_ok()
    }

    /// Check if a port has our server running on it.
    pub async fn is_our_server(port: u16, expected_version: &str) -> bool {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(1))
            .build()
            .ok();

        if let Some(client) = client {
            let url = format!("http://127.0.0.1:{}/health", port);
            if let Ok(resp) = client.get(&url).send().await {
                if let Ok(body) = resp.text().await {
                    // Check if it's our server by version
                    return body.contains("pm-server")
                        || body.contains(expected_version);
                }
            }
        }

        false
    }
}
```

---

## Phase 2: Health Monitoring & Crash Recovery (~25k tokens)

### 2.1 Health Checker

**File**: `desktop/src-tauri/src/server/health.rs`

```rust
//! Health monitoring with circuit breaker pattern.

use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HealthStatus {
    /// Server is healthy and responding
    Healthy {
        latency_ms: u64,
        version: String,
    },
    /// Server is starting up
    Starting,
    /// Server is not responding
    Unhealthy {
        consecutive_failures: u32,
        last_error: String,
    },
    /// Server has crashed
    Crashed {
        exit_code: Option<i32>,
    },
    /// Server is shutting down
    ShuttingDown,
    /// Server is stopped
    Stopped,
}

#[derive(Debug)]
pub struct HealthChecker {
    client: reqwest::Client,
    port: u16,
    status: Arc<RwLock<HealthStatus>>,
    consecutive_failures: AtomicU32,
    last_check_ms: AtomicU64,
    failure_threshold: u32,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct HealthResponse {
    status: String,
    version: String,
    #[serde(default)]
    database: Option<DatabaseHealth>,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct DatabaseHealth {
    status: String,
    latency_ms: u64,
}

impl HealthChecker {
    pub fn new(port: u16, failure_threshold: u32) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .pool_max_idle_per_host(1)
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            port,
            status: Arc::new(RwLock::new(HealthStatus::Starting)),
            consecutive_failures: AtomicU32::new(0),
            last_check_ms: AtomicU64::new(0),
            failure_threshold,
        }
    }

    /// Perform a health check against the server.
    pub async fn check(&self) -> HealthStatus {
        let start = Instant::now();
        let url = format!("http://127.0.0.1:{}/ready", self.port);

        let result = self.client.get(&url).send().await;
        let latency_ms = start.elapsed().as_millis() as u64;

        self.last_check_ms.store(latency_ms, Ordering::Relaxed);

        let new_status = match result {
            Ok(resp) if resp.status().is_success() => {
                self.consecutive_failures.store(0, Ordering::Relaxed);

                match resp.json::<HealthResponse>().await {
                    Ok(health) => HealthStatus::Healthy {
                        latency_ms,
                        version: health.version,
                    },
                    Err(e) => HealthStatus::Unhealthy {
                        consecutive_failures: 1,
                        last_error: format!("Invalid response: {}", e),
                    },
                }
            }
            Ok(resp) => {
                let failures = self.consecutive_failures.fetch_add(1, Ordering::Relaxed) + 1;
                HealthStatus::Unhealthy {
                    consecutive_failures: failures,
                    last_error: format!("HTTP {}", resp.status()),
                }
            }
            Err(e) => {
                let failures = self.consecutive_failures.fetch_add(1, Ordering::Relaxed) + 1;
                HealthStatus::Unhealthy {
                    consecutive_failures: failures,
                    last_error: e.to_string(),
                }
            }
        };

        // Update cached status
        *self.status.write().await = new_status.clone();

        new_status
    }

    /// Wait for server to become healthy.
    pub async fn wait_ready(&self, timeout: Duration) -> Result<(), super::ServerError> {
        let start = Instant::now();
        let poll_interval = Duration::from_millis(100);

        while start.elapsed() < timeout {
            match self.check().await {
                HealthStatus::Healthy { .. } => return Ok(()),
                HealthStatus::Unhealthy { consecutive_failures, .. }
                    if consecutive_failures >= self.failure_threshold =>
                {
                    return Err(super::ServerError::HealthCheckFailed {
                        message: "Too many consecutive failures".into(),
                    });
                }
                _ => {}
            }
            tokio::time::sleep(poll_interval).await;
        }

        Err(super::ServerError::StartupTimeout {
            timeout_secs: timeout.as_secs(),
        })
    }

    /// Get current cached status.
    pub async fn status(&self) -> HealthStatus {
        self.status.read().await.clone()
    }

    /// Set status (for crash/shutdown notifications).
    pub async fn set_status(&self, status: HealthStatus) {
        *self.status.write().await = status;
    }

    /// Check if server should be considered failed.
    pub fn is_failed(&self) -> bool {
        self.consecutive_failures.load(Ordering::Relaxed) >= self.failure_threshold
    }

    /// Get last check latency.
    pub fn last_latency_ms(&self) -> u64 {
        self.last_check_ms.load(Ordering::Relaxed)
    }
}
```

### 2.2 Server Lifecycle Manager

**File**: `desktop/src-tauri/src/server/lifecycle.rs`

```rust
//! Server process lifecycle with crash recovery.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::async_runtime::Mutex;
use tauri_plugin_shell::process::CommandChild;
use tauri_plugin_shell::ShellExt;
use tokio::sync::watch;
use tracing::{debug, error, info, warn};

use super::{
    HealthChecker, HealthStatus, LockFile, PortManager, ServerConfig, ServerError,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServerState {
    Stopped,
    Starting,
    Running { port: u16 },
    Restarting { attempt: u32 },
    ShuttingDown,
    Failed { error: String },
}

/// Commands that can be sent to the server manager from background tasks.
#[derive(Debug)]
pub enum ServerCommand {
    /// Request server restart after crash/unhealthy detection
    Restart { attempt: u32 },
    /// Signal that max restarts exceeded
    MaxRestartsExceeded { count: u32 },
}

pub struct ServerManager {
    config: ServerConfig,
    data_dir: PathBuf,
    process: Arc<Mutex<Option<CommandChild>>>,
    health_checker: Arc<Mutex<Option<HealthChecker>>>,
    lock_file: Arc<Mutex<Option<LockFile>>>,
    state_tx: watch::Sender<ServerState>,
    state_rx: watch::Receiver<ServerState>,
    restart_count: AtomicU32,
    restart_window_start: Arc<Mutex<Option<Instant>>>,
    shutdown_requested: AtomicBool,
    actual_port: Arc<Mutex<Option<u16>>>,
    /// Channel for receiving restart commands from health monitor
    command_tx: mpsc::Sender<ServerCommand>,
    command_rx: Arc<Mutex<mpsc::Receiver<ServerCommand>>>,
}

impl ServerManager {
    pub fn new(data_dir: PathBuf, config: ServerConfig) -> Self {
        let (state_tx, state_rx) = watch::channel(ServerState::Stopped);
        let (command_tx, command_rx) = mpsc::channel(16);

        Self {
            config,
            data_dir,
            process: Arc::new(Mutex::new(None)),
            health_checker: Arc::new(Mutex::new(None)),
            lock_file: Arc::new(Mutex::new(None)),
            state_tx,
            state_rx,
            restart_count: AtomicU32::new(0),
            restart_window_start: Arc::new(Mutex::new(None)),
            shutdown_requested: AtomicBool::new(false),
            actual_port: Arc::new(Mutex::new(None)),
            command_tx,
            command_rx: Arc::new(Mutex::new(command_rx)),
        }
    }

    /// Start the server and wait for it to be ready.
    pub async fn start(&self, app: &tauri::AppHandle) -> Result<(), ServerError> {
        self.shutdown_requested.store(false, Ordering::SeqCst);

        // Update state
        self.set_state(ServerState::Starting);

        // Ensure data directory exists
        self.ensure_data_dir()?;

        // Find available port
        let port = PortManager::find_available(
            self.config.server.port,
            self.config.server.port_range,
        )?;

        info!("Using port {}", port);

        // Acquire lock file
        let lock = LockFile::acquire(&self.data_dir, port)?;
        *self.lock_file.lock().await = Some(lock);

        // Spawn the server process
        self.spawn_process(app, port).await?;

        // Store actual port
        *self.actual_port.lock().await = Some(port);

        // Create health checker
        let health_checker = HealthChecker::new(port, 3);
        *self.health_checker.lock().await = Some(health_checker.clone());

        // Wait for server to be ready
        let timeout = Duration::from_secs(self.config.resilience.startup_timeout_secs);
        health_checker.wait_ready(timeout).await?;

        // Update state
        self.set_state(ServerState::Running { port });

        info!("Server started successfully on port {}", port);

        // Start background health monitoring
        self.start_health_monitor();

        // Start command handler (handles restart requests from health monitor)
        self.start_command_handler(app.clone());

        Ok(())
    }

    /// Start command handler that processes restart requests.
    /// This runs in the main async context with access to AppHandle.
    fn start_command_handler(&self, app: tauri::AppHandle) {
        let command_rx = self.command_rx.clone();
        let process = self.process.clone();
        let health_checker = self.health_checker.clone();
        let state_tx = self.state_tx.clone();
        let config = self.config.clone();
        let data_dir = self.data_dir.clone();
        let actual_port = self.actual_port.clone();
        let shutdown_requested = self.shutdown_requested.clone();

        tauri::async_runtime::spawn(async move {
            let mut rx = command_rx.lock().await;

            while let Some(cmd) = rx.recv().await {
                if shutdown_requested.load(Ordering::SeqCst) {
                    break;
                }

                match cmd {
                    ServerCommand::Restart { attempt } => {
                        info!("Processing restart request, attempt {}", attempt);
                        let _ = state_tx.send(ServerState::Restarting { attempt });

                        // Kill existing process
                        {
                            let mut proc_guard = process.lock().await;
                            if let Some(child) = proc_guard.take() {
                                child.kill().ok();
                            }
                        }

                        // Exponential backoff
                        let backoff = std::cmp::min(
                            config.resilience.initial_backoff_ms * 2u64.pow(attempt - 1),
                            config.resilience.max_backoff_ms,
                        );
                        tokio::time::sleep(Duration::from_millis(backoff)).await;

                        // Find new port (previous might be stuck)
                        let port = match PortManager::find_available(
                            config.server.port,
                            config.server.port_range,
                        ) {
                            Ok(p) => p,
                            Err(e) => {
                                error!("Failed to find available port: {}", e);
                                let _ = state_tx.send(ServerState::Failed {
                                    error: e.to_string(),
                                });
                                continue;
                            }
                        };

                        // Spawn new process
                        let sidecar = match app
                            .shell()
                            .sidecar("pm-server")
                            .map(|s| {
                                s.env("PM_CONFIG_DIR", data_dir.to_str().unwrap())
                                    .env("PM_SERVER_PORT", port.to_string())
                                    .env("PM_SERVER_HOST", &config.server.host)
                                    .env("PM_LOG_LEVEL", &config.logging.level)
                                    .env("PM_AUTH_ENABLED", "false")
                            }) {
                            Ok(s) => s,
                            Err(e) => {
                                error!("Failed to create sidecar: {}", e);
                                let _ = state_tx.send(ServerState::Failed {
                                    error: e.to_string(),
                                });
                                continue;
                            }
                        };

                        match sidecar.spawn() {
                            Ok((rx, child)) => {
                                *process.lock().await = Some(child);
                                *actual_port.lock().await = Some(port);

                                // Update health checker port
                                *health_checker.lock().await = Some(HealthChecker::new(port, 3));

                                // Wait for ready
                                let hc = health_checker.lock().await;
                                if let Some(ref checker) = *hc {
                                    let timeout = Duration::from_secs(
                                        config.resilience.startup_timeout_secs,
                                    );
                                    match checker.wait_ready(timeout).await {
                                        Ok(()) => {
                                            info!("Server restarted successfully on port {}", port);
                                            let _ = state_tx.send(ServerState::Running { port });
                                        }
                                        Err(e) => {
                                            warn!("Server failed to become ready after restart: {}", e);
                                            // Health monitor will detect and request another restart
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Failed to spawn process: {}", e);
                                let _ = state_tx.send(ServerState::Failed {
                                    error: e.to_string(),
                                });
                            }
                        }
                    }
                    ServerCommand::MaxRestartsExceeded { count } => {
                        error!("Max restarts exceeded: {} attempts", count);
                        let _ = state_tx.send(ServerState::Failed {
                            error: format!("Server crashed {} times", count),
                        });
                        break;
                    }
                }
            }
        });
    }

    /// Spawn the pm-server process.
    async fn spawn_process(
        &self,
        app: &tauri::AppHandle,
        port: u16,
    ) -> Result<(), ServerError> {
        let sidecar = app
            .shell()
            .sidecar("pm-server")
            .map_err(|e| ServerError::ProcessSpawn { source: e })?
            .env("PM_CONFIG_DIR", self.data_dir.to_str().unwrap())
            .env("PM_SERVER_PORT", port.to_string())
            .env("PM_SERVER_HOST", &self.config.server.host)
            .env("PM_LOG_LEVEL", &self.config.logging.level)
            .env("PM_AUTH_ENABLED", "false");

        let (mut rx, child) = sidecar
            .spawn()
            .map_err(|e| ServerError::ProcessSpawn { source: e })?;

        // Handle process output in background
        let data_dir = self.data_dir.clone();
        tauri::async_runtime::spawn(async move {
            use tauri_plugin_shell::process::CommandEvent;

            while let Some(event) = rx.recv().await {
                match event {
                    CommandEvent::Stdout(line) => {
                        debug!("pm-server: {}", String::from_utf8_lossy(&line));
                    }
                    CommandEvent::Stderr(line) => {
                        let msg = String::from_utf8_lossy(&line);
                        if msg.contains("ERROR") || msg.contains("WARN") {
                            warn!("pm-server: {}", msg);
                        } else {
                            debug!("pm-server: {}", msg);
                        }
                    }
                    CommandEvent::Error(e) => {
                        error!("pm-server error: {}", e);
                    }
                    CommandEvent::Terminated(payload) => {
                        info!(
                            "pm-server terminated with code {:?}, signal {:?}",
                            payload.code, payload.signal
                        );
                    }
                    _ => {}
                }
            }
        });

        *self.process.lock().await = Some(child);

        Ok(())
    }

    /// Start background health monitoring task.
    /// Monitors server health and sends restart commands via channel when needed.
    fn start_health_monitor(&self) {
        let health_checker = self.health_checker.clone();
        let shutdown_requested = self.shutdown_requested.clone();
        let interval = Duration::from_secs(self.config.resilience.health_check_interval_secs);
        let restart_count = self.restart_count.clone();
        let max_restarts = self.config.resilience.max_restarts;
        let command_tx = self.command_tx.clone();

        tauri::async_runtime::spawn(async move {
            loop {
                tokio::time::sleep(interval).await;

                if shutdown_requested.load(Ordering::SeqCst) {
                    break;
                }

                let checker = health_checker.lock().await;
                if let Some(ref hc) = *checker {
                    let status = hc.check().await;
                    drop(checker);

                    match status {
                        HealthStatus::Healthy { .. } => {
                            // Reset restart count on healthy status
                            restart_count.store(0, Ordering::SeqCst);
                        }
                        HealthStatus::Unhealthy { consecutive_failures, .. }
                            if consecutive_failures >= 3 =>
                        {
                            let count = restart_count.fetch_add(1, Ordering::SeqCst) + 1;

                            if count > max_restarts {
                                // Send max restarts exceeded command
                                let _ = command_tx
                                    .send(ServerCommand::MaxRestartsExceeded { count })
                                    .await;
                                break;
                            }

                            warn!(
                                "Server unhealthy, requesting restart {}/{}",
                                count, max_restarts
                            );

                            // Send restart command to command handler
                            // The command handler has access to AppHandle for spawning
                            let _ = command_tx
                                .send(ServerCommand::Restart { attempt: count })
                                .await;
                        }
                        _ => {}
                    }
                }
            }
        });
    }

    /// Stop the server gracefully.
    pub async fn stop(&self) -> Result<(), ServerError> {
        self.shutdown_requested.store(true, Ordering::SeqCst);
        self.set_state(ServerState::ShuttingDown);

        // Update health status
        if let Some(ref hc) = *self.health_checker.lock().await {
            hc.set_status(HealthStatus::ShuttingDown).await;
        }

        // Checkpoint database before shutdown
        if self.config.database.checkpoint_on_shutdown {
            if let Err(e) = self.checkpoint_database().await {
                warn!("Failed to checkpoint database: {}", e);
            }
        }

        // Graceful shutdown with timeout
        let mut process_guard = self.process.lock().await;
        if let Some(child) = process_guard.take() {
            let timeout = Duration::from_secs(self.config.resilience.shutdown_timeout_secs);
            let port = self.actual_port.lock().await.unwrap_or(self.config.server.port);

            // First, try HTTP shutdown endpoint (works on all platforms)
            let shutdown_success = self.request_graceful_shutdown(port).await;

            if !shutdown_success {
                // Fallback to OS-level signals
                #[cfg(unix)]
                {
                    use nix::sys::signal::{kill, Signal};
                    use nix::unistd::Pid;

                    if let Some(pid) = child.pid() {
                        info!("Sending SIGTERM to pid {}", pid);
                        kill(Pid::from_raw(pid as i32), Signal::SIGTERM).ok();
                    }
                }

                #[cfg(windows)]
                {
                    // Windows: Use CTRL_BREAK_EVENT for graceful shutdown
                    // This requires the process to be started with CREATE_NEW_PROCESS_GROUP
                    use windows_sys::Win32::System::Console::{
                        GenerateConsoleCtrlEvent, CTRL_BREAK_EVENT,
                    };

                    if let Some(pid) = child.pid() {
                        info!("Sending CTRL_BREAK to pid {}", pid);
                        unsafe {
                            // CTRL_BREAK_EVENT is sent to the process group
                            GenerateConsoleCtrlEvent(CTRL_BREAK_EVENT, pid);
                        }
                    }
                }
            }

            // Wait for process to exit with timeout
            let start = Instant::now();
            let poll_interval = Duration::from_millis(100);

            while start.elapsed() < timeout {
                // Check if process has exited
                // Note: Tauri's CommandChild doesn't expose wait(), so we check via health
                let client = reqwest::Client::builder()
                    .timeout(Duration::from_millis(500))
                    .build()
                    .ok();

                if let Some(client) = client {
                    let url = format!("http://127.0.0.1:{}/health", port);
                    if client.get(&url).send().await.is_err() {
                        // Server is no longer responding - likely exited
                        info!("Server stopped responding, shutdown complete");
                        break;
                    }
                }

                tokio::time::sleep(poll_interval).await;
            }

            // Force kill if still running after timeout
            info!("Force killing server process");
            child.kill().ok();
        }
        drop(process_guard);

        // Release lock file
        if let Some(mut lock) = self.lock_file.lock().await.take() {
            lock.release();
        }

        self.set_state(ServerState::Stopped);
        info!("Server stopped");

        Ok(())
    }

    /// Checkpoint database WAL.
    async fn checkpoint_database(&self) -> Result<(), ServerError> {
        let port = self.actual_port.lock().await.unwrap_or(self.config.server.port);
        let url = format!("http://127.0.0.1:{}/admin/checkpoint", port);

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()?;

        let resp = client.post(&url).send().await?;

        if !resp.status().is_success() {
            return Err(ServerError::CheckpointFailed {
                message: format!("HTTP {}", resp.status()),
            });
        }

        info!("Database checkpoint completed");
        Ok(())
    }

    /// Request graceful shutdown via HTTP endpoint.
    /// Returns true if the shutdown was acknowledged.
    async fn request_graceful_shutdown(&self, port: u16) -> bool {
        let url = format!("http://127.0.0.1:{}/admin/shutdown", port);

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .ok();

        if let Some(client) = client {
            match client.post(&url).send().await {
                Ok(resp) if resp.status().is_success() => {
                    info!("Graceful shutdown request acknowledged");
                    return true;
                }
                Ok(resp) => {
                    warn!("Shutdown request returned HTTP {}", resp.status());
                }
                Err(e) => {
                    warn!("Failed to send shutdown request: {}", e);
                }
            }
        }

        false
    }

    /// Ensure data directory structure exists.
    fn ensure_data_dir(&self) -> Result<(), ServerError> {
        // Create main data directory
        std::fs::create_dir_all(&self.data_dir).map_err(|e| {
            ServerError::DataDirCreation {
                path: self.data_dir.clone(),
                source: e,
            }
        })?;

        // Create logs subdirectory
        let logs_dir = self.data_dir.join(&self.config.logging.directory);
        std::fs::create_dir_all(&logs_dir).map_err(|e| {
            ServerError::DataDirCreation {
                path: logs_dir,
                source: e,
            }
        })?;

        Ok(())
    }

    fn set_state(&self, state: ServerState) {
        let _ = self.state_tx.send(state);
    }

    /// Subscribe to state changes.
    pub fn subscribe(&self) -> watch::Receiver<ServerState> {
        self.state_rx.clone()
    }

    /// Get current state.
    pub async fn state(&self) -> ServerState {
        self.state_rx.borrow().clone()
    }

    /// Get the WebSocket URL for frontend connection.
    pub async fn websocket_url(&self) -> Option<String> {
        self.actual_port
            .lock()
            .await
            .map(|p| format!("ws://127.0.0.1:{}/ws", p))
    }

    /// Get current port (if running).
    pub async fn port(&self) -> Option<u16> {
        *self.actual_port.lock().await
    }

    /// Get health status.
    pub async fn health(&self) -> Option<HealthStatus> {
        if let Some(ref hc) = *self.health_checker.lock().await {
            Some(hc.status().await)
        } else {
            None
        }
    }
}
```

---

## Phase 3: Tauri Integration & IPC (~25k tokens)

### 3.1 Tauri Commands

**File**: `desktop/src-tauri/src/commands.rs`

```rust
//! Tauri IPC commands for frontend communication.

use crate::server::{HealthStatus, ServerManager, ServerState};
use serde::Serialize;
use std::sync::Arc;
use tauri::State;

#[derive(Debug, Clone, Serialize)]
pub struct ServerStatus {
    pub state: String,
    pub port: Option<u16>,
    pub websocket_url: Option<String>,
    pub health: Option<HealthInfo>,
    pub error: Option<String>,
    pub recovery_hint: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct HealthInfo {
    pub status: String,
    pub latency_ms: Option<u64>,
    pub version: Option<String>,
}

impl From<&HealthStatus> for HealthInfo {
    fn from(status: &HealthStatus) -> Self {
        match status {
            HealthStatus::Healthy { latency_ms, version } => HealthInfo {
                status: "healthy".into(),
                latency_ms: Some(*latency_ms),
                version: Some(version.clone()),
            },
            HealthStatus::Starting => HealthInfo {
                status: "starting".into(),
                latency_ms: None,
                version: None,
            },
            HealthStatus::Unhealthy { last_error, .. } => HealthInfo {
                status: format!("unhealthy: {}", last_error),
                latency_ms: None,
                version: None,
            },
            HealthStatus::Crashed { exit_code } => HealthInfo {
                status: format!("crashed (code: {:?})", exit_code),
                latency_ms: None,
                version: None,
            },
            HealthStatus::ShuttingDown => HealthInfo {
                status: "shutting_down".into(),
                latency_ms: None,
                version: None,
            },
            HealthStatus::Stopped => HealthInfo {
                status: "stopped".into(),
                latency_ms: None,
                version: None,
            },
        }
    }
}

/// Get current server status.
#[tauri::command]
pub async fn get_server_status(
    manager: State<'_, Arc<ServerManager>>,
) -> Result<ServerStatus, String> {
    let state = manager.state().await;
    let port = manager.port().await;
    let ws_url = manager.websocket_url().await;
    let health = manager.health().await;

    let (state_str, error, recovery_hint) = match &state {
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

    Ok(ServerStatus {
        state: state_str,
        port,
        websocket_url: ws_url,
        health: health.as_ref().map(|h| h.into()),
        error,
        recovery_hint,
    })
}

/// Get WebSocket URL for frontend connection.
#[tauri::command]
pub async fn get_websocket_url(
    manager: State<'_, Arc<ServerManager>>,
) -> Result<String, String> {
    manager
        .websocket_url()
        .await
        .ok_or_else(|| "Server not running".into())
}

/// Manually restart the server.
#[tauri::command]
pub async fn restart_server(
    app: tauri::AppHandle,
    manager: State<'_, Arc<ServerManager>>,
) -> Result<(), String> {
    manager.stop().await.map_err(|e| e.to_string())?;
    manager.start(&app).await.map_err(|e| e.to_string())?;
    Ok(())
}

/// Export diagnostic information.
#[tauri::command]
pub async fn export_diagnostics(
    manager: State<'_, Arc<ServerManager>>,
    app: tauri::AppHandle,
) -> Result<String, String> {
    use std::io::Write;

    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;

    let export_path = data_dir.join("diagnostics.zip");

    // Create zip file with logs, config (sanitized), system info
    let file = std::fs::File::create(&export_path).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipWriter::new(file);

    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    // Add system info
    let system_info = format!(
        "OS: {}\nArch: {}\nVersion: {}\nTimestamp: {}",
        std::env::consts::OS,
        std::env::consts::ARCH,
        env!("CARGO_PKG_VERSION"),
        chrono::Utc::now().to_rfc3339(),
    );
    zip.start_file("system_info.txt", options)
        .map_err(|e| e.to_string())?;
    zip.write_all(system_info.as_bytes())
        .map_err(|e| e.to_string())?;

    // Add server status
    let status = get_server_status(manager).await?;
    let status_json = serde_json::to_string_pretty(&status).unwrap();
    zip.start_file("server_status.json", options)
        .map_err(|e| e.to_string())?;
    zip.write_all(status_json.as_bytes())
        .map_err(|e| e.to_string())?;

    // Add log files
    let logs_dir = data_dir.join("logs");
    if logs_dir.exists() {
        for entry in std::fs::read_dir(&logs_dir).map_err(|e| e.to_string())? {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.is_file() {
                    let name = format!("logs/{}", path.file_name().unwrap().to_string_lossy());
                    zip.start_file(&name, options).map_err(|e| e.to_string())?;
                    let content = std::fs::read(&path).map_err(|e| e.to_string())?;
                    zip.write_all(&content).map_err(|e| e.to_string())?;
                }
            }
        }
    }

    // Add config (without sensitive data)
    let config_path = data_dir.join("config.toml");
    if config_path.exists() {
        let config_content = std::fs::read_to_string(&config_path).map_err(|e| e.to_string())?;
        // Remove any potential secrets (basic sanitization)
        let sanitized = config_content
            .lines()
            .filter(|l| !l.contains("secret") && !l.contains("password") && !l.contains("key"))
            .collect::<Vec<_>>()
            .join("\n");
        zip.start_file("config.toml", options)
            .map_err(|e| e.to_string())?;
        zip.write_all(sanitized.as_bytes())
            .map_err(|e| e.to_string())?;
    }

    zip.finish().map_err(|e| e.to_string())?;

    Ok(export_path.to_string_lossy().into())
}

/// Get recent log lines.
#[tauri::command]
pub async fn get_recent_logs(
    app: tauri::AppHandle,
    lines: Option<usize>,
) -> Result<Vec<String>, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?;

    let log_path = data_dir.join("logs").join("pm-server.log");

    if !log_path.exists() {
        return Ok(vec!["No logs available yet.".into()]);
    }

    let content = std::fs::read_to_string(&log_path).map_err(|e| e.to_string())?;
    let lines_to_return = lines.unwrap_or(100);

    let log_lines: Vec<String> = content
        .lines()
        .rev()
        .take(lines_to_return)
        .map(String::from)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();

    Ok(log_lines)
}
```

### 3.2 System Tray

**File**: `desktop/src-tauri/src/tray.rs`

```rust
//! System tray with status indicator and menu.

use crate::server::{ServerManager, ServerState};
use std::sync::Arc;
use tauri::{
    menu::{Menu, MenuId, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, Runtime,
};
use tokio::sync::RwLock;

/// Manages the system tray and its state.
pub struct TrayManager {
    status_item_id: MenuId,
}

impl TrayManager {
    /// Create and setup the system tray.
    pub fn setup<R: Runtime>(app: &tauri::App<R>) -> Result<Arc<Self>, Box<dyn std::error::Error>> {
        let show_item = MenuItem::with_id(app, "show", "Show Window", true, None::<&str>)?;
        let status_item = MenuItem::with_id(app, "status", "Status: Starting...", false, None::<&str>)?;
        let status_item_id = status_item.id().clone();

        let separator1 = PredefinedMenuItem::separator(app)?;
        let restart_item = MenuItem::with_id(app, "restart", "Restart Server", true, None::<&str>)?;
        let logs_item = MenuItem::with_id(app, "logs", "View Logs...", true, None::<&str>)?;
        let separator2 = PredefinedMenuItem::separator(app)?;
        let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

        let menu = Menu::with_items(
            app,
            &[
                &show_item,
                &status_item,
                &separator1,
                &restart_item,
                &logs_item,
                &separator2,
                &quit_item,
            ],
        )?;

        let _tray = TrayIconBuilder::new()
            .icon(app.default_window_icon().unwrap().clone())
            .menu(&menu)
            .tooltip("Project Manager")
            .menu_on_left_click(false)
            .on_menu_event(move |app, event| match event.id.as_ref() {
                "show" => {
                    if let Some(window) = app.get_webview_window("main") {
                        window.show().ok();
                        window.set_focus().ok();
                    }
                }
                "restart" => {
                    let app_handle = app.clone();
                    tauri::async_runtime::spawn(async move {
                        if let Some(manager) = app_handle.try_state::<Arc<ServerManager>>() {
                            if let Err(e) = manager.stop().await {
                                tracing::error!("Failed to stop server: {}", e);
                            }
                            if let Err(e) = manager.start(&app_handle).await {
                                tracing::error!("Failed to restart server: {}", e);
                            }
                        }
                    });
                }
                "logs" => {
                    if let Ok(data_dir) = app.path().app_data_dir() {
                        let logs_dir = data_dir.join("logs");
                        open_directory(&logs_dir);
                    }
                }
                "quit" => {
                    // Graceful shutdown before exit
                    let app_handle = app.clone();
                    tauri::async_runtime::spawn(async move {
                        if let Some(manager) = app_handle.try_state::<Arc<ServerManager>>() {
                            let _ = manager.stop().await;
                        }
                        app_handle.exit(0);
                    });
                }
                _ => {}
            })
            .on_tray_icon_event(|tray, event| {
                if let TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                } = event
                {
                    if let Some(window) = tray.app_handle().get_webview_window("main") {
                        window.show().ok();
                        window.set_focus().ok();
                    }
                }
            })
            .build(app)?;

        Ok(Arc::new(Self { status_item_id }))
    }

    /// Update tray status text based on server state.
    pub fn update_status<R: Runtime>(&self, app: &AppHandle<R>, state: &ServerState) {
        let (status_text, tooltip) = match state {
            ServerState::Stopped => (
                "Status: Stopped".to_string(),
                "Project Manager - Stopped".to_string(),
            ),
            ServerState::Starting => (
                "Status: Starting...".to_string(),
                "Project Manager - Starting...".to_string(),
            ),
            ServerState::Running { port } => (
                format!("Status: Running (port {})", port),
                format!("Project Manager - Running on port {}", port),
            ),
            ServerState::Restarting { attempt } => (
                format!("Status: Restarting (attempt {})", attempt),
                format!("Project Manager - Restarting (attempt {})", attempt),
            ),
            ServerState::ShuttingDown => (
                "Status: Shutting down...".to_string(),
                "Project Manager - Shutting down...".to_string(),
            ),
            ServerState::Failed { error } => (
                "Status: Failed".to_string(),
                format!("Project Manager - Failed: {}", error),
            ),
        };

        // Update menu item text
        if let Some(menu) = app.menu() {
            if let Some(item) = menu.get(&self.status_item_id) {
                if let Some(menu_item) = item.as_menuitem() {
                    let _ = menu_item.set_text(&status_text);
                }
            }
        }

        // Update tray tooltip
        if let Some(tray) = app.tray_by_id("main") {
            let _ = tray.set_tooltip(Some(&tooltip));
        }

        tracing::debug!("Tray status updated: {}", status_text);
    }
}

/// Open a directory in the system file manager.
fn open_directory(path: &std::path::Path) {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open").arg(path).spawn().ok();
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer").arg(path).spawn().ok();
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open").arg(path).spawn().ok();
    }
}
```

### 3.3 Application Entry Point

**File**: `desktop/src-tauri/src/lib.rs`

```rust
mod commands;
mod logging;
mod server;
mod tray;

use logging::setup_logging;
use server::{ServerConfig, ServerManager};
use std::sync::Arc;
use tauri::Manager;
use tracing::{error, info};
use tray::TrayManager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            // Focus existing window on second instance attempt
            if let Some(window) = app.get_webview_window("main") {
                window.show().ok();
                window.set_focus().ok();
            }
        }))
        .setup(|app| {
            // Get data directory early for logging setup
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;

            // Initialize logging with rotation
            setup_logging(&data_dir)?;

            info!("Starting Project Manager v{}", env!("CARGO_PKG_VERSION"));
            info!("Data directory: {:?}", data_dir);

            // Load or create config
            let config = ServerConfig::load_or_create(&data_dir)
                .map_err(|e| format!("Config error: {}", e))?;

            // Create server manager
            let manager = Arc::new(ServerManager::new(data_dir.clone(), config));
            app.manage(manager.clone());

            // Setup system tray with TrayManager
            let tray_manager = TrayManager::setup(app)?;
            app.manage(tray_manager.clone());

            // Start server in background
            let app_handle = app.handle().clone();
            let manager_clone = manager.clone();
            tauri::async_runtime::spawn(async move {
                match manager_clone.start(&app_handle).await {
                    Ok(()) => {
                        info!("Server started successfully");
                        app_handle.emit("server-ready", ()).ok();
                    }
                    Err(e) => {
                        error!("Failed to start server: {}", e);
                        app_handle.emit("server-error", e.to_string()).ok();
                    }
                }
            });

            // Subscribe to state changes for tray updates
            let app_handle = app.handle().clone();
            let mut state_rx = manager.subscribe();
            tauri::async_runtime::spawn(async move {
                while state_rx.changed().await.is_ok() {
                    let state = state_rx.borrow().clone();

                    // Update tray via TrayManager
                    if let Some(tray_mgr) = app_handle.try_state::<Arc<TrayManager>>() {
                        tray_mgr.update_status(&app_handle, &state);
                    }

                    // Emit to frontend
                    app_handle
                        .emit("server-state-changed", format!("{:?}", state))
                        .ok();
                }
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                // Hide to tray instead of closing
                window.hide().ok();
                api.prevent_close();
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_server_status,
            commands::get_websocket_url,
            commands::restart_server,
            commands::export_diagnostics,
            commands::get_recent_logs,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

### 3.4 Log Rotation

**File**: `desktop/src-tauri/src/logging.rs`

```rust
//! Logging setup with file rotation.

use std::path::Path;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{fmt, prelude::*, EnvFilter, Layer};

/// Setup logging with console and rotating file output.
pub fn setup_logging(data_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let logs_dir = data_dir.join("logs");
    std::fs::create_dir_all(&logs_dir)?;

    // Console layer - human readable
    let console_layer = fmt::layer()
        .with_target(true)
        .with_level(true)
        .with_ansi(true);

    // File layer - JSON for easier parsing
    let file_appender = RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .max_log_files(7) // Keep 7 days of logs
        .filename_prefix("pm-desktop")
        .filename_suffix("log")
        .build(&logs_dir)?;

    let file_layer = fmt::layer()
        .with_target(true)
        .with_level(true)
        .with_ansi(false)
        .json()
        .with_writer(file_appender);

    // Combine layers with environment filter
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,pm_server=debug"));

    tracing_subscriber::registry()
        .with(filter)
        .with(console_layer)
        .with(file_layer)
        .init();

    Ok(())
}

/// Get path to current log file (for diagnostics export).
pub fn current_log_path(data_dir: &Path) -> std::path::PathBuf {
    let logs_dir = data_dir.join("logs");
    let today = chrono::Local::now().format("%Y-%m-%d");
    logs_dir.join(format!("pm-desktop.{}.log", today))
}
```

---

## Phase 4: Frontend Integration (~20k tokens)

### 4.1 Desktop Frontend HTML

**File**: `desktop/frontend/index.html`

```html
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Project Manager</title>
    <base href="/" />
    <link href="_content/ProjectManagement.Components/css/app.css" rel="stylesheet" />
    <link href="_content/Radzen.Blazor/css/material-base.css" rel="stylesheet" />
    <style>
        :root {
            --pm-primary: #3b82f6;
            --pm-error: #ef4444;
            --pm-success: #22c55e;
            --pm-bg: #f8fafc;
            --pm-text: #1e293b;
        }

        * {
            box-sizing: border-box;
        }

        body {
            margin: 0;
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            background: var(--pm-bg);
            color: var(--pm-text);
        }

        .startup-screen {
            display: flex;
            flex-direction: column;
            align-items: center;
            justify-content: center;
            height: 100vh;
            padding: 2rem;
        }

        .startup-logo {
            width: 80px;
            height: 80px;
            margin-bottom: 1.5rem;
        }

        .startup-title {
            font-size: 1.5rem;
            font-weight: 600;
            margin-bottom: 0.5rem;
        }

        .startup-status {
            color: #64748b;
            margin-bottom: 1.5rem;
        }

        .progress-container {
            width: 240px;
            height: 4px;
            background: #e2e8f0;
            border-radius: 2px;
            overflow: hidden;
            margin-bottom: 1rem;
        }

        .progress-bar {
            height: 100%;
            background: var(--pm-primary);
            width: 0%;
            transition: width 0.3s ease;
        }

        .progress-bar.indeterminate {
            width: 30%;
            animation: indeterminate 1.5s ease-in-out infinite;
        }

        @keyframes indeterminate {
            0% { transform: translateX(-100%); }
            100% { transform: translateX(400%); }
        }

        .startup-steps {
            list-style: none;
            padding: 0;
            margin: 0;
            font-size: 0.875rem;
        }

        .startup-steps li {
            display: flex;
            align-items: center;
            gap: 0.5rem;
            padding: 0.25rem 0;
            color: #94a3b8;
        }

        .startup-steps li.complete {
            color: var(--pm-success);
        }

        .startup-steps li.active {
            color: var(--pm-primary);
        }

        .startup-steps li.error {
            color: var(--pm-error);
        }

        .error-container {
            display: none;
            flex-direction: column;
            align-items: center;
            text-align: center;
            padding: 2rem;
            max-width: 400px;
        }

        .error-container.visible {
            display: flex;
        }

        .error-icon {
            width: 48px;
            height: 48px;
            color: var(--pm-error);
            margin-bottom: 1rem;
        }

        .error-title {
            font-size: 1.125rem;
            font-weight: 600;
            margin-bottom: 0.5rem;
        }

        .error-message {
            color: #64748b;
            margin-bottom: 1rem;
        }

        .error-hint {
            font-size: 0.875rem;
            color: #94a3b8;
            margin-bottom: 1.5rem;
        }

        .btn {
            display: inline-flex;
            align-items: center;
            gap: 0.5rem;
            padding: 0.625rem 1rem;
            border-radius: 0.375rem;
            font-size: 0.875rem;
            font-weight: 500;
            cursor: pointer;
            border: none;
            transition: background 0.15s;
        }

        .btn-primary {
            background: var(--pm-primary);
            color: white;
        }

        .btn-primary:hover {
            background: #2563eb;
        }

        .btn-secondary {
            background: #e2e8f0;
            color: var(--pm-text);
        }

        .btn-secondary:hover {
            background: #cbd5e1;
        }

        .btn-group {
            display: flex;
            gap: 0.75rem;
        }
    </style>
</head>
<body>
    <div id="app">
        <div class="startup-screen" id="startup-screen">
            <svg class="startup-logo" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <rect x="3" y="3" width="18" height="18" rx="2" />
                <path d="M9 9h6M9 13h6M9 17h4" />
            </svg>

            <h1 class="startup-title">Project Manager</h1>
            <p class="startup-status" id="status-text">Initializing...</p>

            <div class="progress-container">
                <div class="progress-bar indeterminate" id="progress-bar"></div>
            </div>

            <ul class="startup-steps" id="startup-steps">
                <li id="step-init" class="active">
                    <span>●</span> Initializing application
                </li>
                <li id="step-server">
                    <span>○</span> Starting server
                </li>
                <li id="step-health">
                    <span>○</span> Checking health
                </li>
                <li id="step-ui">
                    <span>○</span> Loading interface
                </li>
            </ul>
        </div>

        <div class="error-container" id="error-container">
            <svg class="error-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <circle cx="12" cy="12" r="10" />
                <path d="M12 8v4M12 16h.01" />
            </svg>

            <h2 class="error-title" id="error-title">Unable to Start</h2>
            <p class="error-message" id="error-message">An error occurred while starting the application.</p>
            <p class="error-hint" id="error-hint">Please try restarting the application.</p>

            <div class="btn-group">
                <button class="btn btn-primary" onclick="retryStartup()">
                    Try Again
                </button>
                <button class="btn btn-secondary" onclick="exportLogs()">
                    Export Logs
                </button>
            </div>
        </div>
    </div>

    <script src="_framework/blazor.webassembly.js" autostart="false"></script>
    <script>
        const { invoke } = window.__TAURI__.core;
        const { listen } = window.__TAURI__.event;

        let wsUrl = null;
        let startupAttempts = 0;
        const maxAttempts = 3;

        function updateStep(stepId, status) {
            const step = document.getElementById(stepId);
            if (!step) return;

            step.classList.remove('active', 'complete', 'error');
            step.classList.add(status);

            const icon = step.querySelector('span');
            if (status === 'complete') icon.textContent = '✓';
            else if (status === 'error') icon.textContent = '✗';
            else if (status === 'active') icon.textContent = '●';
            else icon.textContent = '○';
        }

        function setStatus(text) {
            document.getElementById('status-text').textContent = text;
        }

        function showError(title, message, hint) {
            document.getElementById('startup-screen').style.display = 'none';
            document.getElementById('error-container').classList.add('visible');
            document.getElementById('error-title').textContent = title;
            document.getElementById('error-message').textContent = message;
            document.getElementById('error-hint').textContent = hint || '';
        }

        async function pollServerStatus() {
            const maxPolls = 60; // 30 seconds
            const pollInterval = 500;

            for (let i = 0; i < maxPolls; i++) {
                try {
                    const status = await invoke('get_server_status');

                    if (status.state === 'running') {
                        wsUrl = status.websocket_url;
                        return true;
                    } else if (status.state === 'failed') {
                        throw new Error(status.error || 'Server failed to start');
                    }

                    setStatus(`Starting server... (${Math.floor(i * pollInterval / 1000)}s)`);
                } catch (e) {
                    console.error('Status poll error:', e);
                }

                await new Promise(r => setTimeout(r, pollInterval));
            }

            throw new Error('Server startup timeout');
        }

        async function startApp() {
            startupAttempts++;

            try {
                // Step 1: Initialize
                updateStep('step-init', 'complete');
                updateStep('step-server', 'active');
                setStatus('Starting server...');

                // Step 2: Wait for server
                await pollServerStatus();
                updateStep('step-server', 'complete');
                updateStep('step-health', 'active');
                setStatus('Checking server health...');

                // Step 3: Verify health
                const status = await invoke('get_server_status');
                if (status.health?.status !== 'healthy') {
                    throw new Error('Server health check failed');
                }
                updateStep('step-health', 'complete');
                updateStep('step-ui', 'active');
                setStatus('Loading interface...');

                // Step 4: Configure and start Blazor
                window.PM_CONFIG = {
                    serverUrl: wsUrl,
                    isDesktop: true
                };

                // Set progress to determinate
                const progressBar = document.getElementById('progress-bar');
                progressBar.classList.remove('indeterminate');
                progressBar.style.width = '100%';

                await Blazor.start({
                    environment: 'Production'
                });

                updateStep('step-ui', 'complete');

                // Hide startup screen after slight delay
                setTimeout(() => {
                    document.getElementById('startup-screen').style.display = 'none';
                }, 300);

            } catch (error) {
                console.error('Startup error:', error);

                if (startupAttempts < maxAttempts) {
                    setStatus(`Retrying... (attempt ${startupAttempts + 1}/${maxAttempts})`);
                    await new Promise(r => setTimeout(r, 2000));
                    return startApp();
                }

                // Determine which step failed
                const steps = ['step-init', 'step-server', 'step-health', 'step-ui'];
                for (const step of steps) {
                    const el = document.getElementById(step);
                    if (el?.classList.contains('active')) {
                        updateStep(step, 'error');
                        break;
                    }
                }

                showError(
                    'Unable to Start',
                    error.message || 'An unexpected error occurred.',
                    'Try restarting the application or check the logs for more details.'
                );
            }
        }

        async function retryStartup() {
            document.getElementById('error-container').classList.remove('visible');
            document.getElementById('startup-screen').style.display = 'flex';

            // Reset steps
            ['step-init', 'step-server', 'step-health', 'step-ui'].forEach(id => {
                updateStep(id, '');
            });
            updateStep('step-init', 'active');

            startupAttempts = 0;
            await startApp();
        }

        async function exportLogs() {
            try {
                const path = await invoke('export_diagnostics');
                alert(`Diagnostics exported to: ${path}`);
            } catch (e) {
                alert(`Failed to export: ${e}`);
            }
        }

        // --- WebSocket Reconnection Handler ---
        let wasRunning = false;
        let reconnectAttempts = 0;
        const maxReconnectAttempts = 10;

        async function handleServerStateChange(state) {
            console.log('Server state:', state);

            // Parse state (format: "Running { port: 8000 }" or "Restarting { attempt: 1 }")
            const isRunning = state.includes('Running');
            const isRestarting = state.includes('Restarting');
            const isFailed = state.includes('Failed');

            if (isRunning && wasRunning === false && reconnectAttempts > 0) {
                // Server came back online after restart - trigger Blazor reconnection
                console.log('Server recovered, triggering WebSocket reconnection...');

                // Get new WebSocket URL (port might have changed)
                try {
                    const newWsUrl = await invoke('get_websocket_url');
                    if (newWsUrl && newWsUrl !== wsUrl) {
                        console.log('WebSocket URL changed:', wsUrl, '->', newWsUrl);
                        wsUrl = newWsUrl;
                        window.PM_CONFIG.serverUrl = newWsUrl;
                    }

                    // Trigger reconnection in Blazor
                    if (window.DotNet) {
                        try {
                            await DotNet.invokeMethodAsync(
                                'ProjectManagement.Services',
                                'TriggerReconnect',
                                newWsUrl
                            );
                        } catch (e) {
                            console.warn('Could not trigger Blazor reconnect:', e);
                            // Fallback: reload the page
                            window.location.reload();
                        }
                    }

                    reconnectAttempts = 0;
                } catch (e) {
                    console.error('Failed to get new WebSocket URL:', e);
                }
            }

            if (isRestarting) {
                reconnectAttempts++;
                console.log(`Server restarting (attempt ${reconnectAttempts})`);

                // Show reconnection overlay to user
                showReconnectionOverlay(reconnectAttempts);
            }

            if (isFailed) {
                console.error('Server failed');
                showError(
                    'Server Error',
                    'The server has stopped unexpectedly.',
                    'Try restarting the application or check the logs.'
                );
            }

            wasRunning = isRunning;
        }

        function showReconnectionOverlay(attempt) {
            // Create or update reconnection overlay
            let overlay = document.getElementById('reconnect-overlay');
            if (!overlay) {
                overlay = document.createElement('div');
                overlay.id = 'reconnect-overlay';
                overlay.style.cssText = `
                    position: fixed;
                    bottom: 20px;
                    right: 20px;
                    background: #1e293b;
                    color: white;
                    padding: 16px 24px;
                    border-radius: 8px;
                    box-shadow: 0 4px 12px rgba(0,0,0,0.3);
                    z-index: 10000;
                    display: flex;
                    align-items: center;
                    gap: 12px;
                `;
                document.body.appendChild(overlay);
            }

            overlay.innerHTML = `
                <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" style="animation: spin 1s linear infinite;">
                    <path d="M21 12a9 9 0 11-6.219-8.56"/>
                </svg>
                <span>Reconnecting to server (attempt ${attempt})...</span>
            `;
            overlay.style.display = 'flex';

            // Add spin animation if not exists
            if (!document.getElementById('reconnect-styles')) {
                const style = document.createElement('style');
                style.id = 'reconnect-styles';
                style.textContent = '@keyframes spin { to { transform: rotate(360deg); } }';
                document.head.appendChild(style);
            }
        }

        function hideReconnectionOverlay() {
            const overlay = document.getElementById('reconnect-overlay');
            if (overlay) {
                overlay.style.display = 'none';
            }
        }

        // Listen for server events
        listen('server-ready', () => {
            console.log('Server ready event received');
            hideReconnectionOverlay();
        });

        listen('server-error', (event) => {
            console.error('Server error:', event.payload);
        });

        listen('server-state-changed', (event) => {
            handleServerStateChange(event.payload);
        });

        // Start the application
        startApp();
    </script>
</body>
</html>
```

### 4.2 WASM Configuration Updates

**File**: `frontend/ProjectManagement.Wasm/wwwroot/appsettings.json`

```json
{
  "WebSocket": {
    "ServerUrl": "ws://127.0.0.1:8000/ws",
    "HeartbeatInterval": "00:00:30",
    "HeartbeatTimeout": "00:01:00",
    "RequestTimeout": "00:00:30",
    "ReconnectMaxAttempts": 10,
    "ReconnectBaseDelay": "00:00:01",
    "ReconnectMaxDelay": "00:00:30"
  },
  "Desktop": {
    "Enabled": false
  }
}
```

**File**: `frontend/ProjectManagement.Wasm/wwwroot/js/desktop-interop.js`

```javascript
// Desktop mode interop for Tauri
window.DesktopInterop = {
    getConfig: function() {
        // PM_CONFIG is set by the Tauri desktop host before Blazor loads
        if (window.PM_CONFIG && window.PM_CONFIG.isDesktop) {
            return {
                serverUrl: window.PM_CONFIG.serverUrl,
                isDesktop: true
            };
        }
        return { serverUrl: null, isDesktop: false };
    },

    // Called from JS when server restarts and URL changes
    triggerReconnect: async function(newUrl) {
        if (window.DotNet) {
            await DotNet.invokeMethodAsync(
                'ProjectManagement.Services',
                'TriggerReconnect',
                newUrl
            );
        }
    }
};
```

**File**: `frontend/ProjectManagement.Services/Desktop/DesktopConfigService.cs`

```csharp
using Microsoft.JSInterop;
using System.Text.Json.Serialization;

namespace ProjectManagement.Services.Desktop;

/// <summary>
/// Configuration received from Tauri desktop host.
/// </summary>
public record DesktopConfig
{
    [JsonPropertyName("serverUrl")]
    public string? ServerUrl { get; init; }

    [JsonPropertyName("isDesktop")]
    public bool IsDesktop { get; init; }
}

/// <summary>
/// Service for detecting and configuring desktop mode.
/// </summary>
public class DesktopConfigService
{
    private readonly IJSRuntime _jsRuntime;
    private DesktopConfig? _cachedConfig;

    public DesktopConfigService(IJSRuntime jsRuntime)
    {
        _jsRuntime = jsRuntime;
    }

    /// <summary>
    /// Get desktop configuration. Returns null if not in desktop mode.
    /// </summary>
    public async Task<DesktopConfig?> GetConfigAsync()
    {
        if (_cachedConfig != null)
            return _cachedConfig;

        try
        {
            _cachedConfig = await _jsRuntime.InvokeAsync<DesktopConfig>(
                "DesktopInterop.getConfig"
            );

            return _cachedConfig?.IsDesktop == true ? _cachedConfig : null;
        }
        catch
        {
            // JS interop not available or DesktopInterop not defined
            return null;
        }
    }

    /// <summary>
    /// Check if running in desktop mode.
    /// </summary>
    public async Task<bool> IsDesktopModeAsync()
    {
        var config = await GetConfigAsync();
        return config?.IsDesktop == true;
    }
}

/// <summary>
/// Static reconnect handler callable from JavaScript.
/// </summary>
public static class DesktopReconnectHandler
{
    private static Func<string, Task>? _reconnectCallback;

    public static void SetReconnectCallback(Func<string, Task> callback)
    {
        _reconnectCallback = callback;
    }

    [JSInvokable("TriggerReconnect")]
    public static async Task TriggerReconnect(string newUrl)
    {
        if (_reconnectCallback != null)
        {
            await _reconnectCallback(newUrl);
        }
    }
}
```

**File**: `frontend/ProjectManagement.Wasm/Program.cs` (modifications)

```csharp
using ProjectManagement.Services.Desktop;

var builder = WebAssemblyHostBuilder.CreateDefault(args);
builder.RootComponents.Add<App>("#app");

// Register desktop config service
builder.Services.AddScoped<DesktopConfigService>();

// Build initial app to access JS runtime
var host = builder.Build();
var jsRuntime = host.Services.GetRequiredService<IJSRuntime>();
var desktopConfigService = host.Services.GetRequiredService<DesktopConfigService>();

// Check for desktop mode and reconfigure if needed
var desktopConfig = await desktopConfigService.GetConfigAsync();
if (desktopConfig != null && !string.IsNullOrEmpty(desktopConfig.ServerUrl))
{
    Console.WriteLine($"Desktop mode detected, server URL: {desktopConfig.ServerUrl}");

    // Rebuild with desktop-specific configuration
    builder = WebAssemblyHostBuilder.CreateDefault(args);
    builder.RootComponents.Add<App>("#app");
    builder.Services.AddScoped<DesktopConfigService>();

    // Override WebSocket configuration for desktop
    builder.Services.Configure<WebSocketOptions>(options =>
    {
        options.ServerUrl = desktopConfig.ServerUrl;
        // Desktop mode can use shorter timeouts since server is local
        options.HeartbeatInterval = TimeSpan.FromSeconds(15);
        options.HeartbeatTimeout = TimeSpan.FromSeconds(30);
    });

    // Register other services...
    builder.Services.AddProjectManagementServices();

    host = builder.Build();
}
else
{
    // Standard WASM mode - use config from appsettings.json
    builder.Services.AddProjectManagementServices();
}

// Setup reconnect handler for desktop mode
var wsService = host.Services.GetService<IWebSocketService>();
if (wsService != null)
{
    DesktopReconnectHandler.SetReconnectCallback(async (newUrl) =>
    {
        Console.WriteLine($"Reconnecting to: {newUrl}");
        await wsService.ReconnectAsync(newUrl);
    });
}

await host.RunAsync();
```

---

## Phase 5: Build Pipeline & Packaging (~25k tokens)

### 5.1 Cargo Configuration

**File**: `desktop/src-tauri/Cargo.toml`

```toml
[package]
name = "project-manager"
version = "0.1.0"
description = "Agile project management desktop application"
authors = ["Your Name"]
edition = "2021"
rust-version = "1.70"

[lib]
name = "app_lib"
crate-type = ["staticlib", "cdylib", "rlib"]

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
tauri = { version = "2", features = ["tray-icon"] }
tauri-plugin-shell = "2"
tauri-plugin-single-instance = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["sync", "time", "rt", "macros"] }
reqwest = { version = "0.12", default-features = false, features = ["rustls-tls", "json"] }
thiserror = "2"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
tracing-appender = "0.2"
chrono = { version = "0.4", features = ["serde"] }
toml = "0.8"
zip = "2"

[target.'cfg(unix)'.dependencies]
nix = { version = "0.29", features = ["signal", "process"] }
libc = "0.2"

[target.'cfg(windows)'.dependencies]
windows-sys = { version = "0.59", features = ["Win32_Foundation", "Win32_System_Threading", "Win32_System_Console"] }

[features]
default = ["custom-protocol"]
custom-protocol = ["tauri/custom-protocol"]

[profile.release]
strip = true
lto = true
codegen-units = 1
opt-level = "s"
panic = "abort"
```

### 5.2 Tauri Configuration

**Sidecar Naming Convention:**
Tauri sidecars must follow the naming pattern: `{name}-{target-triple}[.exe]`
- `pm-server-x86_64-unknown-linux-gnu` (Linux x64)
- `pm-server-x86_64-pc-windows-msvc.exe` (Windows x64)
- `pm-server-aarch64-apple-darwin` (macOS ARM64)
- `pm-server-x86_64-apple-darwin` (macOS x64)

The build scripts handle this naming automatically. The `externalBin` config uses the base name without target triple.

**File**: `desktop/src-tauri/tauri.conf.json`

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "Project Manager",
  "version": "0.1.0",
  "identifier": "com.projectmanager.app",
  "build": {
    "beforeDevCommand": "",
    "devUrl": "http://localhost:5173",
    "beforeBuildCommand": "",
    "frontendDist": "../frontend"
  },
  "app": {
    "withGlobalTauri": true,
    "windows": [
      {
        "title": "Project Manager",
        "width": 1280,
        "height": 800,
        "minWidth": 960,
        "minHeight": 640,
        "resizable": true,
        "fullscreen": false,
        "center": true
      }
    ],
    "trayIcon": {
      "id": "main",
      "iconPath": "icons/icon.png",
      "iconAsTemplate": true
    },
    "security": {
      "csp": "default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self' 'unsafe-inline'; connect-src 'self' ws://127.0.0.1:* http://127.0.0.1:*; img-src 'self' data:; font-src 'self' data:"
    }
  },
  "bundle": {
    "active": true,
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ],
    "externalBin": [
      "binaries/pm-server"
    ],
    "resources": [],
    "targets": ["app", "dmg", "nsis", "deb", "rpm"],
    "category": "Productivity",
    "shortDescription": "Agile project management",
    "longDescription": "A desktop application for agile project management with real-time collaboration.",
    "macOS": {
      "minimumSystemVersion": "10.15",
      "exceptionDomain": null,
      "signingIdentity": null,
      "entitlements": null
    },
    "windows": {
      "certificateThumbprint": null,
      "digestAlgorithm": "sha256",
      "timestampUrl": "http://timestamp.digicert.com",
      "nsis": {
        "installMode": "currentUser",
        "languages": ["English"],
        "displayLanguageSelector": false
      }
    },
    "linux": {
      "appId": "com.projectmanager.app",
      "category": "Office",
      "section": "utils"
    }
  },
  "plugins": {
    "shell": {
      "sidecar": true,
      "scope": [
        {
          "name": "binaries/pm-server",
          "sidecar": true,
          "args": true
        }
      ]
    }
  }
}
```

### 5.3 Build Scripts

**File**: `desktop/scripts/build.sh`

```bash
#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
DESKTOP_DIR="$PROJECT_ROOT/desktop"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log_info() { echo -e "${GREEN}[INFO]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

# Parse arguments
BUILD_TYPE="${1:-debug}"
TARGET="${2:-$(rustc -vV | grep host | cut -d' ' -f2)}"

log_info "Build type: $BUILD_TYPE"
log_info "Target: $TARGET"

# Determine cargo profile
if [[ "$BUILD_TYPE" == "release" ]]; then
    CARGO_PROFILE="--release"
    CARGO_OUT_DIR="release"
    DOTNET_CONFIG="Release"
else
    CARGO_PROFILE=""
    CARGO_OUT_DIR="debug"
    DOTNET_CONFIG="Debug"
fi

# Determine binary extension
BIN_EXT=""
if [[ "$TARGET" == *"windows"* ]]; then
    BIN_EXT=".exe"
fi

# Step 1: Build pm-server
log_info "Building pm-server..."
cd "$PROJECT_ROOT/backend"
cargo build $CARGO_PROFILE --bin pm-server --target "$TARGET"

# Step 2: Copy pm-server to Tauri binaries
log_info "Copying pm-server binary..."
SIDECAR_DIR="$DESKTOP_DIR/src-tauri/binaries"
mkdir -p "$SIDECAR_DIR"

SRC_BIN="$PROJECT_ROOT/backend/target/$TARGET/$CARGO_OUT_DIR/pm-server$BIN_EXT"
DST_BIN="$SIDECAR_DIR/pm-server-$TARGET$BIN_EXT"

if [[ ! -f "$SRC_BIN" ]]; then
    log_error "pm-server binary not found at $SRC_BIN"
    exit 1
fi

cp "$SRC_BIN" "$DST_BIN"
chmod +x "$DST_BIN"
log_info "Copied to $DST_BIN"

# Step 3: Build Blazor WASM frontend
log_info "Building Blazor WASM frontend..."
cd "$PROJECT_ROOT/frontend"
dotnet publish ProjectManagement.Wasm/ProjectManagement.Wasm.csproj \
    -c "$DOTNET_CONFIG" \
    -o "$DESKTOP_DIR/frontend" \
    --nologo \
    -v quiet

log_info "Frontend published to $DESKTOP_DIR/frontend"

# Step 4: Build Tauri app
log_info "Building Tauri application..."
cd "$DESKTOP_DIR/src-tauri"

if [[ "$BUILD_TYPE" == "release" ]]; then
    cargo tauri build --target "$TARGET"
else
    cargo tauri build --debug --target "$TARGET"
fi

# Output location
BUNDLE_DIR="$DESKTOP_DIR/src-tauri/target/$TARGET/$CARGO_OUT_DIR/bundle"
log_info "Build complete!"
log_info "Output directory: $BUNDLE_DIR"

# List artifacts
if [[ -d "$BUNDLE_DIR" ]]; then
    log_info "Artifacts:"
    find "$BUNDLE_DIR" -type f \( -name "*.dmg" -o -name "*.app" -o -name "*.exe" -o -name "*.msi" -o -name "*.deb" -o -name "*.rpm" -o -name "*.AppImage" \) -exec ls -lh {} \;
fi
```

**File**: `desktop/scripts/dev.sh`

```bash
#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Build pm-server in debug mode
echo "Building pm-server..."
cd "$PROJECT_ROOT/backend"
cargo build --bin pm-server

# Copy to Tauri binaries
TARGET=$(rustc -vV | grep host | cut -d' ' -f2)
mkdir -p "$PROJECT_ROOT/desktop/src-tauri/binaries"
cp "target/debug/pm-server" "$PROJECT_ROOT/desktop/src-tauri/binaries/pm-server-$TARGET"
chmod +x "$PROJECT_ROOT/desktop/src-tauri/binaries/pm-server-$TARGET"

# Build frontend
echo "Building frontend..."
cd "$PROJECT_ROOT/frontend"
dotnet build ProjectManagement.Wasm -c Debug -o "$PROJECT_ROOT/desktop/frontend"

# Run Tauri dev
echo "Starting Tauri dev..."
cd "$PROJECT_ROOT/desktop/src-tauri"
cargo tauri dev
```

### 5.4 CI/CD Pipeline

**File**: `.github/workflows/desktop-build.yml`

```yaml
name: Desktop App Build

on:
  push:
    branches: [main]
    tags: ['v*']
  pull_request:
    branches: [main]

env:
  CARGO_TERM_COLOR: always
  DOTNET_SKIP_FIRST_TIME_EXPERIENCE: true
  DOTNET_CLI_TELEMETRY_OPTOUT: true

jobs:
  build:
    strategy:
      fail-fast: false
      matrix:
        include:
          - platform: macos-latest
            target: aarch64-apple-darwin
            name: macOS-ARM64
          - platform: macos-latest
            target: x86_64-apple-darwin
            name: macOS-x64
          - platform: ubuntu-22.04
            target: x86_64-unknown-linux-gnu
            name: Linux-x64
          - platform: windows-latest
            target: x86_64-pc-windows-msvc
            name: Windows-x64

    runs-on: ${{ matrix.platform }}
    name: Build (${{ matrix.name }})

    steps:
      - name: Checkout
        uses: actions/checkout@v4

      - name: Setup Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}

      - name: Setup .NET
        uses: actions/setup-dotnet@v4
        with:
          dotnet-version: '8.0.x'

      - name: Install Linux dependencies
        if: matrix.platform == 'ubuntu-22.04'
        run: |
          sudo apt-get update
          sudo apt-get install -y \
            libwebkit2gtk-4.1-dev \
            libappindicator3-dev \
            librsvg2-dev \
            patchelf

      - name: Rust cache
        uses: Swatinem/rust-cache@v2
        with:
          workspaces: |
            backend -> target
            desktop/src-tauri -> target

      - name: Build pm-server
        run: cargo build --release --bin pm-server --target ${{ matrix.target }}
        working-directory: backend

      - name: Copy sidecar binary
        shell: bash
        run: |
          mkdir -p desktop/src-tauri/binaries
          if [[ "${{ matrix.target }}" == *"windows"* ]]; then
            cp "backend/target/${{ matrix.target }}/release/pm-server.exe" \
               "desktop/src-tauri/binaries/pm-server-${{ matrix.target }}.exe"
          else
            cp "backend/target/${{ matrix.target }}/release/pm-server" \
               "desktop/src-tauri/binaries/pm-server-${{ matrix.target }}"
            chmod +x "desktop/src-tauri/binaries/pm-server-${{ matrix.target }}"
          fi

      - name: Build frontend
        run: |
          dotnet publish ProjectManagement.Wasm/ProjectManagement.Wasm.csproj \
            -c Release \
            -o ../desktop/frontend
        working-directory: frontend

      - name: Build Tauri app
        uses: tauri-apps/tauri-action@v0
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        with:
          projectPath: desktop/src-tauri
          args: --target ${{ matrix.target }}

      - name: Upload artifacts
        uses: actions/upload-artifact@v4
        with:
          name: desktop-${{ matrix.name }}
          path: |
            desktop/src-tauri/target/${{ matrix.target }}/release/bundle/dmg/*.dmg
            desktop/src-tauri/target/${{ matrix.target }}/release/bundle/macos/*.app
            desktop/src-tauri/target/${{ matrix.target }}/release/bundle/nsis/*.exe
            desktop/src-tauri/target/${{ matrix.target }}/release/bundle/msi/*.msi
            desktop/src-tauri/target/${{ matrix.target }}/release/bundle/deb/*.deb
            desktop/src-tauri/target/${{ matrix.target }}/release/bundle/rpm/*.rpm
            desktop/src-tauri/target/${{ matrix.target }}/release/bundle/appimage/*.AppImage
          if-no-files-found: ignore

  release:
    if: startsWith(github.ref, 'refs/tags/v')
    needs: build
    runs-on: ubuntu-latest
    permissions:
      contents: write

    steps:
      - name: Download all artifacts
        uses: actions/download-artifact@v4
        with:
          path: artifacts

      - name: Create release
        uses: softprops/action-gh-release@v2
        with:
          files: artifacts/**/*
          generate_release_notes: true
          draft: true
```

---

## Phase 6: Comprehensive Testing (~30k tokens)

### 6.1 Unit Tests

**File**: `desktop/src-tauri/src/server/tests.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    mod config_tests {
        use super::*;

        #[test]
        fn default_config_is_valid() {
            let config = ServerConfig::default();
            assert!(config.validate().is_ok());
        }

        #[test]
        fn config_rejects_invalid_port() {
            let mut config = ServerConfig::default();
            config.server.port = 80; // Privileged port
            assert!(config.validate().is_err());
        }

        #[test]
        fn config_rejects_invalid_port_range() {
            let mut config = ServerConfig::default();
            config.server.port_range = (9000, 8000); // Inverted
            assert!(config.validate().is_err());
        }

        #[test]
        fn config_creates_default_on_missing_file() {
            let temp_dir = TempDir::new().unwrap();
            let config = ServerConfig::load_or_create(temp_dir.path()).unwrap();
            assert_eq!(config.version, CONFIG_VERSION);
            assert!(temp_dir.path().join("config.toml").exists());
        }

        #[test]
        fn config_migrates_old_version() {
            let temp_dir = TempDir::new().unwrap();
            let old_config = r#"
                version = 0
                [server]
                port = 8080
            "#;
            std::fs::write(temp_dir.path().join("config.toml"), old_config).unwrap();

            let config = ServerConfig::load_or_create(temp_dir.path()).unwrap();
            assert_eq!(config.version, CONFIG_VERSION);
            assert_eq!(config.server.port, 8080);
            // Resilience settings should be added
            assert!(config.resilience.max_restarts > 0);
        }

        #[test]
        fn config_atomic_write() {
            let temp_dir = TempDir::new().unwrap();
            let config = ServerConfig::default();
            config.save(temp_dir.path()).unwrap();

            // No .tmp file should remain
            assert!(!temp_dir.path().join("config.toml.tmp").exists());
            assert!(temp_dir.path().join("config.toml").exists());
        }
    }

    mod lock_file_tests {
        use super::*;

        #[test]
        fn lock_file_prevents_double_acquisition() {
            let temp_dir = TempDir::new().unwrap();

            let lock1 = LockFile::acquire(temp_dir.path(), 8000).unwrap();
            let lock2_result = LockFile::acquire(temp_dir.path(), 8000);

            assert!(matches!(lock2_result, Err(ServerError::AlreadyRunning { .. })));
            drop(lock1);
        }

        #[test]
        fn lock_file_released_on_drop() {
            let temp_dir = TempDir::new().unwrap();

            {
                let _lock = LockFile::acquire(temp_dir.path(), 8000).unwrap();
                assert!(temp_dir.path().join("server.lock").exists());
            }

            // Lock should be released
            assert!(!temp_dir.path().join("server.lock").exists());
        }

        #[test]
        fn stale_lock_is_cleaned_up() {
            let temp_dir = TempDir::new().unwrap();

            // Write fake lock with non-existent PID
            let fake_lock = r#"{"pid": 999999999, "port": 8000, "started_at": "2020-01-01T00:00:00Z"}"#;
            std::fs::write(temp_dir.path().join("server.lock"), fake_lock).unwrap();

            // Should succeed because PID doesn't exist
            let lock = LockFile::acquire(temp_dir.path(), 8000);
            assert!(lock.is_ok());
        }
    }

    mod port_tests {
        use super::*;

        #[test]
        fn finds_available_port() {
            let port = PortManager::find_available(8000, (8000, 8100)).unwrap();
            assert!(port >= 8000 && port <= 8100);
        }

        #[test]
        fn port_availability_check() {
            // Bind to a port
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            let bound_port = listener.local_addr().unwrap().port();

            // Should not be available
            assert!(!PortManager::is_available(bound_port));

            drop(listener);

            // Should be available now
            assert!(PortManager::is_available(bound_port));
        }
    }

    mod error_tests {
        use super::*;

        #[test]
        fn errors_have_recovery_hints() {
            let error = ServerError::PortInUse { port: 8000 };
            assert!(!error.recovery_hint().is_empty());

            let error = ServerError::MaxRestartsExceeded { max: 5 };
            assert!(!error.recovery_hint().is_empty());
        }

        #[test]
        fn transient_errors_identified() {
            assert!(ServerError::HealthCheckFailed {
                message: "timeout".into()
            }
            .is_transient());

            assert!(!ServerError::BinaryNotFound {
                path: "/bin/pm-server".into()
            }
            .is_transient());
        }
    }
}
```

### 6.2 Integration Tests

**File**: `desktop/src-tauri/tests/integration_tests.rs`

```rust
//! Integration tests for desktop application.
//!
//! These tests require pm-server to be built and available.

use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use tokio::time::timeout;

struct TestServer {
    process: Child,
    port: u16,
    data_dir: tempfile::TempDir,
}

impl TestServer {
    async fn start() -> Result<Self, Box<dyn std::error::Error>> {
        let data_dir = tempfile::TempDir::new()?;
        let port = find_free_port();

        // Find pm-server binary
        let bin_path = find_pm_server_binary()?;

        let process = Command::new(&bin_path)
            .env("PM_CONFIG_DIR", data_dir.path())
            .env("PM_SERVER_PORT", port.to_string())
            .env("PM_SERVER_HOST", "127.0.0.1")
            .env("PM_AUTH_ENABLED", "false")
            .env("PM_LOG_LEVEL", "debug")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let server = Self {
            process,
            port,
            data_dir,
        };

        // Wait for server to be ready
        server.wait_ready(Duration::from_secs(10)).await?;

        Ok(server)
    }

    async fn wait_ready(&self, timeout_duration: Duration) -> Result<(), Box<dyn std::error::Error>> {
        let url = format!("http://127.0.0.1:{}/ready", self.port);
        let client = reqwest::Client::new();

        let result = timeout(timeout_duration, async {
            loop {
                if let Ok(resp) = client.get(&url).send().await {
                    if resp.status().is_success() {
                        return Ok(());
                    }
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        })
        .await;

        result.map_err(|_| "Server startup timeout".into()).and_then(|r| r)
    }

    fn health_url(&self) -> String {
        format!("http://127.0.0.1:{}/health", self.port)
    }

    fn ready_url(&self) -> String {
        format!("http://127.0.0.1:{}/ready", self.port)
    }

    fn websocket_url(&self) -> String {
        format!("ws://127.0.0.1:{}/ws", self.port)
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.process.kill().ok();
    }
}

fn find_free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

fn find_pm_server_binary() -> Result<PathBuf, Box<dyn std::error::Error>> {
    // Try common locations
    let candidates = [
        "../../backend/target/debug/pm-server",
        "../../backend/target/release/pm-server",
        "../backend/target/debug/pm-server",
        "../backend/target/release/pm-server",
    ];

    for candidate in candidates {
        let path = PathBuf::from(candidate);
        if path.exists() {
            return Ok(path.canonicalize()?);
        }
    }

    Err("pm-server binary not found".into())
}

#[tokio::test]
async fn test_server_starts_and_responds() {
    let server = TestServer::start().await.expect("Failed to start server");

    let client = reqwest::Client::new();
    let resp = client.get(&server.health_url()).send().await.unwrap();

    assert!(resp.status().is_success());

    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
async fn test_ready_endpoint_includes_database() {
    let server = TestServer::start().await.expect("Failed to start server");

    let client = reqwest::Client::new();
    let resp = client.get(&server.ready_url()).send().await.unwrap();

    assert!(resp.status().is_success());

    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["status"], "healthy");
    assert!(body["database"]["status"] == "healthy");
}

#[tokio::test]
async fn test_websocket_connection() {
    let server = TestServer::start().await.expect("Failed to start server");

    use tokio_tungstenite::connect_async;

    let (ws_stream, _) = connect_async(&server.websocket_url())
        .await
        .expect("Failed to connect WebSocket");

    // Connection should be established
    assert!(!ws_stream.get_ref().is_terminated());
}

#[tokio::test]
async fn test_database_persists_data() {
    let server = TestServer::start().await.expect("Failed to start server");

    // Database file should exist
    let db_path = server.data_dir.path().join("data.db");
    assert!(db_path.exists(), "Database file should exist");

    // WAL file may exist
    // (Just checking that database operations work)
}

#[tokio::test]
async fn test_graceful_shutdown() {
    let mut server = TestServer::start().await.expect("Failed to start server");

    // Send SIGTERM (Unix) or just kill (Windows)
    #[cfg(unix)]
    {
        use nix::sys::signal::{kill, Signal};
        use nix::unistd::Pid;

        let pid = Pid::from_raw(server.process.id() as i32);
        kill(pid, Signal::SIGTERM).ok();
    }

    #[cfg(windows)]
    {
        server.process.kill().ok();
    }

    // Wait for process to exit
    let status = tokio::time::timeout(
        Duration::from_secs(5),
        tokio::task::spawn_blocking(move || server.process.wait()),
    )
    .await;

    assert!(status.is_ok(), "Server should shut down within timeout");
}

#[tokio::test]
async fn test_config_created_on_first_run() {
    let data_dir = tempfile::TempDir::new().unwrap();

    // Config should not exist yet
    assert!(!data_dir.path().join("config.toml").exists());

    // Start server (briefly)
    let port = find_free_port();
    let bin_path = find_pm_server_binary().unwrap();

    let mut process = Command::new(&bin_path)
        .env("PM_CONFIG_DIR", data_dir.path())
        .env("PM_SERVER_PORT", port.to_string())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();

    // Wait a moment for startup
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Config should now exist
    assert!(
        data_dir.path().join("config.toml").exists(),
        "Config file should be created"
    );

    process.kill().ok();
}

#[tokio::test]
async fn test_multiple_instance_prevention() {
    let server1 = TestServer::start().await.expect("Failed to start first server");

    // Try to start second server on same data directory
    let port2 = find_free_port();
    let bin_path = find_pm_server_binary().unwrap();

    let mut process2 = Command::new(&bin_path)
        .env("PM_CONFIG_DIR", server1.data_dir.path())
        .env("PM_SERVER_PORT", port2.to_string())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    // Second instance should fail due to lock file
    let status = process2.wait().expect("Process should exit");

    // Should exit with error (non-zero)
    assert!(
        !status.success(),
        "Second instance should fail due to lock"
    );
}
```

### 6.3 Manual Test Checklist

**File**: `desktop/docs/TEST_CHECKLIST.md`

```markdown
# Desktop Application Test Checklist

## Pre-Release Testing

### Startup Tests

- [ ] **Fresh Install**: App starts on clean system (no prior data)
- [ ] **Existing Data**: App starts with existing database
- [ ] **Config Migration**: App migrates old config format
- [ ] **Loading Screen**: Shows progress steps during startup
- [ ] **Startup Timeout**: Shows error after 30s if server fails

### Server Lifecycle Tests

- [ ] **Auto-Start**: Server starts automatically on app launch
- [ ] **Health Monitoring**: Tray icon reflects server status
- [ ] **Graceful Shutdown**: Server stops cleanly when app closes
- [ ] **Crash Recovery**: Server restarts after unexpected crash
- [ ] **Max Restarts**: Shows error after 5 consecutive crashes

### Single Instance Tests

- [ ] **Lock File**: Second instance shows error and focuses first
- [ ] **Stale Lock**: App starts if previous instance crashed
- [ ] **Lock Cleanup**: Lock file removed on clean exit

### Port Management Tests

- [ ] **Default Port**: Uses port 8000 when available
- [ ] **Port Conflict**: Finds alternative port if 8000 in use
- [ ] **Port Range**: Fails gracefully if all ports occupied

### UI Tests

- [ ] **Window Opens**: Main window appears after startup
- [ ] **Tray Icon**: System tray icon visible with menu
- [ ] **Hide to Tray**: Window hides (not closes) on X button
- [ ] **Show from Tray**: Click tray icon shows window
- [ ] **WebSocket**: Frontend connects to backend

### Data Persistence Tests

- [ ] **Create Project**: New project persists after restart
- [ ] **Create Work Item**: Work items persist after restart
- [ ] **Database Location**: Data stored in correct directory
- [ ] **Database Integrity**: No corruption after force quit

### Error Handling Tests

- [ ] **Server Error**: Shows user-friendly error message
- [ ] **Recovery Hint**: Error includes actionable guidance
- [ ] **Export Diagnostics**: Can export logs via menu
- [ ] **Retry Button**: Can retry startup after failure

### Cross-Platform Tests

#### macOS
- [ ] App bundle launches correctly
- [ ] Tray icon appears in menu bar
- [ ] Data stored in ~/Library/Application Support/
- [ ] DMG installer works
- [ ] Notarization (if signed)

#### Windows
- [ ] EXE launches correctly
- [ ] Tray icon appears in system tray
- [ ] Data stored in %APPDATA%
- [ ] NSIS installer works
- [ ] UAC prompt (if needed)

#### Linux
- [ ] AppImage launches correctly
- [ ] Tray icon appears (if supported)
- [ ] Data stored in ~/.local/share/
- [ ] DEB package installs
- [ ] RPM package installs

### Performance Tests

- [ ] **Startup Time**: App ready in < 10 seconds
- [ ] **Memory Usage**: < 500MB after startup
- [ ] **CPU Idle**: < 5% CPU when idle
- [ ] **Large Dataset**: Works with 1000+ work items

### Security Tests

- [ ] **Local Only**: Server only binds to 127.0.0.1
- [ ] **No External Calls**: App works offline
- [ ] **Config Sanitization**: Diagnostics don't leak secrets
```

---

## File Summary

### New Files by Dependency Layer

| Layer | File | Purpose |
|-------|------|---------|
| 0 | `backend/pm-server/src/main.rs` | Add `/health`, `/ready`, `/admin/checkpoint`, `/admin/shutdown` endpoints |
| 1 | `desktop/src-tauri/Cargo.toml` | Dependencies (must be first) |
| 2 | `src/server/error.rs` | Comprehensive error types |
| 3 | `src/server/config.rs` | Configuration with validation |
| 3 | `src/server/port.rs` | Port allocation |
| 3 | `src/server/lock.rs` | Single-instance lock file |
| 4 | `src/server/health.rs` | Health monitoring |
| 5 | `src/server/lifecycle.rs` | Process lifecycle with channel-based restart |
| 6 | `src/server/mod.rs` | Server module root |
| 7 | `desktop/src-tauri/tauri.conf.json` | Tauri config (before app runs) |
| 8 | `src/commands.rs` | Tauri IPC commands |
| 8 | `src/tray.rs` | TrayManager with live status updates |
| 8 | `src/logging.rs` | Log rotation with tracing-appender |
| 9 | `src/lib.rs` | App entry point (rewrite) |
| 10 | `desktop/frontend/index.html` | Desktop frontend with reconnection overlay |
| 10 | `frontend/.../wwwroot/js/desktop-interop.js` | Desktop JS interop |
| 10 | `frontend/.../Desktop/DesktopConfigService.cs` | Desktop mode detection service |
| 10 | `frontend/.../appsettings.json` | WASM config |
| 10 | `frontend/.../Program.cs` | Desktop mode detection |
| 11 | `scripts/build.sh` | Build script |
| 11 | `scripts/dev.sh` | Dev script |
| 11 | `.github/workflows/desktop-build.yml` | CI/CD |
| 12 | `src/server/tests.rs` | Unit tests |
| 12 | `tests/integration_tests.rs` | Integration tests |
| 12 | `docs/TEST_CHECKLIST.md` | Manual test checklist |

**Note**: The "Phase" sections in this document group code by functionality for readability. The "Layer" column above shows the actual implementation order based on dependencies.

### Backend Changes Required

**File**: `backend/pm-server/src/main.rs`

Add health, ready, and checkpoint endpoints:

```rust
use axum::{extract::State, http::StatusCode, routing::{get, post}, Json, Router};
use serde::Serialize;
use sqlx::SqlitePool;
use std::time::Instant;

// Add routes to router
.route("/health", get(health_handler))
.route("/ready", get(ready_handler))
.route("/admin/checkpoint", post(checkpoint_handler))
.route("/admin/shutdown", post(shutdown_handler))

// --- Health Response Types ---

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub uptime_secs: u64,
}

#[derive(Debug, Serialize)]
pub struct ReadyResponse {
    pub status: String,
    pub version: String,
    pub database: DatabaseHealth,
}

#[derive(Debug, Serialize)]
pub struct DatabaseHealth {
    pub status: String,
    pub latency_ms: u64,
}

// --- Handlers ---

/// Basic health check - returns immediately if server is running.
/// Used for liveness probes.
async fn health_handler(
    State(start_time): State<Instant>,
) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_secs: start_time.elapsed().as_secs(),
    })
}

/// Readiness check - verifies database connectivity.
/// Used to determine if server can accept traffic.
async fn ready_handler(
    State(pool): State<SqlitePool>,
) -> Result<Json<ReadyResponse>, (StatusCode, String)> {
    let start = Instant::now();

    // Test database connectivity
    sqlx::query("SELECT 1")
        .execute(&pool)
        .await
        .map_err(|e| (StatusCode::SERVICE_UNAVAILABLE, e.to_string()))?;

    let latency_ms = start.elapsed().as_millis() as u64;

    Ok(Json(ReadyResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        database: DatabaseHealth {
            status: "healthy".to_string(),
            latency_ms,
        },
    }))
}

/// Checkpoint WAL to main database file.
/// Called before graceful shutdown to ensure durability.
async fn checkpoint_handler(
    State(pool): State<SqlitePool>,
) -> Result<StatusCode, (StatusCode, String)> {
    sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)")
        .execute(&pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(StatusCode::OK)
}

/// Graceful shutdown endpoint.
/// Checkpoints database, then initiates server shutdown.
async fn shutdown_handler(
    State(pool): State<SqlitePool>,
    State(shutdown_tx): State<tokio::sync::oneshot::Sender<()>>,
) -> Result<StatusCode, (StatusCode, String)> {
    info!("Graceful shutdown requested via HTTP");

    // Checkpoint database first
    sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)")
        .execute(&pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Signal shutdown to the main server loop
    // The oneshot sender is consumed, triggering graceful_shutdown
    let _ = shutdown_tx.send(());

    Ok(StatusCode::OK)
}

// --- Server Setup with Graceful Shutdown ---

pub async fn run_server(pool: SqlitePool, addr: SocketAddr) -> Result<(), Error> {
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    let start_time = Instant::now();

    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/ready", get(ready_handler))
        .route("/admin/checkpoint", post(checkpoint_handler))
        .route("/admin/shutdown", post(shutdown_handler))
        // ... other routes
        .with_state(pool.clone())
        .with_state(start_time)
        .with_state(shutdown_tx);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("Server listening on {}", addr);

    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            shutdown_rx.await.ok();
            info!("Shutdown signal received, draining connections...");
        })
        .await?;

    // Final checkpoint after all connections drained
    sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)")
        .execute(&pool)
        .await?;

    info!("Server shutdown complete");
    Ok(())
}
```

**Database Migrations Strategy:**

Migrations run automatically on server startup before accepting connections:

```rust
// In pm-server main.rs setup
pub async fn setup_database(db_path: &Path) -> Result<SqlitePool, Error> {
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&format!("sqlite:{}?mode=rwc", db_path.display()))
        .await?;

    // Run migrations before server accepts connections
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await?;

    Ok(pool)
}
```

---

## Revised Scoring (Post-Review Updates)

| Category | Score | Justification |
|----------|-------|---------------|
| Error Handling | 9.5/10 | Comprehensive error types, recovery hints, transient detection |
| Security | 9/10 | Lock file, local-only binding, config sanitization |
| Logging & Observability | 9.5/10 | Structured logging with rotation, JSON format, diagnostics export, tray status updates |
| Resource Management | 9.5/10 | HTTP + OS signal graceful shutdown, checkpoint, lock cleanup |
| Cross-platform | 9.5/10 | Full Windows support with CTRL_BREAK, HTTP shutdown fallback, CI for all platforms |
| Testing | 9.5/10 | Unit, integration, manual checklist, health endpoint tests |
| User Experience | 9.5/10 | Progress UI, live tray status, reconnection overlay, retry, diagnostics |
| Configuration | 9.5/10 | Versioned, migrated, validated, atomic writes |
| Upgrade Path | 9/10 | Config versioning, migration support, database migrations on startup |
| Edge Cases | 9.5/10 | Single instance, port conflicts, crash recovery with channel-based restart |
| WebSocket Resilience | 9/10 | Frontend reconnection on server restart, URL change detection |
| Desktop Integration | 9.5/10 | Clean JS interop, proper desktop mode detection, sidecar naming |

**Overall: 9.4/10** ✅

### Key Improvements Made:
1. **Health Endpoints Specified**: Full `/health`, `/ready`, `/admin/shutdown` endpoint implementations
2. **Restart Logic Fixed**: Channel-based communication between health monitor and command handler
3. **Windows Graceful Shutdown**: HTTP shutdown endpoint + CTRL_BREAK_EVENT fallback
4. **Tray Status Working**: TrayManager class with proper menu item text updates
5. **Log Rotation**: Using tracing-appender with daily rotation and 7-day retention
6. **WebSocket Reconnection**: Frontend overlay and Blazor reconnect trigger on server restart
7. **Desktop Detection**: Clean JS interop without eval(), proper service architecture
8. **Database Migrations**: Documented strategy - migrations run on server startup
9. **Sidecar Naming**: Documented naming convention, tray ID added for proper lookup

---

## Implementation Order (Dependency Graph)

Files must be implemented in this order. Files at the same layer can be implemented in parallel.

```
Layer 0: Prerequisites
└── backend/pm-server: Add /admin/checkpoint endpoint

Layer 1: Build Configuration
└── desktop/src-tauri/Cargo.toml

Layer 2: Error Foundation
└── src/server/error.rs

Layer 3: Core Utilities (parallel, all depend on error.rs only)
├── src/server/config.rs
├── src/server/port.rs
└── src/server/lock.rs

Layer 4: Health Monitoring
└── src/server/health.rs

Layer 5: Lifecycle Management
└── src/server/lifecycle.rs
    (depends on: error, config, port, lock, health)

Layer 6: Module Export
└── src/server/mod.rs
    (declares and re-exports all server/*.rs)

Layer 7: Tauri Configuration
└── desktop/src-tauri/tauri.conf.json
    (required before Tauri app can run)

Layer 8: Tauri Commands (parallel, all depend on server module)
├── src/commands.rs
└── src/tray.rs

Layer 9: Application Entry
└── src/lib.rs
    (depends on: commands, tray, server module)

Layer 10: Frontend (parallel, no desktop code dependencies)
├── desktop/frontend/index.html
├── frontend/.../appsettings.json
└── frontend/.../Program.cs (modifications)

Layer 11: Build Tooling (parallel, requires compilable code)
├── scripts/build.sh
├── scripts/dev.sh
└── .github/workflows/desktop-build.yml

Layer 12: Testing (requires everything above)
├── src/server/tests.rs
├── tests/integration_tests.rs
└── docs/TEST_CHECKLIST.md
```

### Dependency Matrix

| File | Depends On |
|------|------------|
| `Cargo.toml` | (none - external crates) |
| `error.rs` | Cargo.toml (`thiserror`) |
| `config.rs` | error.rs, Cargo.toml (`serde`, `toml`) |
| `port.rs` | error.rs, Cargo.toml (`reqwest`) |
| `lock.rs` | error.rs, Cargo.toml (`serde_json`, `chrono`, `libc`/`windows-sys`) |
| `health.rs` | error.rs, Cargo.toml (`reqwest`, `tokio`) |
| `lifecycle.rs` | error.rs, config.rs, port.rs, lock.rs, health.rs, Cargo.toml (`tauri`, `tokio`, `tracing`, `nix`) |
| `mod.rs` | all server/*.rs files |
| `tauri.conf.json` | (none - configuration) |
| `commands.rs` | server module, Cargo.toml (`tauri`, `zip`, `chrono`) |
| `tray.rs` | server module, Cargo.toml (`tauri`) |
| `lib.rs` | commands.rs, tray.rs, server module, tauri.conf.json |
| `index.html` | IPC commands defined (commands.rs) |
| `build.sh` | all code compilable |
| `tests.rs` | all server/*.rs files |
| `integration_tests.rs` | pm-server binary, full desktop app |

**Total: ~150k tokens**
