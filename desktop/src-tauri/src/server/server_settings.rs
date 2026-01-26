use crate::server::config::{
    default_host, default_max_connections, default_port, default_port_range,
};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerSettings {
    /// Host to bind to (always 127.0.0.1 for security)
    #[serde(default = "default_host")]
    pub host: String,

    /// Preferred port
    #[serde(default = "default_port")]
    pub port: u16,

    /// Port range for fallback if primary port unavailable
    #[serde(default = "default_port_range")]
    pub port_range: (u16, u16),

    /// Maximum concurrent WebSocket connections
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,
}

impl Default for ServerSettings {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            port_range: default_port_range(),
            max_connections: default_max_connections(),
        }
    }
}
