mod auth_config;
mod circuit_breaker_config;
mod config;
mod database_config;
mod error;
mod handler_config;
mod log_level;
mod logging_config;
mod rate_limit_config;
mod retry_config;
mod server_config;
mod validation_config;
mod websocket_config;

mod activity_log_config;
mod api_config;

#[cfg(test)]
mod tests;

pub use activity_log_config::ActivityLogConfig;
pub use api_config::{ApiConfig, DEFAULT_LLM_USER_ID, DEFAULT_LLM_USER_NAME};
pub use auth_config::AuthConfig;
pub use circuit_breaker_config::CircuitBreakerConfig;
pub use config::Config;
pub use database_config::DatabaseConfig;
pub use error::{ConfigError, ConfigErrorResult};
pub use handler_config::HandlerConfig;
pub use log_level::LogLevel;
pub use logging_config::LoggingConfig;
pub use rate_limit_config::RateLimitConfig;
pub use retry_config::RetryConfig;
pub use server_config::ServerConfig;
pub use validation_config::{
    DEFAULT_TIME_ENTRIES_LIMIT, MAX_BLOCKED_DEPENDENCIES_PER_ITEM,
    MAX_BLOCKING_DEPENDENCIES_PER_ITEM, MAX_FUTURE_TIMESTAMP_TOLERANCE_SECONDS,
    MAX_TIME_ENTRIES_LIMIT, MAX_TIME_ENTRY_DESCRIPTION_LENGTH, MAX_TIME_ENTRY_DURATION_SECONDS,
    ValidationConfig,
};
pub use websocket_config::WebSocketConfig;

// =============================================================================
// Server Configuration
// =============================================================================

const DEFAULT_HOST: &str = "127.0.0.1";
const DEFAULT_PORT: u16 = 8000;
const MIN_PORT: u16 = 1024;
const MIN_MAX_CONNECTIONS: usize = 1;
const MAX_MAX_CONNECTIONS: usize = 100000;
const DEFAULT_MAX_CONNECTIONS: usize = 10000;

// =============================================================================
// Database Configuration
// =============================================================================

const DEFAULT_DATABASE_FILENAME: &str = "data.db";

// =============================================================================
// Authentication Configuration
// =============================================================================

const DEFAULT_AUTH_ENABLED: bool = false;
const DEFAULT_DESKTOP_USER_ID: &str = "local-user";
const MIN_JWT_SECRET_LENGTH: usize = 32;

// =============================================================================
// Logging Configuration
// =============================================================================

const DEFAULT_LOG_LEVEL_STRING: &str = "info";
const DEFAULT_LOG_LEVEL: log::LevelFilter = log::LevelFilter::Info;
const DEFAULT_LOG_DIRECTORY: &str = "log";
const DEFAULT_LOG_COLORED: bool = true;
