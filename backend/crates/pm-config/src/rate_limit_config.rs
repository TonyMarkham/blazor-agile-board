use crate::{ConfigError, ConfigErrorResult};

use serde::Deserialize;

// Rate limit constraints
pub const MIN_RATE_LIMIT_REQUESTS: u32 = 1;
pub const MAX_RATE_LIMIT_REQUESTS: u32 = 10000;
pub const DEFAULT_RATE_LIMIT_REQUESTS: u32 = 100;

pub const MIN_RATE_LIMIT_WINDOW_SECS: u64 = 1;
pub const MAX_RATE_LIMIT_WINDOW_SECS: u64 = 3600;
pub const DEFAULT_RATE_LIMIT_WINDOW_SECS: u64 = 60;

/// Rate limiting settings.
/// Applied per-connection to prevent abuse.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct RateLimitConfig {
    /// Maximum requests per window
    pub max_requests: u32,
    /// Window duration in seconds
    pub window_secs: u64,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_requests: DEFAULT_RATE_LIMIT_REQUESTS,
            window_secs: DEFAULT_RATE_LIMIT_WINDOW_SECS,
        }
    }
}

impl RateLimitConfig {
    pub fn validate(&self) -> ConfigErrorResult<()> {
        if self.max_requests < MIN_RATE_LIMIT_REQUESTS
            || self.max_requests > MAX_RATE_LIMIT_REQUESTS
        {
            return Err(ConfigError::config(format!(
                "rate_limit.max_requests must be {}-{}, got {}",
                MIN_RATE_LIMIT_REQUESTS, MAX_RATE_LIMIT_REQUESTS, self.max_requests
            )));
        }

        if self.window_secs < MIN_RATE_LIMIT_WINDOW_SECS
            || self.window_secs > MAX_RATE_LIMIT_WINDOW_SECS
        {
            return Err(ConfigError::config(format!(
                "rate_limit.window_secs must be {}-{}, got {}",
                MIN_RATE_LIMIT_WINDOW_SECS, MAX_RATE_LIMIT_WINDOW_SECS, self.window_secs
            )));
        }

        Ok(())
    }
}
