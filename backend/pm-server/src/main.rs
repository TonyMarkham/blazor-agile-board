mod config;
mod error;
mod health;
mod logger;
mod routes;

use crate::config::Config;

use pm_auth::{JwtValidator, RateLimitConfig, RateLimiterFactory};
use pm_ws::{
    AppState, BroadcastConfig, ConnectionConfig, ConnectionLimits, ConnectionRegistry, Metrics,
    ShutdownCoordinator, TenantBroadcaster,
};

use std::error::Error;
use std::sync::Arc;

use log::{error, info};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Load configuration from environment
    let config = Config::from_env()?;

    // Initialize logger
    logger::initialize(&config.log_level, config.log_colored)?;

    info!("Starting pm-server v{}", env!("CARGO_PKG_VERSION"));
    info!("Configuration loaded: bind_addr={}", config.bind_addr);

    // Create JWT validator (HS256 or RS256 based on config)
    let jwt_validator = if let Some(secret) = &config.jwt_secret {
        info!("Using HS256 JWT validation");
        Arc::new(JwtValidator::with_hs256(secret.as_bytes()))
    } else if let Some(public_key) = &config.jwt_public_key {
        info!("Using RS256 JWT validation");
        Arc::new(JwtValidator::with_rs256(public_key)?)
    } else {
        unreachable!("Config validation ensures at least one JWT method is present");
    };

    // Create rate limiter factory
    let rate_limiter_factory = RateLimiterFactory::new(RateLimitConfig {
        max_requests: config.rate_limit_requests,
        window_secs: config.rate_limit_window_secs,
    });

    // Create connection registry with limits
    let registry = ConnectionRegistry::new(ConnectionLimits {
        max_per_tenant: config.max_connections_per_tenant,
        max_total: config.max_total_connections,
    });

    // Create tenant broadcaster
    let broadcaster = TenantBroadcaster::new(BroadcastConfig {
        channel_capacity: config.broadcast_capacity,
    });

    // Create metrics collector
    let metrics = Metrics::new();

    // Create shutdown coordinator
    let shutdown = ShutdownCoordinator::new();

    // Create connection config
    let connection_config = ConnectionConfig {
        send_buffer_size: config.ws_send_buffer_size,
        heartbeat_interval_secs: config.heartbeat_interval_secs,
        heartbeat_timeout_secs: config.heartbeat_timeout_secs,
    };

    // Build application state
    let app_state = AppState {
        jwt_validator,
        rate_limiter_factory,
        broadcaster,
        registry,
        metrics,
        shutdown: shutdown.clone(),
        config: connection_config,
    };

    // Build router
    let app = routes::build_router(app_state);

    // Create TCP listener
    let listener = TcpListener::bind(&config.bind_addr).await?;
    info!("Server listening on {}", config.bind_addr);

    // Spawn signal handler for graceful shutdown
    let shutdown_for_signal = shutdown.clone();
    tokio::spawn(async move {
        match tokio::signal::ctrl_c().await {
            Ok(()) => {
                info!("Received SIGINT (Ctrl+C), initiating graceful shutdown");
                shutdown_for_signal.shutdown();
            }
            Err(e) => {
                error!("Failed to listen for SIGINT: {}", e);
            }
        }
    });

    // Start server with graceful shutdown
    info!("Server ready to accept connections");
    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            shutdown.subscribe_guard().wait().await;
            info!("Graceful shutdown complete");
        })
        .await?;

    Ok(())
}
