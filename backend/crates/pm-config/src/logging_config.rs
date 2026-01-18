use crate::{DEFAULT_LOG_COLORED, DEFAULT_LOG_DIRECTORY, DEFAULT_LOG_LEVEL, LogLevel};

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct LoggingConfig {
    pub level: LogLevel,
    pub dir: String,
    /// Enable colored output (default: true)
    pub colored: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: LogLevel(DEFAULT_LOG_LEVEL),
            dir: String::from(DEFAULT_LOG_DIRECTORY),
            colored: DEFAULT_LOG_COLORED,
        }
    }
}
