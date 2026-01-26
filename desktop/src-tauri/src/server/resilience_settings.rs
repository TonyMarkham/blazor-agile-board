use crate::server::config::{
    default_health_interval, default_initial_backoff, default_max_backoff, default_max_restarts,
    default_restart_window, default_shutdown_timeout, default_startup_timeout,
};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResilienceSettings {
    /// Maximum server restart attempts before giving up
    #[serde(default = "default_max_restarts")]
    pub max_restarts: u32,

    /// Time window for restart counting (seconds)
    #[serde(default = "default_restart_window")]
    pub restart_window_secs: u64,

    /// Initial backoff delay (milliseconds)
    #[serde(default = "default_initial_backoff")]
    pub initial_backoff_ms: u64,

    /// Maximum backoff delay (milliseconds)
    #[serde(default = "default_max_backoff")]
    pub max_backoff_ms: u64,

    /// Startup timeout (seconds)
    #[serde(default = "default_startup_timeout")]
    pub startup_timeout_secs: u64,

    /// Graceful shutdown timeout (seconds)
    #[serde(default = "default_shutdown_timeout")]
    pub shutdown_timeout_secs: u64,

    /// Health check interval (seconds)
    #[serde(default = "default_health_interval")]
    pub health_check_interval_secs: u64,
}

impl Default for ResilienceSettings {
    fn default() -> Self {
        Self {
            max_restarts: default_max_restarts(),
            restart_window_secs: default_restart_window(),
            initial_backoff_ms: default_initial_backoff(),
            max_backoff_ms: default_max_backoff(),
            startup_timeout_secs: default_startup_timeout(),
            shutdown_timeout_secs: default_shutdown_timeout(),
            health_check_interval_secs: default_health_interval(),
        }
    }
}
