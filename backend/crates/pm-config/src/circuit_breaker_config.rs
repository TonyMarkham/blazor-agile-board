use crate::{ConfigError, ConfigErrorResult};

use serde::Deserialize;

// Circuit breaker constraints
pub const MIN_FAILURE_THRESHOLD: u32 = 1;
pub const MAX_FAILURE_THRESHOLD: u32 = 100;
pub const DEFAULT_FAILURE_THRESHOLD: u32 = 5;

pub const MIN_OPEN_DURATION_SECS: u64 = 1;
pub const MAX_OPEN_DURATION_SECS: u64 = 300;
pub const DEFAULT_OPEN_DURATION_SECS: u64 = 30;

pub const MIN_HALF_OPEN_SUCCESS_THRESHOLD: u32 = 1;
pub const MAX_HALF_OPEN_SUCCESS_THRESHOLD: u32 = 50;
pub const DEFAULT_HALF_OPEN_SUCCESS_THRESHOLD: u32 = 3;

pub const MIN_FAILURE_WINDOW_SECS: u64 = 1;
pub const MAX_FAILURE_WINDOW_SECS: u64 = 600;
pub const DEFAULT_FAILURE_WINDOW_SECS: u64 = 60;

/// Circuit breaker configuration for database resilience.
///
/// The circuit breaker prevents cascading failures by temporarily
/// blocking requests when the database is unhealthy.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct CircuitBreakerConfig {
    /// Number of failures before opening the circuit
    pub failure_threshold: u32,
    /// Seconds to keep circuit open before testing recovery
    pub open_duration_secs: u64,
    /// Successful requests needed in half-open state to close circuit
    pub half_open_success_threshold: u32,
    /// Window in seconds for counting failures
    pub failure_window_secs: u64,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: DEFAULT_FAILURE_THRESHOLD,
            open_duration_secs: DEFAULT_OPEN_DURATION_SECS,
            half_open_success_threshold: DEFAULT_HALF_OPEN_SUCCESS_THRESHOLD,
            failure_window_secs: DEFAULT_FAILURE_WINDOW_SECS,
        }
    }
}

impl CircuitBreakerConfig {
    pub fn validate(&self) -> ConfigErrorResult<()> {
        if self.failure_threshold < MIN_FAILURE_THRESHOLD
            || self.failure_threshold > MAX_FAILURE_THRESHOLD
        {
            return Err(ConfigError::config(format!(
                "circuit_breaker.failure_threshold must be {}-{}, got {}",
                MIN_FAILURE_THRESHOLD, MAX_FAILURE_THRESHOLD, self.failure_threshold
            )));
        }

        if self.open_duration_secs < MIN_OPEN_DURATION_SECS
            || self.open_duration_secs > MAX_OPEN_DURATION_SECS
        {
            return Err(ConfigError::config(format!(
                "circuit_breaker.open_duration_secs must be {}-{}, got {}",
                MIN_OPEN_DURATION_SECS, MAX_OPEN_DURATION_SECS, self.open_duration_secs
            )));
        }

        if self.half_open_success_threshold < MIN_HALF_OPEN_SUCCESS_THRESHOLD
            || self.half_open_success_threshold > MAX_HALF_OPEN_SUCCESS_THRESHOLD
        {
            return Err(ConfigError::config(format!(
                "circuit_breaker.half_open_success_threshold must be {}-{}, got {}",
                MIN_HALF_OPEN_SUCCESS_THRESHOLD,
                MAX_HALF_OPEN_SUCCESS_THRESHOLD,
                self.half_open_success_threshold
            )));
        }

        if self.failure_window_secs < MIN_FAILURE_WINDOW_SECS
            || self.failure_window_secs > MAX_FAILURE_WINDOW_SECS
        {
            return Err(ConfigError::config(format!(
                "circuit_breaker.failure_window_secs must be {}-{}, got {}",
                MIN_FAILURE_WINDOW_SECS, MAX_FAILURE_WINDOW_SECS, self.failure_window_secs
            )));
        }

        Ok(())
    }
}
