use crate::error::{Result as ServerErrorResult, ServerError};

use std::net::SocketAddr;

/// Server configuration loaded from environment variables
#[derive(Debug, Clone)]
pub struct Config {
    /// Server bind address (default: 0.0.0.0:3000)
    pub bind_addr: SocketAddr,

    /// JWT secret for HS256 validation
    pub jwt_secret: Option<String>,

    /// JWT public key for RS256 validation (PEM format)
    pub jwt_public_key: Option<String>,

    /// Maximum connections per tenant (default: 1000)
    pub max_connections_per_tenant: usize,

    /// Maximum total connections (default: 10000)
    pub max_total_connections: usize,

    /// Rate limit: max requests per connection (default: 100)
    pub rate_limit_requests: u32,

    /// Rate limit: window in seconds (default: 60)
    pub rate_limit_window_secs: u64,

    /// WebSocket send buffer size (default: 100)
    pub ws_send_buffer_size: usize,

    /// Heartbeat interval in seconds (default: 30)
    pub heartbeat_interval_secs: u64,

    /// Heartbeat timeout in seconds (default: 60)
    pub heartbeat_timeout_secs: u64,

    /// Broadcast channel capacity per tenant (default: 1000)
    pub broadcast_capacity: usize,

    /// Log level (default: info)
    pub log_level: String,

    /// Enable colored logs (default: true)
    pub log_colored: bool,
}

impl Config {
    /// Load configuration from environment variables
    pub fn from_env() -> ServerErrorResult<Self> {
        // Load .env file if present (development)
        let _ = dotenvy::dotenv();

        let bind_addr = std::env::var("BIND_ADDR")
            .unwrap_or_else(|_| "0.0.0.0:3000".to_string())
            .parse()
            .map_err(|source| ServerError::InvalidBindAddr { source })?;

        let config = Self {
            bind_addr,

            jwt_secret: std::env::var("JWT_SECRET").ok(),
            jwt_public_key: std::env::var("JWT_PUBLIC_KEY").ok(),

            max_connections_per_tenant: std::env::var("MAX_CONNECTIONS_PER_TENANT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(1000),

            max_total_connections: std::env::var("MAX_TOTAL_CONNECTIONS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(10000),

            rate_limit_requests: std::env::var("RATE_LIMIT_REQUESTS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(100),

            rate_limit_window_secs: std::env::var("RATE_LIMIT_WINDOW_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(60),

            ws_send_buffer_size: std::env::var("WS_SEND_BUFFER_SIZE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(100),

            heartbeat_interval_secs: std::env::var("HEARTBEAT_INTERVAL_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(30),

            heartbeat_timeout_secs: std::env::var("HEARTBEAT_TIMEOUT_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(60),

            broadcast_capacity: std::env::var("BROADCAST_CAPACITY")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(1000),

            log_level: std::env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string()),

            log_colored: std::env::var("LOG_COLORED")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(true),
        };

        // Validate: must have either JWT_SECRET or JWT_PUBLIC_KEY
        config.validate()?;

        Ok(config)
    }

    /// Validate configuration
    fn validate(&self) -> ServerErrorResult<()> {
        if self.jwt_secret.is_none() && self.jwt_public_key.is_none() {
            return Err(ServerError::MissingJwtConfig);
        }

        if self.jwt_secret.is_some() && self.jwt_public_key.is_some() {
            log::warn!("Both JWT_SECRET and JWT_PUBLIC_KEY provided, using JWT_SECRET (HS256)");
        }

        Ok(())
    }
}
