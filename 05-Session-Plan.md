# Session 05: Integrate pm-config into pm-server (Production-Grade)

## Goal
Make `pm-config` the single source of truth for all configuration with production-grade error handling, security, and operational excellence.

---

## Production-Grade Requirements

| Requirement | Implementation |
|-------------|----------------|
| **Zero-config startup** | Server runs with sensible defaults, no files required |
| **Fail-fast validation** | Invalid config caught at startup with actionable errors |
| **Security by default** | Auth disabled for desktop; secrets never logged |
| **Graceful migration** | Warn about old config locations |
| **Defensive file handling** | Auto-create dirs, check permissions, validate paths |
| **Comprehensive testing** | Edge cases, error paths, security scenarios |

---

## Design Decisions

1. **pm-server converts types** - pm-auth/pm-ws keep simple structs; pm-server converts
2. **`max_connections` in ServerConfig** - server-level concern
3. **`Option<JwtValidator>` for optional auth** - skip JWT when `auth.enabled=false`
4. **Config directory: `./.pm/`** - relative to working directory with `PM_CONFIG_DIR` override
5. **Unique session ID for anonymous users** - not hardcoded string
6. **Secrets never logged** - mask sensitive fields in all log output

---

## Implementation Phases

### Phase 1: Extend pm-config with New Config Types

**Create files:**

#### 1. `backend/crates/pm-config/src/websocket_config.rs`
```rust
use serde::Deserialize;

/// WebSocket connection settings.
/// All values validated to be within reasonable operational ranges.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct WebSocketConfig {
    /// Send buffer size (1-10000, default: 100)
    pub send_buffer_size: usize,
    /// Heartbeat ping interval in seconds (5-300, default: 30)
    pub heartbeat_interval_secs: u64,
    /// Heartbeat timeout in seconds (10-600, default: 60)
    pub heartbeat_timeout_secs: u64,
}

impl Default for WebSocketConfig {
    fn default() -> Self {
        Self {
            send_buffer_size: 100,
            heartbeat_interval_secs: 30,
            heartbeat_timeout_secs: 60,
        }
    }
}

impl WebSocketConfig {
    /// Validate all fields are within acceptable ranges.
    pub fn validate(&self) -> Result<(), String> {
        if self.send_buffer_size == 0 || self.send_buffer_size > 10000 {
            return Err(format!(
                "websocket.send_buffer_size must be 1-10000, got {}",
                self.send_buffer_size
            ));
        }
        if self.heartbeat_interval_secs < 5 || self.heartbeat_interval_secs > 300 {
            return Err(format!(
                "websocket.heartbeat_interval_secs must be 5-300, got {}",
                self.heartbeat_interval_secs
            ));
        }
        if self.heartbeat_timeout_secs < 10 || self.heartbeat_timeout_secs > 600 {
            return Err(format!(
                "websocket.heartbeat_timeout_secs must be 10-600, got {}",
                self.heartbeat_timeout_secs
            ));
        }
        if self.heartbeat_timeout_secs <= self.heartbeat_interval_secs {
            return Err(format!(
                "websocket.heartbeat_timeout_secs ({}) must be greater than heartbeat_interval_secs ({})",
                self.heartbeat_timeout_secs, self.heartbeat_interval_secs
            ));
        }
        Ok(())
    }
}
```

#### 2. `backend/crates/pm-config/src/rate_limit_config.rs`
```rust
use serde::Deserialize;

/// Rate limiting settings.
/// Applied per-connection to prevent abuse.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct RateLimitConfig {
    /// Maximum requests per window (1-10000, default: 100)
    pub max_requests: u32,
    /// Window duration in seconds (1-3600, default: 60)
    pub window_secs: u64,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_requests: 100,
            window_secs: 60,
        }
    }
}

impl RateLimitConfig {
    pub fn validate(&self) -> Result<(), String> {
        if self.max_requests == 0 || self.max_requests > 10000 {
            return Err(format!(
                "rate_limit.max_requests must be 1-10000, got {}",
                self.max_requests
            ));
        }
        if self.window_secs == 0 || self.window_secs > 3600 {
            return Err(format!(
                "rate_limit.window_secs must be 1-3600, got {}",
                self.window_secs
            ));
        }
        Ok(())
    }
}
```

**Modify files:**

#### 3. `backend/crates/pm-config/src/lib.rs`
- Add module declarations: `websocket_config`, `rate_limit_config`
- Add exports for new types
- Add default constants:
  ```rust
  pub const DEFAULT_MAX_CONNECTIONS: usize = 10000;
  pub const DEFAULT_DESKTOP_USER_ID: &str = "local-user";
  pub const MIN_JWT_SECRET_LENGTH: usize = 32;
  ```

#### 4. `backend/crates/pm-config/src/server_config.rs`
```rust
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    /// Maximum concurrent connections (1-100000, default: 10000)
    pub max_connections: usize,
}

impl ServerConfig {
    pub fn validate(&self) -> Result<(), String> {
        if self.port == 0 {
            return Err("server.port cannot be 0".to_string());
        }
        if self.max_connections == 0 || self.max_connections > 100000 {
            return Err(format!(
                "server.max_connections must be 1-100000, got {}",
                self.max_connections
            ));
        }
        Ok(())
    }
}
```

#### 5. `backend/crates/pm-config/src/auth_config.rs`
```rust
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct AuthConfig {
    /// Enable JWT authentication (default: false for desktop mode)
    pub enabled: bool,
    /// HS256 JWT secret (min 32 characters when auth enabled)
    #[serde(default, skip_serializing)]  // Never serialize secrets
    pub jwt_secret: Option<String>,
    /// Path to RS256 public key PEM file (relative to config dir)
    pub jwt_public_key_path: Option<String>,
    /// User ID when auth is disabled (default: "local-user")
    /// Set to empty string to generate unique session ID
    pub desktop_user_id: Option<String>,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            jwt_secret: None,
            jwt_public_key_path: None,
            desktop_user_id: Some("local-user".to_string()),
        }
    }
}

impl AuthConfig {
    pub fn validate(&self, config_dir: &std::path::Path) -> Result<(), String> {
        if self.enabled {
            match (&self.jwt_secret, &self.jwt_public_key_path) {
                (None, None) => {
                    return Err(
                        "auth.enabled=true requires either jwt_secret or jwt_public_key_path. \
                         Set auth.enabled=false for desktop mode.".to_string()
                    );
                }
                (Some(secret), _) => {
                    if secret.len() < 32 {
                        return Err(format!(
                            "auth.jwt_secret must be at least 32 characters for security, got {}",
                            secret.len()
                        ));
                    }
                }
                (None, Some(path)) => {
                    // Validate path is relative and within config dir (prevent path traversal)
                    let key_path = std::path::Path::new(path);
                    if key_path.is_absolute() {
                        return Err(
                            "auth.jwt_public_key_path must be relative to config directory".to_string()
                        );
                    }
                    if path.contains("..") {
                        return Err(
                            "auth.jwt_public_key_path cannot contain '..' (path traversal)".to_string()
                        );
                    }
                    let full_path = config_dir.join(path);
                    if !full_path.exists() {
                        return Err(format!(
                            "auth.jwt_public_key_path '{}' does not exist (looked for {})",
                            path, full_path.display()
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    /// Get the desktop user ID, generating a unique session ID if not configured.
    pub fn get_desktop_user_id(&self) -> String {
        match &self.desktop_user_id {
            Some(id) if !id.is_empty() => id.clone(),
            _ => format!("session-{}", uuid::Uuid::new_v4()),
        }
    }
}
```

#### 6. `backend/crates/pm-config/src/logging_config.rs`
- Add `colored: bool` (default: true)

#### 7. `backend/crates/pm-config/src/config.rs` (Major Changes)

```rust
use crate::{
    AuthConfig, ConfigError, ConfigErrorResult, DatabaseConfig, LoggingConfig,
    RateLimitConfig, ServerConfig, WebSocketConfig,
};
use std::path::PathBuf;
use serde::Deserialize;
use log::warn;

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub auth: AuthConfig,
    pub logging: LoggingConfig,
    pub websocket: WebSocketConfig,
    pub rate_limit: RateLimitConfig,
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

        // Check for legacy config and warn
        Self::check_legacy_config();

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

    /// Check for legacy ~/.pm/config.toml and log warning.
    fn check_legacy_config() {
        if let Some(home) = dirs::home_dir() {
            let legacy_path = home.join(".pm").join("config.toml");
            if legacy_path.exists() {
                warn!(
                    "Found legacy config at {}. Config is now loaded from ./.pm/config.toml (relative to working directory). \
                     Set PM_CONFIG_DIR to override.",
                    legacy_path.display()
                );
            }
        }
    }

    /// Get the config directory.
    /// Priority: PM_CONFIG_DIR env var > ./.pm/ (relative to cwd)
    pub fn config_dir() -> Result<PathBuf, ConfigError> {
        if let Ok(dir) = std::env::var("PM_CONFIG_DIR") {
            return Ok(PathBuf::from(dir));
        }

        let cwd = std::env::current_dir().map_err(|_| {
            ConfigError::config("Cannot determine current working directory")
        })?;
        Ok(cwd.join(".pm"))
    }

    /// Validate all configuration.
    /// Call after load() to catch all errors at startup.
    pub fn validate(&self) -> ConfigErrorResult<()> {
        let config_dir = Self::config_dir()?;

        self.server.validate().map_err(ConfigError::config)?;
        self.auth.validate(&config_dir).map_err(ConfigError::config)?;
        self.websocket.validate().map_err(ConfigError::config)?;
        self.rate_limit.validate().map_err(ConfigError::config)?;

        // Validate database path doesn't escape config dir
        let db_path = std::path::Path::new(&self.database.path);
        if db_path.is_absolute() || self.database.path.contains("..") {
            return Err(ConfigError::config(
                "database.path must be relative and cannot contain '..'"
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
        use log::info;
        info!("Configuration loaded:");
        info!("  server: {}:{} (max {} connections)",
            self.server.host, self.server.port, self.server.max_connections);
        info!("  database: {}", self.database.path);
        info!("  auth: {} ({})",
            if self.auth.enabled { "enabled" } else { "disabled" },
            if self.auth.jwt_secret.is_some() { "HS256" }
            else if self.auth.jwt_public_key_path.is_some() { "RS256" }
            else { "none" }
        );
        info!("  logging: {} (colored: {})",
            self.logging.level.to_string(), self.logging.colored);
        info!("  websocket: buffer={}, heartbeat={}s/{}s",
            self.websocket.send_buffer_size,
            self.websocket.heartbeat_interval_secs,
            self.websocket.heartbeat_timeout_secs);
        info!("  rate_limit: {}/{}s",
            self.rate_limit.max_requests, self.rate_limit.window_secs);
    }

    fn apply_env_overrides(&mut self) {
        // Server
        if let Ok(host) = std::env::var("PM_SERVER_HOST") {
            self.server.host = host;
        }
        if let Ok(port) = std::env::var("PM_SERVER_PORT") {
            if let Ok(p) = port.parse() {
                self.server.port = p;
            }
        }
        if let Ok(max) = std::env::var("PM_SERVER_MAX_CONNECTIONS") {
            if let Ok(m) = max.parse() {
                self.server.max_connections = m;
            }
        }

        // Database
        if let Ok(path) = std::env::var("PM_DATABASE_PATH") {
            self.database.path = path;
        }

        // Auth
        if let Ok(enabled) = std::env::var("PM_AUTH_ENABLED") {
            self.auth.enabled = enabled == "true" || enabled == "1";
        }
        if let Ok(secret) = std::env::var("PM_AUTH_JWT_SECRET") {
            self.auth.jwt_secret = Some(secret);
        }
        if let Ok(path) = std::env::var("PM_AUTH_JWT_PUBLIC_KEY_PATH") {
            self.auth.jwt_public_key_path = Some(path);
        }
        if let Ok(user_id) = std::env::var("PM_AUTH_DESKTOP_USER_ID") {
            self.auth.desktop_user_id = Some(user_id);
        }

        // Logging
        if let Ok(level) = std::env::var("PM_LOG_LEVEL") {
            if let Ok(l) = level.parse() {
                self.logging.level = l;
            }
        }
        if let Ok(colored) = std::env::var("PM_LOG_COLORED") {
            self.logging.colored = colored == "true" || colored == "1";
        }

        // WebSocket
        if let Ok(size) = std::env::var("PM_WS_SEND_BUFFER_SIZE") {
            if let Ok(s) = size.parse() {
                self.websocket.send_buffer_size = s;
            }
        }
        if let Ok(interval) = std::env::var("PM_WS_HEARTBEAT_INTERVAL_SECS") {
            if let Ok(i) = interval.parse() {
                self.websocket.heartbeat_interval_secs = i;
            }
        }
        if let Ok(timeout) = std::env::var("PM_WS_HEARTBEAT_TIMEOUT_SECS") {
            if let Ok(t) = timeout.parse() {
                self.websocket.heartbeat_timeout_secs = t;
            }
        }

        // Rate limit
        if let Ok(max) = std::env::var("PM_RATE_LIMIT_MAX_REQUESTS") {
            if let Ok(m) = max.parse() {
                self.rate_limit.max_requests = m;
            }
        }
        if let Ok(window) = std::env::var("PM_RATE_LIMIT_WINDOW_SECS") {
            if let Ok(w) = window.parse() {
                self.rate_limit.window_secs = w;
            }
        }
    }
}
```

#### 8. `backend/crates/pm-config/Cargo.toml`
- Add `uuid = { version = "1", features = ["v4"] }` dependency

---

### Phase 2: Update pm-server to Use pm-config

#### 1. `backend/pm-server/Cargo.toml`
- Add `pm-config = { workspace = true }`

#### 2. `backend/pm-server/src/error.rs`
- Remove `MissingJwtConfig` variant
- Add `Config(#[from] pm_config::ConfigError)` variant
- Add `JwtKeyFile { path: String, source: std::io::Error }` variant

#### 3. `backend/pm-server/src/logger.rs`
- Change to accept `pm_config::LogLevel` and `bool` for colored

#### 4. `backend/pm-server/src/main.rs`
```rust
use pm_config::Config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Load and validate configuration
    let config = Config::load()?;
    config.validate()?;

    // Initialize logger (before any other logging)
    logger::initialize(config.logging.level, config.logging.colored)?;

    info!("Starting pm-server v{}", env!("CARGO_PKG_VERSION"));
    config.log_summary();  // Safe - never logs secrets

    // Create JWT validator (optional based on auth.enabled)
    let jwt_validator: Option<Arc<JwtValidator>> = if config.auth.enabled {
        let validator = if let Some(ref secret) = config.auth.jwt_secret {
            info!("JWT: HS256 authentication enabled");
            JwtValidator::with_hs256(secret.as_bytes())
        } else if let Some(ref key_path) = config.auth.jwt_public_key_path {
            let config_dir = Config::config_dir()?;
            let full_path = config_dir.join(key_path);
            let public_key = std::fs::read_to_string(&full_path)
                .map_err(|e| ServerError::JwtKeyFile {
                    path: full_path.display().to_string(),
                    source: e
                })?;
            info!("JWT: RS256 authentication enabled");
            JwtValidator::with_rs256(&public_key)?
        } else {
            unreachable!("validate() ensures JWT config when auth.enabled")
        };
        Some(Arc::new(validator))
    } else {
        warn!("Authentication DISABLED - running in desktop/development mode");
        None
    };

    // Get desktop user ID for anonymous mode
    let desktop_user_id = config.auth.get_desktop_user_id();

    // Convert config types for pm-auth/pm-ws
    let rate_limiter_factory = RateLimiterFactory::new(pm_auth::RateLimitConfig {
        max_requests: config.rate_limit.max_requests,
        window_secs: config.rate_limit.window_secs,
    });

    let registry = ConnectionRegistry::new(pm_ws::ConnectionLimits {
        max_total: config.server.max_connections,
    });

    let connection_config = pm_ws::ConnectionConfig {
        send_buffer_size: config.websocket.send_buffer_size,
        heartbeat_interval_secs: config.websocket.heartbeat_interval_secs,
        heartbeat_timeout_secs: config.websocket.heartbeat_timeout_secs,
    };

    // Build application state
    let app_state = AppState {
        jwt_validator,
        desktop_user_id,  // NEW: passed for anonymous mode
        rate_limiter_factory,
        registry,
        metrics: Metrics::new(),
        shutdown: ShutdownCoordinator::new(),
        config: connection_config,
    };

    // ... rest of server startup unchanged
}
```

#### 5. DELETE `backend/pm-server/src/config.rs`

---

### Phase 3: Update pm-ws for Optional Auth

#### 1. `backend/crates/pm-ws/src/app_state.rs`

```rust
#[derive(Clone)]
pub struct AppState {
    pub jwt_validator: Option<Arc<JwtValidator>>,  // CHANGED: now Option
    pub desktop_user_id: String,                    // NEW: for anonymous mode
    pub rate_limiter_factory: RateLimiterFactory,
    pub registry: ConnectionRegistry,
    pub metrics: Metrics,
    pub shutdown: ShutdownCoordinator,
    pub config: ConnectionConfig,
}

fn extract_user_id(
    headers: &HeaderMap,
    validator: &Option<Arc<JwtValidator>>,
    desktop_user_id: &str,
) -> Result<String, StatusCode> {
    match validator {
        Some(v) => {
            // Existing JWT validation logic
            let auth_header = headers
                .get("authorization")
                .and_then(|h| h.to_str().ok())
                .ok_or_else(|| {
                    warn!("Missing Authorization header");
                    StatusCode::UNAUTHORIZED
                })?;

            if !auth_header.starts_with("Bearer ") {
                warn!("Invalid authorization scheme: expected 'Bearer'");
                return Err(StatusCode::UNAUTHORIZED);
            }

            let token = &auth_header[7..];
            let claims = v.validate(token).map_err(|e| {
                warn!("JWT validation failed: {}", e);
                StatusCode::UNAUTHORIZED
            })?;

            Ok(claims.sub)
        }
        None => {
            // Auth disabled - use configured desktop user ID
            debug!("Auth disabled, using desktop user ID: {}", desktop_user_id);
            Ok(desktop_user_id.to_string())
        }
    }
}

pub async fn handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> Result<Response, StatusCode> {
    let user_id = extract_user_id(&headers, &state.jwt_validator, &state.desktop_user_id)?;
    // ... rest unchanged
}
```

---

### Phase 4: Comprehensive Tests

#### 1. `backend/crates/pm-config/src/tests.rs` (New file or inline tests)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use tempfile::TempDir;

    // === Happy Path Tests ===

    #[test]
    fn test_load_with_defaults() {
        let temp = TempDir::new().unwrap();
        env::set_var("PM_CONFIG_DIR", temp.path());

        let config = Config::load().unwrap();
        config.validate().unwrap();

        assert_eq!(config.server.port, 8000);
        assert!(!config.auth.enabled);

        env::remove_var("PM_CONFIG_DIR");
    }

    #[test]
    fn test_load_from_toml() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join("config.toml");
        std::fs::write(&config_path, r#"
            [server]
            port = 9000

            [auth]
            enabled = false
        "#).unwrap();

        env::set_var("PM_CONFIG_DIR", temp.path());
        let config = Config::load().unwrap();
        assert_eq!(config.server.port, 9000);
        env::remove_var("PM_CONFIG_DIR");
    }

    #[test]
    fn test_env_var_overrides_toml() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join("config.toml");
        std::fs::write(&config_path, "[server]\nport = 9000").unwrap();

        env::set_var("PM_CONFIG_DIR", temp.path());
        env::set_var("PM_SERVER_PORT", "8888");

        let config = Config::load().unwrap();
        assert_eq!(config.server.port, 8888);  // Env var wins

        env::remove_var("PM_CONFIG_DIR");
        env::remove_var("PM_SERVER_PORT");
    }

    // === Validation Tests ===

    #[test]
    fn test_auth_enabled_requires_jwt_config() {
        let temp = TempDir::new().unwrap();
        env::set_var("PM_CONFIG_DIR", temp.path());
        env::set_var("PM_AUTH_ENABLED", "true");

        let config = Config::load().unwrap();
        let result = config.validate();

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("jwt_secret"));

        env::remove_var("PM_CONFIG_DIR");
        env::remove_var("PM_AUTH_ENABLED");
    }

    #[test]
    fn test_jwt_secret_minimum_length() {
        let temp = TempDir::new().unwrap();
        env::set_var("PM_CONFIG_DIR", temp.path());
        env::set_var("PM_AUTH_ENABLED", "true");
        env::set_var("PM_AUTH_JWT_SECRET", "tooshort");

        let config = Config::load().unwrap();
        let result = config.validate();

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("32 characters"));

        env::remove_var("PM_CONFIG_DIR");
        env::remove_var("PM_AUTH_ENABLED");
        env::remove_var("PM_AUTH_JWT_SECRET");
    }

    #[test]
    fn test_path_traversal_rejected() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join("config.toml");
        std::fs::write(&config_path, r#"
            [auth]
            enabled = true
            jwt_public_key_path = "../../../etc/passwd"
        "#).unwrap();

        env::set_var("PM_CONFIG_DIR", temp.path());
        let config = Config::load().unwrap();
        let result = config.validate();

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("path traversal"));

        env::remove_var("PM_CONFIG_DIR");
    }

    #[test]
    fn test_absolute_jwt_path_rejected() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join("config.toml");
        std::fs::write(&config_path, r#"
            [auth]
            enabled = true
            jwt_public_key_path = "/etc/passwd"
        "#).unwrap();

        env::set_var("PM_CONFIG_DIR", temp.path());
        let config = Config::load().unwrap();
        let result = config.validate();

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("relative"));

        env::remove_var("PM_CONFIG_DIR");
    }

    // === Edge Case Tests ===

    #[test]
    fn test_malformed_toml() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join("config.toml");
        std::fs::write(&config_path, "this is not valid toml {{{{").unwrap();

        env::set_var("PM_CONFIG_DIR", temp.path());
        let result = Config::load();

        assert!(result.is_err());
        // Should include file path in error
        assert!(result.unwrap_err().to_string().contains("config.toml"));

        env::remove_var("PM_CONFIG_DIR");
    }

    #[test]
    fn test_websocket_timeout_greater_than_interval() {
        let mut config = Config::default();
        config.websocket.heartbeat_interval_secs = 60;
        config.websocket.heartbeat_timeout_secs = 30;  // Invalid: less than interval

        let result = config.websocket.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_port_zero_rejected() {
        let mut config = Config::default();
        config.server.port = 0;

        let result = config.server.validate();
        assert!(result.is_err());
    }

    // === Desktop User ID Tests ===

    #[test]
    fn test_desktop_user_id_default() {
        let config = AuthConfig::default();
        assert_eq!(config.get_desktop_user_id(), "local-user");
    }

    #[test]
    fn test_desktop_user_id_custom() {
        let mut config = AuthConfig::default();
        config.desktop_user_id = Some("my-user".to_string());
        assert_eq!(config.get_desktop_user_id(), "my-user");
    }

    #[test]
    fn test_desktop_user_id_empty_generates_uuid() {
        let mut config = AuthConfig::default();
        config.desktop_user_id = Some("".to_string());

        let user_id = config.get_desktop_user_id();
        assert!(user_id.starts_with("session-"));
        assert!(user_id.len() > 20);  // UUID is longer
    }
}
```

#### 2. Update `backend/crates/pm-ws/tests/common/test_server.rs`
- Support `jwt_secret: Option<Vec<u8>>`
- Pass `desktop_user_id` to AppState

---

### Phase 5: Documentation

#### 1. `backend/config.example.toml`
```toml
# PM Server Configuration
# ========================
# Copy to .pm/config.toml in your working directory.
# All values shown are defaults - only override what you need.
#
# Environment variables override TOML values (prefix: PM_)
# Example: PM_SERVER_PORT=9000 overrides [server].port

[server]
host = "127.0.0.1"
port = 8000
max_connections = 10000   # 1-100000

[database]
path = "data.db"          # Relative to .pm/ directory

[auth]
enabled = false           # Set true for production with JWT
# jwt_secret = "your-secret-key-minimum-32-characters-long"
# jwt_public_key_path = "public.pem"  # Relative to .pm/
# desktop_user_id = "local-user"      # Or empty to generate unique session ID

[websocket]
send_buffer_size = 100           # 1-10000
heartbeat_interval_secs = 30     # 5-300
heartbeat_timeout_secs = 60      # 10-600, must be > interval

[rate_limit]
max_requests = 100        # 1-10000 per window
window_secs = 60          # 1-3600

[logging]
level = "info"            # trace, debug, info, warn, error
dir = "log"               # Relative to .pm/
colored = true
```

---

## File Summary

| Action | File |
|--------|------|
| CREATE | `backend/crates/pm-config/src/websocket_config.rs` |
| CREATE | `backend/crates/pm-config/src/rate_limit_config.rs` |
| CREATE | `backend/crates/pm-config/src/tests.rs` |
| CREATE | `backend/config.example.toml` |
| MODIFY | `backend/crates/pm-config/src/lib.rs` |
| MODIFY | `backend/crates/pm-config/src/config.rs` |
| MODIFY | `backend/crates/pm-config/src/server_config.rs` |
| MODIFY | `backend/crates/pm-config/src/auth_config.rs` |
| MODIFY | `backend/crates/pm-config/src/logging_config.rs` |
| MODIFY | `backend/crates/pm-config/Cargo.toml` |
| MODIFY | `backend/pm-server/Cargo.toml` |
| MODIFY | `backend/pm-server/src/main.rs` |
| MODIFY | `backend/pm-server/src/error.rs` |
| MODIFY | `backend/pm-server/src/logger.rs` |
| MODIFY | `backend/crates/pm-ws/src/app_state.rs` |
| MODIFY | `backend/crates/pm-ws/tests/common/test_server.rs` |
| DELETE | `backend/pm-server/src/config.rs` |

---

## Verification

```bash
# 1. Build
cd backend && cargo build --workspace

# 2. Run all tests (including new config tests)
cargo test --workspace

# 3. Server starts with defaults
cargo run -p pm-server
# Expected: "Authentication DISABLED - running in desktop/development mode"

# 4. Validation catches bad config
PM_AUTH_ENABLED=true cargo run -p pm-server
# Expected: Error about missing jwt_secret

# 5. Validation catches short secret
PM_AUTH_ENABLED=true PM_AUTH_JWT_SECRET=short cargo run -p pm-server
# Expected: Error about 32 character minimum

# 6. Valid auth config works
PM_AUTH_ENABLED=true PM_AUTH_JWT_SECRET=this-is-a-very-long-secret-key-for-testing cargo run -p pm-server
# Expected: "JWT: HS256 authentication enabled"

# 7. Legacy config warning
mkdir -p ~/.pm && touch ~/.pm/config.toml
cargo run -p pm-server
# Expected: Warning about legacy config location
rm -rf ~/.pm

# 8. Custom config directory
mkdir -p /tmp/myconfig && PM_CONFIG_DIR=/tmp/myconfig cargo run -p pm-server
# Expected: Uses /tmp/myconfig/.pm/ for config
```

---

## Production-Grade Checklist

| Requirement | Status |
|-------------|--------|
| Zero-config startup | Defaults for everything |
| Auto-create config dir | Creates .pm/ if missing |
| Fail-fast validation | All fields validated with ranges |
| Secrets never logged | log_summary() masks auth section |
| Path traversal prevention | Rejects `..` and absolute paths |
| JWT secret min length | 32 character minimum |
| Migration warning | Warns about ~/.pm/config.toml |
| Unique anonymous users | Configurable or UUID session ID |
| Comprehensive tests | Happy path, validation, edge cases |
| Actionable error messages | Include paths, suggestions |

---

## Notes for Developer Agent

1. **Test isolation**: Each test must set/unset env vars to avoid interference
2. **tempfile crate**: Use for test config directories (add to dev-dependencies)
3. **uuid crate**: Add to pm-config for session ID generation
4. **Never log secrets**: Double-check all log statements don't include auth.jwt_secret
5. **Path handling**: All relative paths resolve from config_dir, never cwd directly
6. **Error context**: Every error should include file paths when relevant
