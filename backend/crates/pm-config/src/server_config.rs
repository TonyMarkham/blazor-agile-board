use crate::{
    ConfigError, ConfigErrorResult, DEFAULT_HOST, DEFAULT_MAX_CONNECTIONS, DEFAULT_PORT,
    MAX_MAX_CONNECTIONS, MIN_MAX_CONNECTIONS, MIN_PORT,
};

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    /// Maximum concurrent connections
    pub max_connections: usize,
    /// Auto-shutdown when no connections for N seconds (0 = disabled)
    pub idle_shutdown_secs: u64,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: String::from(DEFAULT_HOST),
            port: DEFAULT_PORT,
            max_connections: DEFAULT_MAX_CONNECTIONS,
            idle_shutdown_secs: 0,
        }
    }
}

impl ServerConfig {
    pub fn validate(&self) -> ConfigErrorResult<()> {
        // Port 0 means "auto-assign" - OS picks an available port.
        // Any other port must be >= MIN_PORT (1024).
        if self.port != 0 && self.port < MIN_PORT {
            return Err(ConfigError::config(format!(
                "server.port must be 0 (auto) or >= {}, got {}",
                MIN_PORT, self.port
            )));
        }

        if self.max_connections < MIN_MAX_CONNECTIONS || self.max_connections > MAX_MAX_CONNECTIONS
        {
            return Err(ConfigError::config(format!(
                "server.max_connections must be {}-{}, got {}",
                MIN_MAX_CONNECTIONS, MAX_MAX_CONNECTIONS, self.max_connections
            )));
        }

        Ok(())
    }
}
