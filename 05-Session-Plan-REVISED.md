# Session 05: Integrate pm-config into pm-server (REVISED - Production Quality)

## ✅ STATUS: COMPLETE (2026-01-17)

**Estimated Tokens**: 40k
**Actual Tokens**: ~108k (2.7x due to quality improvements)
**All phases completed successfully with production-grade quality**

---

## Overview

Make `pm-config` the single source of truth with proper error handling, constants, and validation.

**Key Quality Standards:**
- All constants defined in `lib.rs` (single source of truth)
- All validation uses `ConfigErrorResult<()>` with `ConfigError::config()`
- Constants used consistently in defaults AND validation
- No magic numbers or strings
- `#[track_caller]` for all validation errors

---

## Phase 1: Extend pm-config

### Chunk 1: Create `rate_limit_config.rs`

**What:** Rate limiting settings with proper constants and validation.

**File:** `backend/crates/pm-config/src/rate_limit_config.rs`

```rust
use serde::Deserialize;
use crate::{ConfigError, ConfigErrorResult};

// Rate limit constraints
pub const MIN_RATE_LIMIT_REQUESTS: u32 = 1;
pub const MAX_RATE_LIMIT_REQUESTS: u32 = 10000;
pub const DEFAULT_RATE_LIMIT_REQUESTS: u32 = 100;

pub const MIN_RATE_LIMIT_WINDOW_SECS: u64 = 1;
pub const MAX_RATE_LIMIT_WINDOW_SECS: u64 = 3600;
pub const DEFAULT_RATE_LIMIT_WINDOW_SECS: u64 = 60;

/// Rate limiting settings.
/// Applied per-connection to prevent abuse.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct RateLimitConfig {
    /// Maximum requests per window
    pub max_requests: u32,
    /// Window duration in seconds
    pub window_secs: u64,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_requests: DEFAULT_RATE_LIMIT_REQUESTS,
            window_secs: DEFAULT_RATE_LIMIT_WINDOW_SECS,
        }
    }
}

impl RateLimitConfig {
    pub fn validate(&self) -> ConfigErrorResult<()> {
        if self.max_requests < MIN_RATE_LIMIT_REQUESTS
            || self.max_requests > MAX_RATE_LIMIT_REQUESTS
        {
            return Err(ConfigError::config(format!(
                "rate_limit.max_requests must be {}-{}, got {}",
                MIN_RATE_LIMIT_REQUESTS, MAX_RATE_LIMIT_REQUESTS, self.max_requests
            )));
        }

        if self.window_secs < MIN_RATE_LIMIT_WINDOW_SECS
            || self.window_secs > MAX_RATE_LIMIT_WINDOW_SECS
        {
            return Err(ConfigError::config(format!(
                "rate_limit.window_secs must be {}-{}, got {}",
                MIN_RATE_LIMIT_WINDOW_SECS, MAX_RATE_LIMIT_WINDOW_SECS,
                self.window_secs
            )));
        }

        Ok(())
    }
}
```

---

### Chunk 2: Update `lib.rs` - Add module declarations and exports

**What:** Declare new modules and export new types.

**File:** `backend/crates/pm-config/src/lib.rs`

**Changes:**

1. Add module declarations after line 8:
```rust
mod rate_limit_config;
```

2. Add exports after line 16:
```rust
pub use rate_limit_config::RateLimitConfig;
pub use websocket_config::WebSocketConfig;
```

3. Add new constants after line 24:
```rust
// Server constraints
pub const MIN_MAX_CONNECTIONS: usize = 1;
pub const MAX_MAX_CONNECTIONS: usize = 100000;
pub const DEFAULT_MAX_CONNECTIONS: usize = 10000;

// Auth constraints
pub const DEFAULT_DESKTOP_USER_ID: &str = "local-user";
pub const MIN_JWT_SECRET_LENGTH: usize = 32;

// Logging
pub const DEFAULT_LOG_COLORED: bool = true;
```

---

### Chunk 3: Extend `server_config.rs` - Add max_connections

**What:** Add connection limit with proper validation.

**File:** `backend/crates/pm-config/src/server_config.rs`

**Full replacement:**

```rust
use crate::{ConfigError, ConfigErrorResult, DEFAULT_HOST, DEFAULT_PORT, DEFAULT_MAX_CONNECTIONS};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    /// Maximum concurrent connections
    pub max_connections: usize,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: String::from(DEFAULT_HOST),
            port: DEFAULT_PORT,
            max_connections: DEFAULT_MAX_CONNECTIONS,
        }
    }
}

impl ServerConfig {
    pub fn validate(&self) -> ConfigErrorResult<()> {
        if self.port == 0 {
            return Err(ConfigError::config("server.port cannot be 0"));
        }

        if self.max_connections < crate::MIN_MAX_CONNECTIONS
            || self.max_connections > crate::MAX_MAX_CONNECTIONS
        {
            return Err(ConfigError::config(format!(
                "server.max_connections must be {}-{}, got {}",
                crate::MIN_MAX_CONNECTIONS, crate::MAX_MAX_CONNECTIONS,
                self.max_connections
            )));
        }

        Ok(())
    }
}
```

---

### Chunk 4: Rewrite `auth_config.rs` - JWT support and validation

**What:** Add JWT fields, desktop user ID, and production-grade validation.

**File:** `backend/crates/pm-config/src/auth_config.rs`

**Full replacement:**

```rust
use crate::{ConfigError, ConfigErrorResult, DEFAULT_AUTH_ENABLED, DEFAULT_DESKTOP_USER_ID, MIN_JWT_SECRET_LENGTH};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct AuthConfig {
    /// Enable JWT authentication (default: false for desktop mode)
    pub enabled: bool,

    /// HS256 JWT secret (min 32 characters when auth enabled)
    #[serde(default, skip_serializing)]
    pub jwt_secret: Option<String>,

    /// Path to RS256 public key PEM file (relative to config dir)
    pub jwt_public_key_path: Option<String>,

    /// User ID when auth is disabled (default: "local-user")
    /// Set to empty string to generate unique session ID per connection
    pub desktop_user_id: Option<String>,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            enabled: DEFAULT_AUTH_ENABLED,
            jwt_secret: None,
            jwt_public_key_path: None,
            desktop_user_id: Some(DEFAULT_DESKTOP_USER_ID.to_string()),
        }
    }
}

impl AuthConfig {
    pub fn validate(&self, config_dir: &std::path::Path) -> ConfigErrorResult<()> {
        if !self.enabled {
            return Ok(());
        }

        // Auth enabled requires JWT config
        match (&self.jwt_secret, &self.jwt_public_key_path) {
            (None, None) => {
                return Err(ConfigError::auth(
                    "auth.enabled=true requires either jwt_secret or jwt_public_key_path. \
                     Set auth.enabled=false for desktop mode."
                ));
            }
            (Some(secret), _) => {
                if secret.len() < MIN_JWT_SECRET_LENGTH {
                    return Err(ConfigError::auth(format!(
                        "auth.jwt_secret must be at least {} characters for security, got {}",
                        MIN_JWT_SECRET_LENGTH, secret.len()
                    )));
                }
            }
            (None, Some(path)) => {
                // Validate path is relative and within config dir (prevent path traversal)
                let key_path = std::path::Path::new(path);
                if key_path.is_absolute() {
                    return Err(ConfigError::auth(
                        "auth.jwt_public_key_path must be relative to config directory"
                    ));
                }
                if path.contains("..") {
                    return Err(ConfigError::auth(
                        "auth.jwt_public_key_path cannot contain '..' (path traversal protection)"
                    ));
                }

                let full_path = config_dir.join(path);
                if !full_path.exists() {
                    return Err(ConfigError::auth(format!(
                        "auth.jwt_public_key_path '{}' does not exist (looked for {})",
                        path, full_path.display()
                    )));
                }
            }
        }

        Ok(())
    }

    /// Get the desktop user ID, generating a unique session ID if configured to do so.
    pub fn get_desktop_user_id(&self) -> String {
        match &self.desktop_user_id {
            Some(id) if !id.is_empty() => id.clone(),
            _ => format!("session-{}", uuid::Uuid::new_v4()),
        }
    }
}
```

---

### Chunk 5: Extend `logging_config.rs` - Add colored field

**What:** Add colored output toggle.

**File:** `backend/crates/pm-config/src/logging_config.rs`

**Full replacement:**

```rust
use crate::{DEFAULT_LOG_DIRECTORY, DEFAULT_LOG_LEVEL, DEFAULT_LOG_COLORED, LogLevel};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct LoggingConfig {
    pub level: LogLevel,
    pub dir: String,
    /// Enable colored output (default: true)
    pub colored: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: LogLevel(DEFAULT_LOG_LEVEL),
            dir: String::from(DEFAULT_LOG_DIRECTORY),
            colored: DEFAULT_LOG_COLORED,
        }
    }
}
```

---

### Chunk 6: Rewrite `config.rs` - Production-grade loading

**What:** Complete rewrite with auto-create dirs, validation, env overrides, legacy warning, config_dir change.

**File:** `backend/crates/pm-config/src/config.rs`

**Full replacement:**

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
                    "Found legacy config at {}. Config is now loaded from ./.pm/config.toml \
                     (relative to working directory). Set PM_CONFIG_DIR to override.",
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

        self.server.validate()?;
        self.auth.validate(&config_dir)?;
        self.websocket.validate()?;
        self.rate_limit.validate()?;

        // Validate database path doesn't escape config dir
        let db_path = std::path::Path::new(&self.database.path);
        if db_path.is_absolute() || self.database.path.contains("..") {
            return Err(ConfigError::database(
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

        let auth_type = if self.auth.jwt_secret.is_some() {
            "HS256"
        } else if self.auth.jwt_public_key_path.is_some() {
            "RS256"
        } else {
            "none"
        };

        info!("  auth: {} ({})",
            if self.auth.enabled { "enabled" } else { "disabled" },
            auth_type
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
        if let Ok(port) = std::env::var("PM_SERVER_PORT")
            && let Ok(p) = port.parse()
        {
            self.server.port = p;
        }
        if let Ok(max) = std::env::var("PM_SERVER_MAX_CONNECTIONS")
            && let Ok(m) = max.parse()
        {
            self.server.max_connections = m;
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
        if let Ok(level) = std::env::var("PM_LOG_LEVEL")
            && let Ok(l) = level.parse()
        {
            self.logging.level = l;
        }
        if let Ok(colored) = std::env::var("PM_LOG_COLORED") {
            self.logging.colored = colored == "true" || colored == "1";
        }

        // WebSocket
        if let Ok(size) = std::env::var("PM_WS_SEND_BUFFER_SIZE")
            && let Ok(s) = size.parse()
        {
            self.websocket.send_buffer_size = s;
        }
        if let Ok(interval) = std::env::var("PM_WS_HEARTBEAT_INTERVAL_SECS")
            && let Ok(i) = interval.parse()
        {
            self.websocket.heartbeat_interval_secs = i;
        }
        if let Ok(timeout) = std::env::var("PM_WS_HEARTBEAT_TIMEOUT_SECS")
            && let Ok(t) = timeout.parse()
        {
            self.websocket.heartbeat_timeout_secs = t;
        }

        // Rate limit
        if let Ok(max) = std::env::var("PM_RATE_LIMIT_MAX_REQUESTS")
            && let Ok(m) = max.parse()
        {
            self.rate_limit.max_requests = m;
        }
        if let Ok(window) = std::env::var("PM_RATE_LIMIT_WINDOW_SECS")
            && let Ok(w) = window.parse()
        {
            self.rate_limit.window_secs = w;
        }
    }
}
```

---

### Chunk 7: Update `Cargo.toml` - Add uuid dependency

**What:** Add uuid crate for session ID generation.

**File:** `backend/crates/pm-config/Cargo.toml`

**Add to [dependencies]:**

```toml
uuid = { version = "1", features = ["v4"] }
```

---

## Phase 2: Update pm-server

### Chunk 8: Update pm-server error types

**What:** Remove old config error, add new ones for pm-config integration.

**File:** `backend/pm-server/src/error.rs`

**Changes:**

1. Remove the `MissingJwtConfig` variant (if it exists)

2. Add new variant:
```rust
    #[error("Config error: {0}")]
    Config(#[from] pm_config::ConfigError),

    #[error("Failed to read JWT key file {path}: {source}")]
    JwtKeyFile {
        path: String,
        #[source]
        source: std::io::Error,
    },
```

---

### Chunk 9: Update logger to accept pm-config types

**What:** Change logger initialization signature.

**File:** `backend/pm-server/src/logger.rs`

**Find the `initialize` function signature and change it to:**

```rust
pub fn initialize(log_level: pm_config::LogLevel, colored: bool) -> Result<(), ServerError>
```

**Update the implementation to use these parameters appropriately.**

---

### Chunk 10: Rewrite pm-server main.rs

**What:** Use pm-config, remove old config, add optional JWT validator, pass desktop_user_id.

**File:** `backend/pm-server/src/main.rs`

**Key changes to make:**

1. Replace config loading:
```rust
// Load and validate configuration
let config = pm_config::Config::load()?;
config.validate()?;
```

2. Initialize logger with new signature:
```rust
logger::initialize(config.logging.level, config.logging.colored)?;
```

3. Log startup:
```rust
info!("Starting pm-server v{}", env!("CARGO_PKG_VERSION"));
config.log_summary();
```

4. Create optional JWT validator:
```rust
let jwt_validator: Option<Arc<JwtValidator>> = if config.auth.enabled {
    let validator = if let Some(ref secret) = config.auth.jwt_secret {
        info!("JWT: HS256 authentication enabled");
        JwtValidator::with_hs256(secret.as_bytes())
    } else if let Some(ref key_path) = config.auth.jwt_public_key_path {
        let config_dir = pm_config::Config::config_dir()?;
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
```

5. Get desktop user ID:
```rust
let desktop_user_id = config.auth.get_desktop_user_id();
```

6. Convert config types for other crates:
```rust
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
```

7. Build AppState with new fields:
```rust
let app_state = AppState {
    jwt_validator,          // Now Option<Arc<JwtValidator>>
    desktop_user_id,        // NEW: String
    rate_limiter_factory,
    registry,
    metrics: Metrics::new(),
    shutdown: ShutdownCoordinator::new(),
    config: connection_config,
};
```

---

### Chunk 11: Add pm-config dependency to pm-server

**What:** Add dependency so pm-server can use pm-config.

**File:** `backend/pm-server/Cargo.toml`

**Add to [dependencies]:**

```toml
pm-config = { workspace = true }
```

---

### Chunk 12: Delete old config file

**What:** Remove the old pm-server config module.

**Action:** Delete `backend/pm-server/src/config.rs`

---

## Phase 3: Update pm-ws

### Chunk 13: Update pm-ws app_state.rs for optional auth

**What:** Make JWT validator optional, add desktop_user_id field, update extraction logic.

**File:** `backend/crates/pm-ws/src/app_state.rs`

**Changes:**

1. Update AppState struct:
```rust
#[derive(Clone)]
pub struct AppState {
    pub jwt_validator: Option<Arc<JwtValidator>>,  // CHANGED: now Option
    pub desktop_user_id: String,                    // NEW
    pub rate_limiter_factory: RateLimiterFactory,
    pub registry: ConnectionRegistry,
    pub metrics: Metrics,
    pub shutdown: ShutdownCoordinator,
    pub config: ConnectionConfig,
}
```

2. Rewrite `extract_user_id` function:
```rust
fn extract_user_id(
    headers: &HeaderMap,
    validator: &Option<Arc<JwtValidator>>,
    desktop_user_id: &str,
) -> Result<String, StatusCode> {
    match validator {
        Some(v) => {
            // JWT validation required
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
```

3. Update handler to pass desktop_user_id:
```rust
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

## Phase 4: Tests

### Chunk 14: Create comprehensive config tests

**What:** Test happy paths, validation, edge cases, security.

**File:** `backend/crates/pm-config/src/config.rs`

**Add at the end of the file:**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use tempfile::TempDir;

    fn setup_test_env(temp_dir: &TempDir) {
        env::set_var("PM_CONFIG_DIR", temp_dir.path());
    }

    fn cleanup_test_env() {
        env::remove_var("PM_CONFIG_DIR");
        env::remove_var("PM_SERVER_PORT");
        env::remove_var("PM_AUTH_ENABLED");
        env::remove_var("PM_AUTH_JWT_SECRET");
        env::remove_var("PM_SERVER_MAX_CONNECTIONS");
    }

    // === Happy Path Tests ===

    #[test]
    fn test_load_with_defaults() {
        let temp = TempDir::new().unwrap();
        setup_test_env(&temp);

        let config = Config::load().unwrap();
        config.validate().unwrap();

        assert_eq!(config.server.port, crate::DEFAULT_PORT);
        assert!(!config.auth.enabled);
        assert_eq!(config.server.max_connections, crate::DEFAULT_MAX_CONNECTIONS);

        cleanup_test_env();
    }

    #[test]
    fn test_load_from_toml() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join("config.toml");
        std::fs::write(&config_path, r#"
            [server]
            port = 9000
            max_connections = 5000

            [auth]
            enabled = false
        "#).unwrap();

        setup_test_env(&temp);
        let config = Config::load().unwrap();

        assert_eq!(config.server.port, 9000);
        assert_eq!(config.server.max_connections, 5000);

        cleanup_test_env();
    }

    #[test]
    fn test_env_var_overrides_toml() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join("config.toml");
        std::fs::write(&config_path, "[server]\nport = 9000").unwrap();

        setup_test_env(&temp);
        env::set_var("PM_SERVER_PORT", "8888");

        let config = Config::load().unwrap();
        assert_eq!(config.server.port, 8888);

        cleanup_test_env();
    }

    // === Validation Tests ===

    #[test]
    fn test_auth_enabled_requires_jwt_config() {
        let temp = TempDir::new().unwrap();
        setup_test_env(&temp);
        env::set_var("PM_AUTH_ENABLED", "true");

        let config = Config::load().unwrap();
        let result = config.validate();

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("jwt_secret") || err_msg.contains("jwt_public_key_path"));

        cleanup_test_env();
    }

    #[test]
    fn test_jwt_secret_minimum_length() {
        let temp = TempDir::new().unwrap();
        setup_test_env(&temp);
        env::set_var("PM_AUTH_ENABLED", "true");
        env::set_var("PM_AUTH_JWT_SECRET", "tooshort");

        let config = Config::load().unwrap();
        let result = config.validate();

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("32 characters"));

        cleanup_test_env();
    }

    #[test]
    fn test_valid_jwt_secret() {
        let temp = TempDir::new().unwrap();
        setup_test_env(&temp);
        env::set_var("PM_AUTH_ENABLED", "true");
        env::set_var("PM_AUTH_JWT_SECRET", "this-is-a-very-long-secret-key-for-testing-purposes");

        let config = Config::load().unwrap();
        let result = config.validate();

        assert!(result.is_ok());

        cleanup_test_env();
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

        setup_test_env(&temp);
        let config = Config::load().unwrap();
        let result = config.validate();

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains(".."));

        cleanup_test_env();
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

        setup_test_env(&temp);
        let config = Config::load().unwrap();
        let result = config.validate();

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("relative"));

        cleanup_test_env();
    }

    #[test]
    fn test_jwt_key_file_must_exist() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join("config.toml");
        std::fs::write(&config_path, r#"
            [auth]
            enabled = true
            jwt_public_key_path = "nonexistent.pem"
        "#).unwrap();

        setup_test_env(&temp);
        let config = Config::load().unwrap();
        let result = config.validate();

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not exist"));

        cleanup_test_env();
    }

    // === Edge Case Tests ===

    #[test]
    fn test_malformed_toml() {
        let temp = TempDir::new().unwrap();
        let config_path = temp.path().join("config.toml");
        std::fs::write(&config_path, "this is not valid toml {{{{").unwrap();

        setup_test_env(&temp);
        let result = Config::load();

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("config.toml"));

        cleanup_test_env();
    }

    #[test]
    fn test_websocket_timeout_greater_than_interval() {
        let temp = TempDir::new().unwrap();
        setup_test_env(&temp);

        let mut config = Config::default();
        config.websocket.heartbeat_interval_secs = 60;
        config.websocket.heartbeat_timeout_secs = 30;

        let result = config.websocket.validate();
        assert!(result.is_err());

        cleanup_test_env();
    }

    #[test]
    fn test_port_zero_rejected() {
        let temp = TempDir::new().unwrap();
        setup_test_env(&temp);

        let mut config = Config::default();
        config.server.port = 0;

        let result = config.server.validate();
        assert!(result.is_err());

        cleanup_test_env();
    }

    #[test]
    fn test_max_connections_out_of_range() {
        let temp = TempDir::new().unwrap();
        setup_test_env(&temp);

        let mut config = Config::default();
        config.server.max_connections = 0;

        let result = config.server.validate();
        assert!(result.is_err());

        config.server.max_connections = 200000;
        let result = config.server.validate();
        assert!(result.is_err());

        cleanup_test_env();
    }

    // === Desktop User ID Tests ===

    #[test]
    fn test_desktop_user_id_default() {
        let config = AuthConfig::default();
        assert_eq!(config.get_desktop_user_id(), crate::DEFAULT_DESKTOP_USER_ID);
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
        assert!(user_id.len() > 20);
    }

    #[test]
    fn test_desktop_user_id_none_generates_uuid() {
        let mut config = AuthConfig::default();
        config.desktop_user_id = None;

        let user_id = config.get_desktop_user_id();
        assert!(user_id.starts_with("session-"));
    }
}
```

**Note:** Add `tempfile = "3"` to `[dev-dependencies]` in `backend/crates/pm-config/Cargo.toml`

---

### Chunk 15: Update pm-ws test helpers

**What:** Update test server builder to support optional JWT and desktop_user_id.

**File:** `backend/crates/pm-ws/tests/common/test_server.rs`

**Changes to TestServerBuilder:**

1. Change jwt_secret field:
```rust
pub struct TestServerBuilder {
    jwt_secret: Option<Vec<u8>>,  // Now Option
    desktop_user_id: String,       // NEW
    // ... other fields
}
```

2. Update default():
```rust
impl Default for TestServerBuilder {
    fn default() -> Self {
        Self {
            jwt_secret: Some(b"test-secret-key-at-least-32-chars-long".to_vec()),
            desktop_user_id: "test-user".to_string(),
            // ... other fields
        }
    }
}
```

3. Add builder method:
```rust
pub fn with_desktop_mode(mut self) -> Self {
    self.jwt_secret = None;
    self.desktop_user_id = "local-user".to_string();
    self
}

pub fn with_desktop_user_id(mut self, user_id: impl Into<String>) -> Self {
    self.desktop_user_id = user_id.into();
    self
}
```

4. Update build() to create Option<Arc<JwtValidator>>:
```rust
let jwt_validator = self.jwt_secret.map(|secret| {
    Arc::new(JwtValidator::with_hs256(&secret))
});

let app_state = AppState {
    jwt_validator,
    desktop_user_id: self.desktop_user_id,
    // ... rest
};
```

---

## Phase 5: Documentation

### Chunk 16: Create config.example.toml

**What:** Example configuration file with all options documented.

**File:** `backend/config.example.toml`

```toml
# PM Server Configuration Example
# ================================
# Copy to .pm/config.toml in your working directory.
# All values shown are defaults - only override what you need.
#
# Environment variables override TOML values (prefix: PM_)
# Example: PM_SERVER_PORT=9000 overrides [server].port

[server]
host = "127.0.0.1"
port = 8000
max_connections = 10000   # Range: 1-100000

[database]
path = "data.db"          # Relative to .pm/ directory

[auth]
enabled = false           # Set true for production with JWT

# When auth.enabled = true, provide ONE of:
# jwt_secret = "your-secret-key-minimum-32-characters-long"
# jwt_public_key_path = "public.pem"  # Relative to .pm/, for RS256

# When auth.enabled = false (desktop mode):
# desktop_user_id = "local-user"      # Or empty string for unique session IDs

[websocket]
send_buffer_size = 100           # Range: 1-10000
heartbeat_interval_secs = 30     # Range: 5-300
heartbeat_timeout_secs = 60      # Range: 10-600, must be > interval

[rate_limit]
max_requests = 100        # Range: 1-10000 per window
window_secs = 60          # Range: 1-3600

[logging]
level = "info"            # trace, debug, info, warn, error
dir = "log"               # Relative to .pm/
colored = true
```

---

## Verification Steps

After implementing all chunks, verify with:

```bash
# 1. Build entire workspace
cd backend && cargo build --workspace

# 2. Run all tests
cargo test --workspace

# 3. Server starts with defaults (desktop mode)
cargo run -p pm-server
# Expected output:
#   "Authentication DISABLED - running in desktop/development mode"
#   Config summary showing all defaults

# 4. Validation catches missing JWT config
PM_AUTH_ENABLED=true cargo run -p pm-server
# Expected: Error about missing jwt_secret or jwt_public_key_path

# 5. Validation catches short secret
PM_AUTH_ENABLED=true PM_AUTH_JWT_SECRET=short cargo run -p pm-server
# Expected: Error about 32 character minimum

# 6. Valid JWT secret works
PM_AUTH_ENABLED=true PM_AUTH_JWT_SECRET=this-is-a-very-long-secret-key-for-testing cargo run -p pm-server
# Expected: "JWT: HS256 authentication enabled"

# 7. Legacy config warning (if ~/.pm/config.toml exists)
mkdir -p ~/.pm && touch ~/.pm/config.toml
cargo run -p pm-server
# Expected: Warning about legacy config location
rm -rf ~/.pm

# 8. Custom config directory
PM_CONFIG_DIR=/tmp/test-config cargo run -p pm-server
# Expected: Creates /tmp/test-config/ and uses it
```

---

## Summary

**Quality improvements over original plan:**
- ✅ All constants defined once, used consistently
- ✅ All validation uses ConfigErrorResult with proper error types
- ✅ No magic numbers or strings
- ✅ Secrets never logged (log_summary uses safe logic)
- ✅ Path traversal protection with clear error messages
- ✅ Comprehensive tests including security scenarios
- ✅ Proper #[track_caller] usage via ConfigError helper methods

**Files created:** 2 (rate_limit_config.rs, config.example.toml)
**Files modified:** 10 (lib.rs, server_config.rs, auth_config.rs, logging_config.rs, config.rs, 2x Cargo.toml, error.rs, logger.rs, main.rs, app_state.rs, test_server.rs)
**Files deleted:** 1 (pm-server/src/config.rs)

---

## 🎉 Actual Completion Summary (2026-01-17)

### What Was Delivered

**Core Implementation:**
- ✅ Production-grade config system with comprehensive validation
- ✅ WebSocket config (send_buffer_size, heartbeat_interval, heartbeat_timeout)
- ✅ Rate limit config (max_requests, window_secs)
- ✅ Server config (host, port 1024-65535, max_connections)
- ✅ Auth config (JWT HS256/RS256, path traversal protection, desktop mode)
- ✅ Optional authentication (`Option<Arc<JwtValidator>>`)
- ✅ Environment variable overrides with clean helper functions
- ✅ Config directory: `./.pm/` relative to cwd (with PM_CONFIG_DIR override)
- ✅ Auto-create config directory on startup
- ✅ Config example bundled in Tauri app

**Testing:**
- ✅ 30+ comprehensive test cases using `googletest` and `serial_test`
- ✅ Organized tests into separate modules (config, auth, server, websocket, desktop_id, edge_cases)
- ✅ RAII `EnvGuard` pattern for test isolation
- ✅ Coverage: happy paths, validation, edge cases, security scenarios
- ✅ All tests passing

**Quality Improvements Beyond Plan:**
- ✅ Constants for all ranges/defaults (single source of truth)
- ✅ Proper `ConfigError` types (not `Result<(), String>`)
- ✅ Port validation: MIN_PORT = 1024 (unprivileged)
- ✅ Removed impossible MAX_PORT check (u16 enforces it)
- ✅ Refactored repetitive env parsing into helper functions
- ✅ Removed function-level `use` statements
- ✅ Fixed Tauri resource bundling (avoided `_up_` directories)
- ✅ All clippy warnings resolved

### Verification Results

```bash
✅ cargo fmt --workspace          # Clean
✅ cargo clippy --workspace       # Clean (with -D warnings)
✅ cargo test --workspace         # All tests pass
✅ cargo build --workspace        # Builds successfully
✅ tauri dev                      # Runs correctly
```

### Key Decisions Made

1. **Port Range**: 1024-65535 (unprivileged ports only)
2. **Config Location**: `./.pm/` relative to working directory (not `~/.pm/`)
3. **Auth Optional**: `Option<Arc<JwtValidator>>` for desktop mode
4. **Desktop User ID**: Configurable or auto-generated UUID
5. **No Legacy Migration**: Removed legacy config check (new project)
6. **Helper Functions**: Clean env parsing instead of repetitive if-lets

### Files Summary

**Created**: 3
- `backend/crates/pm-config/src/websocket_config.rs`
- `backend/crates/pm-config/src/rate_limit_config.rs`
- `backend/config.example.toml`

**Modified**: 15+
- `backend/crates/pm-config/src/lib.rs` (organized constants)
- `backend/crates/pm-config/src/config.rs` (rewritten with helpers)
- `backend/crates/pm-config/src/server_config.rs` (added max_connections)
- `backend/crates/pm-config/src/auth_config.rs` (JWT support)
- `backend/crates/pm-config/src/logging_config.rs` (added colored)
- `backend/crates/pm-config/Cargo.toml` (added uuid)
- `backend/crates/pm-config/src/tests/*` (comprehensive test suite)
- `backend/pm-server/src/main.rs` (integrated pm-config)
- `backend/pm-server/src/error.rs` (updated error types)
- `backend/pm-server/src/logger.rs` (updated signature)
- `backend/pm-server/Cargo.toml` (added pm-config)
- `backend/crates/pm-ws/src/app_state.rs` (optional auth)
- `backend/crates/pm-ws/tests/common/test_server.rs` (optional auth)
- `desktop/src-tauri/tauri.conf.json` (bundled config.example.toml)

**Deleted**: 2
- `backend/pm-server/src/config.rs` (replaced by pm-config)
- Unused error variants and imports

### Lessons Learned

- Token estimates for quality work can be 2-3x initial estimates
- Catching issues early (bad error handling, magic numbers) saves time
- Constants as single source of truth prevents bugs
- Good test organization (separate modules) scales better
- RAII patterns (`EnvGuard`) prevent test pollution
- Type system can enforce constraints (u16 max = 65535)
