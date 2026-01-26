use crate::server::config::{default_idle_shutdown, default_ping_interval};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionSettings {
    /// WebSocket ping interval in seconds
    #[serde(default = "default_ping_interval")]
    pub ping_interval_secs: u64,

    /// Server idle shutdown timeout in seconds (0 = disabled)
    #[serde(default = "default_idle_shutdown")]
    pub idle_shutdown_secs: u64,
}

impl Default for ConnectionSettings {
    fn default() -> Self {
        Self {
            ping_interval_secs: default_ping_interval(),
            idle_shutdown_secs: default_idle_shutdown(),
        }
    }
}
