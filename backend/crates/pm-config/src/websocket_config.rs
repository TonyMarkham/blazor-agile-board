use crate::{ConfigError, ConfigErrorResult};

use serde::Deserialize;

// Send buffer size constraints
pub const MIN_SEND_BUFFER_SIZE: usize = 1;
pub const MAX_SEND_BUFFER_SIZE: usize = 10000;
pub const DEFAULT_SEND_BUFFER_SIZE: usize = 100;

// Heartbeat interval constraints (seconds)
pub const MIN_HEARTBEAT_INTERVAL_SECS: u64 = 5;
pub const MAX_HEARTBEAT_INTERVAL_SECS: u64 = 300;
pub const DEFAULT_HEARTBEAT_INTERVAL_SECS: u64 = 30;

// Heartbeat timeout constraints (seconds)
pub const MIN_HEARTBEAT_TIMEOUT_SECS: u64 = 10;
pub const MAX_HEARTBEAT_TIMEOUT_SECS: u64 = 600;
pub const DEFAULT_HEARTBEAT_TIMEOUT_SECS: u64 = 60;

/// WebSocket connection settings.
/// All values validated to be within reasonable operational ranges.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct WebSocketConfig {
    /// Send buffer size
    pub send_buffer_size: usize,
    /// Heartbeat ping interval in seconds
    pub heartbeat_interval_secs: u64,
    /// Heartbeat timeout in seconds
    pub heartbeat_timeout_secs: u64,
}

impl Default for WebSocketConfig {
    fn default() -> Self {
        Self {
            send_buffer_size: DEFAULT_SEND_BUFFER_SIZE,
            heartbeat_interval_secs: DEFAULT_HEARTBEAT_INTERVAL_SECS,
            heartbeat_timeout_secs: DEFAULT_HEARTBEAT_TIMEOUT_SECS,
        }
    }
}

impl WebSocketConfig {
    /// Validate all fields are within acceptable ranges.
    pub fn validate(&self) -> ConfigErrorResult<()> {
        if self.send_buffer_size < MIN_SEND_BUFFER_SIZE
            || self.send_buffer_size > MAX_SEND_BUFFER_SIZE
        {
            return Err(ConfigError::config(format!(
                "websocket.send_buffer_size must be {}-{}, got {}",
                MIN_SEND_BUFFER_SIZE, MAX_SEND_BUFFER_SIZE, self.send_buffer_size
            )));
        }

        if self.heartbeat_interval_secs < MIN_HEARTBEAT_INTERVAL_SECS
            || self.heartbeat_interval_secs > MAX_HEARTBEAT_INTERVAL_SECS
        {
            return Err(ConfigError::config(format!(
                "websocket.heartbeat_interval_secs must be {}-{}, got {}",
                MIN_HEARTBEAT_INTERVAL_SECS,
                MAX_HEARTBEAT_INTERVAL_SECS,
                self.heartbeat_interval_secs
            )));
        }

        if self.heartbeat_timeout_secs < MIN_HEARTBEAT_TIMEOUT_SECS
            || self.heartbeat_timeout_secs > MAX_HEARTBEAT_TIMEOUT_SECS
        {
            return Err(ConfigError::config(format!(
                "websocket.heartbeat_timeout_secs must be {}-{}, got {}",
                MIN_HEARTBEAT_TIMEOUT_SECS, MAX_HEARTBEAT_TIMEOUT_SECS, self.heartbeat_timeout_secs
            )));
        }

        if self.heartbeat_timeout_secs <= self.heartbeat_interval_secs {
            return Err(ConfigError::config(format!(
                "websocket.heartbeat_timeout_secs ({}) must be greater than heartbeat_interval_secs ({})",
                self.heartbeat_timeout_secs, self.heartbeat_interval_secs
            )));
        }

        Ok(())
    }
}
