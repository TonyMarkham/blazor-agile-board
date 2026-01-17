use crate::DEFAULT_LOG_LEVEL_STRING;

use std::ops::Deref;
use std::str::FromStr;

use log::LevelFilter;
use serde::{Deserialize, Deserializer};

/// Wrapper for LevelFilter with custom deserialization
#[derive(Debug, Clone, Copy)]
pub struct LogLevel(pub LevelFilter);

impl<'de> Deserialize<'de> for LogLevel {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)
            .unwrap_or_else(|_| String::from(DEFAULT_LOG_LEVEL_STRING));

        // FromStr never fails, always returns a valid LogLevel (defaults to Info)
        Ok(LogLevel::from_str(&s).unwrap())
    }
}

impl From<LogLevel> for LevelFilter {
    fn from(log_level: LogLevel) -> Self {
        log_level.0
    }
}

impl Deref for LogLevel {
    type Target = LevelFilter;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromStr for LogLevel {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "off" => Ok(LogLevel(LevelFilter::Off)),
            "error" => Ok(LogLevel(LevelFilter::Error)),
            "warn" => Ok(LogLevel(LevelFilter::Warn)),
            "info" => Ok(LogLevel(LevelFilter::Info)),
            "debug" => Ok(LogLevel(LevelFilter::Debug)),
            "trace" => Ok(LogLevel(LevelFilter::Trace)),
            _ => Ok(LogLevel(LevelFilter::Info)), // Default to Info for invalid values
        }
    }
}
