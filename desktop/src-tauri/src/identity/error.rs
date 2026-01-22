use std::panic::Location;
use std::path::PathBuf;

use error_location::ErrorLocation;
use thiserror::Error;

/// Errors related to user identity management.
#[derive(Error, Debug)]
pub enum IdentityError {
    #[error("Failed to get app data directory: {message} {location}")]
    AppDataDir {
        message: String,
        location: ErrorLocation,
    },

    #[error("Failed to create directory at {path}: {source} {location}")]
    DirCreation {
        path: PathBuf,
        #[source]
        source: std::io::Error,
        location: ErrorLocation,
    },

    #[error("Failed to read identity file at {path}: {source} {location}")]
    FileRead {
        path: PathBuf,
        #[source]
        source: std::io::Error,
        location: ErrorLocation,
    },

    #[error("Failed to write identity file at {path}: {source} {location}")]
    FileWrite {
        path: PathBuf,
        #[source]
        source: std::io::Error,
        location: ErrorLocation,
    },

    #[error("Identity file corrupted at {path}: {message} {location}")]
    Corrupted {
        path: PathBuf,
        message: String,
        location: ErrorLocation,
    },

    #[error("Failed to serialize identity: {source} {location}")]
    Serialization {
        #[source]
        source: serde_json::Error,
        location: ErrorLocation,
    },

    #[error("Atomic rename failed from {from} to {to}: {source} {location}")]
    AtomicRename {
        from: PathBuf,
        to: PathBuf,
        #[source]
        source: std::io::Error,
        location: ErrorLocation,
    },

    #[error("Failed to backup corrupted file: {source} {location}")]
    BackupFailed {
        #[source]
        source: std::io::Error,
        location: ErrorLocation,
    },
}

impl IdentityError {
    /// Whether this error is recoverable via retry.
    pub fn is_transient(&self) -> bool {
        matches!(
            self,
            Self::FileRead { .. } | Self::FileWrite { .. } | Self::AtomicRename { .. }
        )
    }

    /// User-friendly recovery hint.
    pub fn recovery_hint(&self) -> &'static str {
        match self {
            Self::AppDataDir { .. } => {
                "Unable to locate application data directory. \
                   Try restarting the application or reinstalling."
            }
            Self::DirCreation { .. } | Self::FileWrite { .. } => {
                "Unable to write to application data directory. \
                   Check disk space and file permissions."
            }
            Self::FileRead { .. } => {
                "Unable to read identity file. \
                   The file may be locked by another process."
            }
            Self::Corrupted { .. } => {
                "Your identity file is corrupted. \
                   A backup will be created and you'll need to re-register."
            }
            Self::Serialization { .. } => {
                "Internal error preparing identity data. \
                   Please report this issue."
            }
            Self::AtomicRename { .. } => {
                "Unable to save identity file safely. \
                   Check disk space and try again."
            }
            Self::BackupFailed { .. } => {
                "Unable to backup corrupted file. \
                   Check file permissions in the application directory."
            }
        }
    }

    /// Creates AppDataDir error at caller location.
    #[track_caller]
    pub fn app_data_dir(message: impl Into<String>) -> Self {
        Self::AppDataDir {
            message: message.into(),
            location: ErrorLocation::from(Location::caller()),
        }
    }

    /// Creates Corrupted error at caller location.
    #[track_caller]
    pub fn corrupted(path: PathBuf, message: impl Into<String>) -> Self {
        Self::Corrupted {
            path,
            message: message.into(),
            location: ErrorLocation::from(Location::caller()),
        }
    }

    /// Creates DirCreation error at caller location.
    #[track_caller]
    pub fn dir_creation(path: PathBuf, source: std::io::Error) -> Self {
        Self::DirCreation {
            path,
            source,
            location: ErrorLocation::from(Location::caller()),
        }
    }

    /// Creates FileRead error at caller location.
    #[track_caller]
    pub fn file_read(path: PathBuf, source: std::io::Error) -> Self {
        Self::FileRead {
            path,
            source,
            location: ErrorLocation::from(Location::caller()),
        }
    }

    /// Creates FileWrite error at caller location.
    #[track_caller]
    pub fn file_write(path: PathBuf, source: std::io::Error) -> Self {
        Self::FileWrite {
            path,
            source,
            location: ErrorLocation::from(Location::caller()),
        }
    }

    /// Creates AtomicRename error at caller location.
    #[track_caller]
    pub fn atomic_rename(from: PathBuf, to: PathBuf, source: std::io::Error) -> Self {
        Self::AtomicRename {
            from,
            to,
            source,
            location: ErrorLocation::from(Location::caller()),
        }
    }
}

impl From<serde_json::Error> for IdentityError {
    #[track_caller]
    fn from(source: serde_json::Error) -> Self {
        Self::Serialization {
            source,
            location: ErrorLocation::from(Location::caller()),
        }
    }
}

pub type Result<T> = std::result::Result<T, IdentityError>;
