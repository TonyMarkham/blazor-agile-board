use std::panic::Location;
use std::path::PathBuf;

use error_location::ErrorLocation;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ServerError {
    #[error("Failed to create data directory at {path}: {source} {location}")]
    DataDirCreation {
        path: PathBuf,
        #[source]
        source: std::io::Error,
        location: ErrorLocation,
    },

    #[error("Configuration invalid: {message} {location}")]
    ConfigInvalid {
        message: String,
        location: ErrorLocation,
    },

    #[error("Failed to spawn server process: {source} {location}")]
    ProcessSpawn {
        #[source]
        source: tauri_plugin_shell::Error,
        location: ErrorLocation,
    },

    #[error("Server binary not found at {path} {location}")]
    BinaryNotFound {
        path: PathBuf,
        location: ErrorLocation,
    },

    #[error("Port {port} is in use by another application {location}")]
    PortInUse { port: u16, location: ErrorLocation },

    #[error("No available port in range {start}-{end} {location}")]
    NoAvailablePort {
        start: u16,
        end: u16,
        location: ErrorLocation,
    },

    #[error("Server failed to become ready within {timeout_secs}s {location}")]
    StartupTimeout {
        timeout_secs: u64,
        location: ErrorLocation,
    },

    #[error("Health check failed: {message} {location}")]
    HealthCheckFailed {
        message: String,
        location: ErrorLocation,
    },

    #[error("Server crashed with exit code {code:?}: {stderr} {location}")]
    ProcessCrashed {
        code: Option<i32>,
        stderr: String,
        location: ErrorLocation,
    },

    #[error("Graceful shutdown timed out after {timeout_secs}s {location}")]
    ShutdownTimeout {
        timeout_secs: u64,
        location: ErrorLocation,
    },

    #[error("Maximum restart attempts ({max}) exceeded {location}")]
    MaxRestartsExceeded { max: u32, location: ErrorLocation },

    #[error("Another instance is already running (lock file: {path}) {location}")]
    AlreadyRunning {
        path: PathBuf,
        location: ErrorLocation,
    },

    #[error("Failed to acquire lock at {path}: {source} {location}")]
    LockAcquisition {
        path: PathBuf,
        #[source]
        source: std::io::Error,
        location: ErrorLocation,
    },

    #[error("Database integrity check failed: {message} {location}")]
    DatabaseCorruption {
        message: String,
        location: ErrorLocation,
    },

    #[error("Failed to checkpoint database: {message} {location}")]
    CheckpointFailed {
        message: String,
        location: ErrorLocation,
    },

    #[error("IO error: {source} {location}")]
    Io {
        #[source]
        source: std::io::Error,
        location: ErrorLocation,
    },

    #[error("HTTP error: {source} {location}")]
    Http {
        #[source]
        source: reqwest::Error,
        location: ErrorLocation,
    },

    #[error("Server startup failed: {message}")]
    StartupFailed {
        message: String,
        location: ErrorLocation,
    },
}

impl ServerError {
    /// Whether this error is recoverable via retry
    pub fn is_transient(&self) -> bool {
        matches!(
            self,
            Self::HealthCheckFailed { .. } | Self::Http { .. } | Self::StartupTimeout { .. }
        )
    }

    pub fn recovery_hint(&self) -> &'static str {
        match self {
            Self::PortInUse { .. } => {
                "Another application is using the required port. \
                   Close other applications or restart your computer."
            }
            Self::NoAvailablePort { .. } => {
                "No available ports found in the configured range. \
                   Check your network configuration or restart your computer."
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
            Self::ConfigInvalid { .. } => {
                "Configuration file has invalid settings. \
                   Check the logs for details or delete the config file to use defaults."
            }
            Self::LockAcquisition { .. } => {
                "Unable to create lock file. \
                   Check file permissions in the application directory."
            }
            Self::DataDirCreation { .. } => {
                "Unable to create application data directory. \
                   Check file permissions or available disk space."
            }
            _ => "An unexpected error occurred. Please check the logs for details.",
        }
    }
}

impl From<std::io::Error> for ServerError {
    #[track_caller]
    fn from(source: std::io::Error) -> Self {
        Self::Io {
            source,
            location: ErrorLocation::from(Location::caller()),
        }
    }
}

impl From<reqwest::Error> for ServerError {
    #[track_caller]
    fn from(source: reqwest::Error) -> Self {
        Self::Http {
            source,
            location: ErrorLocation::from(Location::caller()),
        }
    }
}

impl From<tauri_plugin_shell::Error> for ServerError {
    #[track_caller]
    fn from(source: tauri_plugin_shell::Error) -> Self {
        Self::ProcessSpawn {
            source,
            location: ErrorLocation::from(Location::caller()),
        }
    }
}

pub type Result<T> = std::result::Result<T, ServerError>;
