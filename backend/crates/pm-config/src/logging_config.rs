use crate::{DEFAULT_LOG_COLORED, DEFAULT_LOG_DIRECTORY, DEFAULT_LOG_LEVEL, LogLevel};

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct LoggingConfig {
    pub level: LogLevel,
    pub dir: String,
    /// Optional log file name (e.g., "pm-server.log")
    /// None = stdout, Some("name.log") = file output
    pub file: Option<String>,
    /// Enable colored output (default: true, ignored when logging to file)
    pub colored: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: LogLevel(DEFAULT_LOG_LEVEL),
            dir: String::from(DEFAULT_LOG_DIRECTORY),
            file: None,
            colored: DEFAULT_LOG_COLORED,
        }
    }
}
