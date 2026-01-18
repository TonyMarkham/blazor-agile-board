use crate::{ConfigError, ConfigErrorResult};

use serde::Deserialize;

// Retry constraints
pub const MIN_MAX_ATTEMPTS: u32 = 1;
pub const MAX_MAX_ATTEMPTS: u32 = 10;
pub const DEFAULT_MAX_ATTEMPTS: u32 = 3;

pub const MIN_INITIAL_DELAY_MS: u64 = 10;
pub const MAX_INITIAL_DELAY_MS: u64 = 10000;
pub const DEFAULT_INITIAL_DELAY_MS: u64 = 100;

pub const MIN_MAX_DELAY_SECS: u64 = 1;
pub const MAX_MAX_DELAY_SECS: u64 = 60;
pub const DEFAULT_MAX_DELAY_SECS: u64 = 5;

pub const MIN_BACKOFF_MULTIPLIER: f64 = 1.0;
pub const MAX_BACKOFF_MULTIPLIER: f64 = 10.0;
pub const DEFAULT_BACKOFF_MULTIPLIER: f64 = 2.0;

pub const DEFAULT_JITTER: bool = true;

/// Retry configuration for transient failure handling.
///
/// Uses exponential backoff with optional jitter to prevent
/// thundering herd problems during recovery.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct RetryConfig {
    /// Maximum number of retry attempts (including initial attempt)
    pub max_attempts: u32,
    /// Initial delay before first retry in milliseconds
    pub initial_delay_ms: u64,
    /// Maximum delay between retries in seconds
    pub max_delay_secs: u64,
    /// Multiplier for exponential backoff (e.g., 2.0 = double each time)
    pub backoff_multiplier: f64,
    /// Add random jitter to delays to prevent thundering herd
    pub jitter: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: DEFAULT_MAX_ATTEMPTS,
            initial_delay_ms: DEFAULT_INITIAL_DELAY_MS,
            max_delay_secs: DEFAULT_MAX_DELAY_SECS,
            backoff_multiplier: DEFAULT_BACKOFF_MULTIPLIER,
            jitter: DEFAULT_JITTER,
        }
    }
}

impl RetryConfig {
    pub fn validate(&self) -> ConfigErrorResult<()> {
        if self.max_attempts < MIN_MAX_ATTEMPTS || self.max_attempts > MAX_MAX_ATTEMPTS {
            return Err(ConfigError::config(format!(
                "retry.max_attempts must be {}-{}, got {}",
                MIN_MAX_ATTEMPTS, MAX_MAX_ATTEMPTS, self.max_attempts
            )));
        }

        if self.initial_delay_ms < MIN_INITIAL_DELAY_MS
            || self.initial_delay_ms > MAX_INITIAL_DELAY_MS
        {
            return Err(ConfigError::config(format!(
                "retry.initial_delay_ms must be {}-{}, got {}",
                MIN_INITIAL_DELAY_MS, MAX_INITIAL_DELAY_MS, self.initial_delay_ms
            )));
        }

        if self.max_delay_secs < MIN_MAX_DELAY_SECS || self.max_delay_secs > MAX_MAX_DELAY_SECS {
            return Err(ConfigError::config(format!(
                "retry.max_delay_secs must be {}-{}, got {}",
                MIN_MAX_DELAY_SECS, MAX_MAX_DELAY_SECS, self.max_delay_secs
            )));
        }

        if self.backoff_multiplier < MIN_BACKOFF_MULTIPLIER
            || self.backoff_multiplier > MAX_BACKOFF_MULTIPLIER
        {
            return Err(ConfigError::config(format!(
                "retry.backoff_multiplier must be {}-{}, got {}",
                MIN_BACKOFF_MULTIPLIER, MAX_BACKOFF_MULTIPLIER, self.backoff_multiplier
            )));
        }

        Ok(())
    }
}
