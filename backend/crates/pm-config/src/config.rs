use crate::{
    ActivityLogConfig, AuthConfig, CircuitBreakerConfig, ConfigError, ConfigErrorResult,
    DatabaseConfig, HandlerConfig, LoggingConfig, RateLimitConfig, RetryConfig, ServerConfig,
    ValidationConfig, WebSocketConfig,
};

use std::path::PathBuf;

use log::info;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub auth: AuthConfig,
    pub logging: LoggingConfig,
    #[serde(default)]
    pub activity_log: ActivityLogConfig,
    pub websocket: WebSocketConfig,
    pub rate_limit: RateLimitConfig,
    pub circuit_breaker: CircuitBreakerConfig,
    pub retry: RetryConfig,
    pub handler: HandlerConfig,
    pub validation: ValidationConfig,
}

impl Config {
    /// Load config with full production error handling.
    ///
    /// Loading order:
    /// 1. Check for PM_CONFIG_DIR env var, else use ./.pm/
    /// 2. Auto-create config directory if it doesn't exist
    /// 3. Load config.toml if it exists, else use defaults
    /// 4. Apply PM_* environment variable overrides
    /// 5. Check for legacy ~/.pm/config.toml and warn
    ///
    /// Does NOT validate - call validate() after load().
    pub fn load() -> ConfigErrorResult<Self> {
        let config_dir = Self::config_dir()?;

        // Auto-create config directory
        if !config_dir.exists() {
            std::fs::create_dir_all(&config_dir).map_err(|e| ConfigError::Io {
                path: config_dir.clone(),
                source: e,
            })?;
        }

        let config_path = config_dir.join("config.toml");

        let mut config = if config_path.exists() {
            Self::load_toml(&config_path)?
        } else {
            Config::default()
        };

        config.apply_env_overrides();

        Ok(config)
    }

    /// Load and parse TOML file with detailed error context.
    fn load_toml(path: &PathBuf) -> ConfigErrorResult<Self> {
        let contents = std::fs::read_to_string(path).map_err(|e| ConfigError::Io {
            path: path.clone(),
            source: e,
        })?;

        toml::from_str(&contents).map_err(|e| ConfigError::Toml {
            path: path.clone(),
            source: e,
        })
    }

    /// Get the config directory.
    /// Priority: PM_CONFIG_DIR env var > ./.pm/ (relative to cwd)
    pub fn config_dir() -> Result<PathBuf, ConfigError> {
        if let Ok(dir) = std::env::var("PM_CONFIG_DIR") {
            return Ok(PathBuf::from(dir));
        }

        let cwd = std::env::current_dir()
            .map_err(|_| ConfigError::config("Cannot determine current working directory"))?;
        Ok(cwd.join(".pm"))
    }

    /// Validate all configuration.
    /// Call after load() to catch all errors at startup.
    pub fn validate(&self) -> ConfigErrorResult<()> {
        let config_dir = Self::config_dir()?;

        self.server.validate()?;
        self.auth.validate(&config_dir)?;
        self.websocket.validate()?;
        self.rate_limit.validate()?;
        self.circuit_breaker.validate()?;
        self.retry.validate()?;
        self.handler.validate()?;
        self.validation.validate()?;

        // Validate database path doesn't escape config dir
        let db_path = std::path::Path::new(&self.database.path);
        if db_path.is_absolute() || self.database.path.contains("..") {
            return Err(ConfigError::database(
                "database.path must be relative and cannot contain '..'",
            ));
        }

        Ok(())
    }

    /// Get absolute path to database file.
    pub fn database_path(&self) -> Result<PathBuf, ConfigError> {
        let config_dir = Self::config_dir()?;
        Ok(config_dir.join(&self.database.path))
    }

    /// Get bind address as string.
    pub fn bind_addr(&self) -> String {
        format!("{}:{}", self.server.host, self.server.port)
    }

    /// Log configuration summary (NEVER logs secrets).
    pub fn log_summary(&self) {
        info!("Configuration loaded:");
        info!(
            "  server: {}:{} (max {} connections)",
            self.server.host, self.server.port, self.server.max_connections
        );
        info!("  database: {}", self.database.path);

        let auth_type = if self.auth.jwt_secret.is_some() {
            "HS256"
        } else if self.auth.jwt_public_key_path.is_some() {
            "RS256"
        } else {
            "none"
        };

        info!(
            "  auth: {} ({})",
            if self.auth.enabled {
                "enabled"
            } else {
                "disabled"
            },
            auth_type
        );

        info!(
            "  activity_log: retention={}d, cleanup={}h",
            self.activity_log.retention_days, self.activity_log.cleanup_interval_hours
        );

        info!(
            "  logging: {} (colored: {})",
            *self.logging.level, self.logging.colored
        );

        info!(
            "  websocket: buffer={}, heartbeat={}s/{}s",
            self.websocket.send_buffer_size,
            self.websocket.heartbeat_interval_secs,
            self.websocket.heartbeat_timeout_secs
        );

        info!(
            "  rate_limit: {}/{}s",
            self.rate_limit.max_requests, self.rate_limit.window_secs
        );

        // NEW LOGGING BELOW
        info!(
            "  circuit_breaker: threshold={}, open={}s, window={}s",
            self.circuit_breaker.failure_threshold,
            self.circuit_breaker.open_duration_secs,
            self.circuit_breaker.failure_window_secs
        );

        info!(
            "  retry: attempts={}, initial={}ms, max={}s, backoff={}x",
            self.retry.max_attempts,
            self.retry.initial_delay_ms,
            self.retry.max_delay_secs,
            self.retry.backoff_multiplier
        );

        info!("  handler: timeout={}s", self.handler.timeout_secs);

        info!(
            "  validation: title={}, desc={}, points={}",
            self.validation.max_title_length,
            self.validation.max_description_length,
            self.validation.max_story_points
        );
    }

    fn apply_env_overrides(&mut self) {
        // Server
        Self::apply_env_string("PM_SERVER_HOST", &mut self.server.host);
        Self::apply_env_parse("PM_SERVER_PORT", &mut self.server.port);
        Self::apply_env_parse(
            "PM_SERVER_MAX_CONNECTIONS",
            &mut self.server.max_connections,
        );
        Self::apply_env_parse("PM_IDLE_SHUTDOWN_SECS", &mut self.server.idle_shutdown_secs);

        // Database
        Self::apply_env_string("PM_DATABASE_PATH", &mut self.database.path);

        // Auth
        Self::apply_env_bool("PM_AUTH_ENABLED", &mut self.auth.enabled);
        Self::apply_env_option_string("PM_AUTH_JWT_SECRET", &mut self.auth.jwt_secret);
        Self::apply_env_option_string(
            "PM_AUTH_JWT_PUBLIC_KEY_PATH",
            &mut self.auth.jwt_public_key_path,
        );
        Self::apply_env_option_string("PM_AUTH_DESKTOP_USER_ID", &mut self.auth.desktop_user_id);

        // Logging
        Self::apply_env_parse("PM_LOG_LEVEL", &mut self.logging.level);
        Self::apply_env_bool("PM_LOG_COLORED", &mut self.logging.colored);
        Self::apply_env_option_string("PM_LOG_FILE", &mut self.logging.file);

        // Activity Log
        Self::apply_env_parse(
            "PM_ACTIVITY_LOG_RETENTION_DAYS",
            &mut self.activity_log.retention_days,
        );
        Self::apply_env_parse(
            "PM_ACTIVITY_LOG_CLEANUP_INTERVAL_HOURS",
            &mut self.activity_log.cleanup_interval_hours,
        );

        // WebSocket
        Self::apply_env_parse(
            "PM_WS_SEND_BUFFER_SIZE",
            &mut self.websocket.send_buffer_size,
        );
        Self::apply_env_parse(
            "PM_WS_HEARTBEAT_INTERVAL_SECS",
            &mut self.websocket.heartbeat_interval_secs,
        );
        Self::apply_env_parse(
            "PM_WS_HEARTBEAT_TIMEOUT_SECS",
            &mut self.websocket.heartbeat_timeout_secs,
        );

        // Rate limit
        Self::apply_env_parse(
            "PM_RATE_LIMIT_MAX_REQUESTS",
            &mut self.rate_limit.max_requests,
        );
        Self::apply_env_parse(
            "PM_RATE_LIMIT_WINDOW_SECS",
            &mut self.rate_limit.window_secs,
        );

        // NEW OVERRIDES BELOW

        // Circuit Breaker
        Self::apply_env_parse(
            "PM_CB_FAILURE_THRESHOLD",
            &mut self.circuit_breaker.failure_threshold,
        );
        Self::apply_env_parse(
            "PM_CB_OPEN_DURATION_SECS",
            &mut self.circuit_breaker.open_duration_secs,
        );
        Self::apply_env_parse(
            "PM_CB_HALF_OPEN_SUCCESS_THRESHOLD",
            &mut self.circuit_breaker.half_open_success_threshold,
        );
        Self::apply_env_parse(
            "PM_CB_FAILURE_WINDOW_SECS",
            &mut self.circuit_breaker.failure_window_secs,
        );

        // Retry
        Self::apply_env_parse("PM_RETRY_MAX_ATTEMPTS", &mut self.retry.max_attempts);
        Self::apply_env_parse(
            "PM_RETRY_INITIAL_DELAY_MS",
            &mut self.retry.initial_delay_ms,
        );
        Self::apply_env_parse("PM_RETRY_MAX_DELAY_SECS", &mut self.retry.max_delay_secs);
        Self::apply_env_parse(
            "PM_RETRY_BACKOFF_MULTIPLIER",
            &mut self.retry.backoff_multiplier,
        );
        Self::apply_env_bool("PM_RETRY_JITTER", &mut self.retry.jitter);

        // Handler
        Self::apply_env_parse("PM_HANDLER_TIMEOUT_SECS", &mut self.handler.timeout_secs);

        // Validation
        Self::apply_env_parse(
            "PM_VALIDATION_MAX_TITLE_LENGTH",
            &mut self.validation.max_title_length,
        );
        Self::apply_env_parse(
            "PM_VALIDATION_MAX_DESCRIPTION_LENGTH",
            &mut self.validation.max_description_length,
        );
        Self::apply_env_parse(
            "PM_VALIDATION_MAX_STORY_POINTS",
            &mut self.validation.max_story_points,
        );
        Self::apply_env_parse(
            "PM_VALIDATION_MAX_ERROR_MESSAGE_LENGTH",
            &mut self.validation.max_error_message_length,
        );
    }

    /// Helper: Apply environment variable override for String values
    fn apply_env_string(var_name: &str, target: &mut String) {
        if let Ok(val) = std::env::var(var_name) {
            *target = val;
        }
    }

    /// Helper: Apply environment variable override for bool values (accepts "true"/"1")
    fn apply_env_bool(var_name: &str, target: &mut bool) {
        if let Ok(val) = std::env::var(var_name) {
            *target = val == "true" || val == "1";
        }
    }

    /// Helper: Apply environment variable override for parseable values
    fn apply_env_parse<T: std::str::FromStr>(var_name: &str, target: &mut T) {
        if let Ok(val) = std::env::var(var_name)
            && let Ok(parsed) = val.parse()
        {
            *target = parsed;
        }
    }

    /// Helper: Apply environment variable override for Option<String> values
    fn apply_env_option_string(var_name: &str, target: &mut Option<String>) {
        if let Ok(val) = std::env::var(var_name) {
            *target = Some(val);
        }
    }
}
