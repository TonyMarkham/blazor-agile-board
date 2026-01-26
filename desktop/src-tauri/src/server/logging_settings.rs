use crate::server::config::{
    default_log_dir, default_log_level, default_log_retention, default_max_log_size,
};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingSettings {
    /// Log level (trace, debug, info, warn, error)
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
