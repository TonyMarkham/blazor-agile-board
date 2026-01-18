use crate::{ConfigError, ConfigErrorResult};

use serde::Deserialize;

// Handler constraints
pub const MIN_TIMEOUT_SECS: u64 = 1;
pub const MAX_TIMEOUT_SECS: u64 = 300;
pub const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// Handler configuration for request processing.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct HandlerConfig {
    /// Maximum time in seconds for a handler to complete
    pub timeout_secs: u64,
}

impl Default for HandlerConfig {
    fn default() -> Self {
        Self {
            timeout_secs: DEFAULT_TIMEOUT_SECS,
        }
    }
}

impl HandlerConfig {
    pub fn validate(&self) -> ConfigErrorResult<()> {
        if self.timeout_secs < MIN_TIMEOUT_SECS || self.timeout_secs > MAX_TIMEOUT_SECS {
            return Err(ConfigError::config(format!(
                "handler.timeout_secs must be {}-{}, got {}",
                MIN_TIMEOUT_SECS, MAX_TIMEOUT_SECS, self.timeout_secs
            )));
        }

        Ok(())
    }
}
