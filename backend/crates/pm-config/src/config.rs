use crate::{
    AuthConfig, ConfigError, ConfigErrorResult, DatabaseConfig, LoggingConfig, ServerConfig,
};
use std::path::PathBuf;

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub auth: AuthConfig,
    pub logging: LoggingConfig,
}

impl Config {
    /// Load config from ~/.pm/config.toml, with env var overrides
    pub fn load() -> ConfigErrorResult<Self> {
        let config_dir = Self::config_dir()?;
        let config_path = config_dir.join("config.toml");

        let mut config = if config_path.exists() {
            let contents = std::fs::read_to_string(&config_path).map_err(|e| ConfigError::Io {
                path: config_path.clone(),
                source: e,
            })?;
            toml::from_str(&contents).map_err(|e| ConfigError::Toml {
                path: config_path,
                source: e,
            })?
        } else {
            Config::default()
        };

        // Apply env var overrides
        config.apply_env_overrides();

        Ok(config)
    }

    /// Get the config directory (~/.pm/)
    pub fn config_dir() -> Result<PathBuf, ConfigError> {
        let home = dirs::home_dir().ok_or(ConfigError::NoHomeDir)?;
        Ok(home.join(".pm"))
    }

    /// Get absolute path to database file
    pub fn database_path(&self) -> Result<PathBuf, ConfigError> {
        let config_dir = Self::config_dir()?;
        Ok(config_dir.join(&self.database.path))
    }

    /// Get bind address as string
    pub fn bind_addr(&self) -> String {
        format!("{}:{}", self.server.host, self.server.port)
    }

    fn apply_env_overrides(&mut self) {
        if let Ok(host) = std::env::var("PM_SERVER_HOST") {
            self.server.host = host;
        }
        if let Ok(port) = std::env::var("PM_SERVER_PORT")
            && let Ok(p) = port.parse()
        {
            self.server.port = p;
        }
        if let Ok(path) = std::env::var("PM_DATABASE_PATH") {
            self.database.path = path;
        }
        if let Ok(enabled) = std::env::var("PM_AUTH_ENABLED") {
            self.auth.enabled = enabled == "true" || enabled == "1";
        }
        if let Ok(level) = std::env::var("PM_LOG_LEVEL")
            && let Ok(parsed_level) = level.parse()
        {
            self.logging.level = parsed_level;
        }
    }
}
