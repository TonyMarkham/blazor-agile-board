mod auth_config;
mod config;
mod database_config;
mod error;
mod log_level;
mod logging_config;
mod server_config;

pub use auth_config::AuthConfig;
pub use config::Config;
pub use database_config::DatabaseConfig;
pub use error::{ConfigError, ConfigErrorResult};
pub use log_level::LogLevel;
pub use logging_config::LoggingConfig;
pub use server_config::ServerConfig;

const DEFAULT_HOST: &str = "127.0.0.1";
const DEFAULT_PORT: u16 = 8000;
const DEFAULT_DATABASE_FILENAME: &str = "data.db";
const DEFAULT_AUTH_ENABLED: bool = false;
const DEFAULT_LOG_LEVEL_STRING: &str = "info";
const DEFAULT_LOG_LEVEL: log::LevelFilter = log::LevelFilter::Info;
const DEFAULT_LOG_DIRECTORY: &str = "log";
