//! Server configuration with validation and versioning.

use crate::server::{
    ConnectionSettings, DatabaseSettings, LoggingSettings, ResilienceSettings, ServerError,
    ServerResult, ServerSettings,
};

use std::panic::Location;
use std::path::Path;

use error_location::ErrorLocation;
use serde::{Deserialize, Serialize};

/// Configuration version for migration support.
/// Increment when adding new fields or changing structure.
pub const CONFIG_VERSION: u32 = 1;

const DEFAULT_HOST: &str = "127.0.0.1";
const DEFAULT_PORT: u16 = 8000;
const DEFAULT_PORT_RANGE_START: u16 = 8000;
const DEFAULT_PORT_RANGE_END: u16 = 8100;
const DEFAULT_MAX_CONNECTIONS: u32 = 100;
const DEFAULT_DB_FILENAME: &str = "data.db";
const DEFAULT_LOG_LEVEL: &str = "info";
const DEFAULT_LOG_DIR: &str = "logs";
const DEFAULT_MAX_LOG_SIZE_MB: u32 = 10;
const DEFAULT_LOG_RETENTION: u32 = 5;
const DEFAULT_MAX_RESTARTS: u32 = 5;
const DEFAULT_RESTART_WINDOW_SECS: u64 = 300; // 5 minutes
const DEFAULT_INITIAL_BACKOFF_MS: u64 = 100;
const DEFAULT_MAX_BACKOFF_MS: u64 = 30000; // 30 seconds
const DEFAULT_STARTUP_TIMEOUT_SECS: u64 = 30;
const DEFAULT_SHUTDOWN_TIMEOUT_SECS: u64 = 10;
const DEFAULT_HEALTH_INTERVAL_SECS: u64 = 5;
const DEFAULT_PING_INTERVAL_SECS: u64 = 30;
const DEFAULT_IDLE_SHUTDOWN_SECS: u64 = 120;

const MIN_PORT: u16 = 1024;

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

    /// Connection and idle settings
    #[serde(default)]
    pub connection: ConnectionSettings,
}

// === Default Value Functions ===

pub(crate) fn default_version() -> u32 {
    CONFIG_VERSION
}
pub(crate) fn default_host() -> String {
    DEFAULT_HOST.into()
}
pub(crate) fn default_port() -> u16 {
    DEFAULT_PORT
}
pub(crate) fn default_port_range() -> (u16, u16) {
    (DEFAULT_PORT_RANGE_START, DEFAULT_PORT_RANGE_END)
}
pub(crate) fn default_max_connections() -> u32 {
    DEFAULT_MAX_CONNECTIONS
}
pub(crate) fn default_db_filename() -> String {
    DEFAULT_DB_FILENAME.into()
}
pub(crate) fn default_true() -> bool {
    true
}
pub(crate) fn default_log_level() -> String {
    DEFAULT_LOG_LEVEL.into()
}
pub(crate) fn default_log_dir() -> String {
    DEFAULT_LOG_DIR.into()
}
pub(crate) fn default_max_log_size() -> u32 {
    DEFAULT_MAX_LOG_SIZE_MB
}
pub(crate) fn default_log_retention() -> u32 {
    DEFAULT_LOG_RETENTION
}
pub(crate) fn default_max_restarts() -> u32 {
    DEFAULT_MAX_RESTARTS
}
pub(crate) fn default_restart_window() -> u64 {
    DEFAULT_RESTART_WINDOW_SECS
}
pub(crate) fn default_initial_backoff() -> u64 {
    DEFAULT_INITIAL_BACKOFF_MS
}
pub(crate) fn default_max_backoff() -> u64 {
    DEFAULT_MAX_BACKOFF_MS
}
pub(crate) fn default_startup_timeout() -> u64 {
    DEFAULT_STARTUP_TIMEOUT_SECS
}
pub(crate) fn default_shutdown_timeout() -> u64 {
    DEFAULT_SHUTDOWN_TIMEOUT_SECS
}
pub(crate) fn default_health_interval() -> u64 {
    DEFAULT_HEALTH_INTERVAL_SECS
}
pub(crate) fn default_ping_interval() -> u64 {
    DEFAULT_PING_INTERVAL_SECS
}
pub(crate) fn default_idle_shutdown() -> u64 {
    DEFAULT_IDLE_SHUTDOWN_SECS
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            version: CONFIG_VERSION,
            server: ServerSettings::default(),
            database: DatabaseSettings::default(),
            logging: LoggingSettings::default(),
            resilience: ResilienceSettings::default(),
            connection: ConnectionSettings::default(),
        }
    }
}

// === Configuration Operations ===

impl ServerConfig {
    /// Load config from file, creating default if not exists.
    pub fn load_or_create(data_dir: &Path) -> ServerResult<Self> {
        let config_path = data_dir.join("config.toml");

        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            let mut config: Self =
                toml::from_str(&content).map_err(|e| ServerError::ConfigInvalid {
                    message: e.to_string(),
                    location: ErrorLocation::from(Location::caller()),
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

    /// Save config to file atomically.
    ///
    /// Uses write-to-temp-then-rename pattern to prevent
    /// partial writes if the process is interrupted.
    pub fn save(&self, data_dir: &Path) -> ServerResult<()> {
        let config_path = data_dir.join("config.toml");
        let content = toml::to_string_pretty(self).map_err(|e| ServerError::ConfigInvalid {
            message: e.to_string(),
            location: ErrorLocation::from(Location::caller()),
        })?;

        // Write atomically via temp file
        let temp_path = config_path.with_extension("toml.tmp");
        std::fs::write(&temp_path, &content)?;
        std::fs::rename(&temp_path, &config_path)?;

        Ok(())
    }

    /// Migrate config from older version.
    fn migrate(mut config: Self) -> ServerResult<Self> {
        // Version 0 -> 1: Add resilience settings
        if config.version == 0 {
            config.resilience = ResilienceSettings::default();
            config.version = 1;
        }

        // Future migrations go here as:
        // if config.version == 1 {
        //     // migrate to version 2
        //     config.version = 2;
        // }

        Ok(config)
    }

    /// Validate configuration values.
    pub fn validate(&self) -> ServerResult<()> {
        // Port must be unprivileged
        if self.server.port < MIN_PORT {
            return Err(ServerError::ConfigInvalid {
                message: format!("Port must be >= {} (unprivileged)", MIN_PORT),
                location: ErrorLocation::from(Location::caller()),
            });
        }

        // Port range must be valid
        if self.server.port_range.0 > self.server.port_range.1 {
            return Err(ServerError::ConfigInvalid {
                message: "Invalid port range: start > end".into(),
                location: ErrorLocation::from(Location::caller()),
            });
        }

        // Startup timeout must be positive
        if self.resilience.startup_timeout_secs == 0 {
            return Err(ServerError::ConfigInvalid {
                message: "Startup timeout must be > 0".into(),
                location: ErrorLocation::from(Location::caller()),
            });
        }

        // Host must be localhost for security
        if self.server.host != DEFAULT_HOST && self.server.host != "localhost" {
            return Err(ServerError::ConfigInvalid {
                message: format!("Host must be {DEFAULT_HOST} or localhost for security"),
                location: ErrorLocation::from(Location::caller()),
            });
        }

        // Idle shutdown must be > 2x ping interval to avoid false shutdowns
        if self.connection.idle_shutdown_secs > 0
            && self.connection.idle_shutdown_secs <= self.connection.ping_interval_secs * 2
        {
            return Err(ServerError::ConfigInvalid {
                message: format!(
                    "idle_shutdown_secs ({}) must be > 2x ping_interval_secs ({}) to avoid false shutdowns",
                    self.connection.idle_shutdown_secs, self.connection.ping_interval_secs
                ),
                location: ErrorLocation::from(Location::caller()),
            });
        }

        Ok(())
    }
}
