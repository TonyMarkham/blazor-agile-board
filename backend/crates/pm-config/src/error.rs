use std::panic::Location;
use std::path::PathBuf;
use std::result::Result as StdResult;

use error_location::ErrorLocation;
use thiserror::Error as ThisError;

#[derive(ThisError, Debug)]
pub enum ConfigError {
    #[error("{category} error: {message} {location}")]
    Generic {
        category: &'static str,
        message: String,
        location: ErrorLocation,
    },

    #[error("IO error reading {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("TOML parse error in {path}: {source}")]
    Toml {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },

    #[error("Home directory not found")]
    NoHomeDir,
}

impl ConfigError {
    /// Create an auth error
    #[track_caller]
    pub fn auth<S: Into<String>>(message: S) -> Self {
        ConfigError::Generic {
            category: "Auth",
            message: message.into(),
            location: ErrorLocation::from(Location::caller()),
        }
    }

    /// Create a config error
    #[track_caller]
    pub fn config<S: Into<String>>(message: S) -> Self {
        ConfigError::Generic {
            category: "Config",
            message: message.into(),
            location: ErrorLocation::from(Location::caller()),
        }
    }

    /// Create a database error
    #[track_caller]
    pub fn database<S: Into<String>>(message: S) -> Self {
        ConfigError::Generic {
            category: "Database",
            message: message.into(),
            location: ErrorLocation::from(Location::caller()),
        }
    }

    /// Create a logging error
    #[track_caller]
    pub fn logging<S: Into<String>>(message: S) -> Self {
        ConfigError::Generic {
            category: "Logging",
            message: message.into(),
            location: ErrorLocation::from(Location::caller()),
        }
    }

    /// Create a server error
    #[track_caller]
    pub fn server<S: Into<String>>(message: S) -> Self {
        ConfigError::Generic {
            category: "Server",
            message: message.into(),
            location: ErrorLocation::from(Location::caller()),
        }
    }

    /// Create a serde error
    #[track_caller]
    pub fn serde<S: Into<String>>(message: S) -> Self {
        ConfigError::Generic {
            category: "Serde",
            message: message.into(),
            location: ErrorLocation::from(Location::caller()),
        }
    }

    /// Convert a serde error into a ConfigError
    #[track_caller]
    pub fn from_serde_error<E: std::fmt::Display>(error: E) -> Self {
        Self::serde(error.to_string())
    }
}

impl serde::de::Error for ConfigError {
    fn custom<T: std::fmt::Display>(msg: T) -> Self {
        ConfigError::serde(msg.to_string())
    }
}

impl serde::ser::Error for ConfigError {
    fn custom<T: std::fmt::Display>(msg: T) -> Self {
        ConfigError::serde(msg.to_string())
    }
}

pub type ConfigErrorResult<T> = StdResult<T, ConfigError>;
