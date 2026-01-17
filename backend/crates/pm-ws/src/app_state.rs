use crate::{
    ConnectionConfig, ConnectionRegistry, Metrics, ShutdownCoordinator, WebSocketConnection,
};
use axum::{
    extract::{
        State,
        ws::{WebSocket, WebSocketUpgrade},
    },
    http::{HeaderMap, StatusCode},
    response::Response,
};
use pm_auth::{JwtValidator, RateLimiterFactory};

use std::sync::Arc;

use log::{debug, error, info, warn};

/// Shared application state for WebSocket handlers
#[derive(Clone)]
pub struct AppState {
    pub jwt_validator: Arc<JwtValidator>,
    pub rate_limiter_factory: RateLimiterFactory,
    pub registry: ConnectionRegistry,
    pub metrics: Metrics,
    pub shutdown: ShutdownCoordinator,
    pub config: ConnectionConfig,
}

/// WebSocket upgrade handler                                                                                                                                                    
pub async fn handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> Result<Response, StatusCode> {
    // Extract and validate JWT from Authorization header
    let user_id = extract_user_id(&headers, &state.jwt_validator)?;
    debug!("WebSocket upgrade request from user {}", user_id);

    // Register connection (enforces connection limits)
    let connection_id = state
        .registry
        .register(user_id.clone())
        .await
        .map_err(|e| {
            error!("Failed to register connection: {}", e);
            StatusCode::SERVICE_UNAVAILABLE
        })?;

    info!("Registered connection {}", connection_id);

    // Create rate limiter for this connection
    let rate_limiter = state.rate_limiter_factory.create();

    // Upgrade to WebSocket
    Ok(ws.on_upgrade(move |socket| handle_socket(socket, connection_id, state, rate_limiter)))
}

/// Handle WebSocket connection after upgrade                                                                                                                                    
async fn handle_socket(
    socket: WebSocket,
    connection_id: crate::ConnectionId,
    state: AppState,
    rate_limiter: pm_auth::ConnectionRateLimiter,
) {
    let shutdown_guard = state.shutdown.subscribe_guard();

    let connection = WebSocketConnection::new(
        connection_id,
        state.config,
        state.metrics.clone(),
        rate_limiter,
    );

    // Handle connection lifecycle
    let result = connection.handle(socket, shutdown_guard).await;

    // Unregister on disconnect
    state.registry.unregister(connection_id).await;

    if let Err(e) = result {
        error!("Connection {connection_id} error: {e}");
    }
}

/// Extract and validate tenant context from JWT in Authorization header                                                                                                         
fn extract_user_id(headers: &HeaderMap, validator: &JwtValidator) -> Result<String, StatusCode> {
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

    let claims = validator.validate(token).map_err(|e| {
        warn!("JWT validation failed: {}", e);
        StatusCode::UNAUTHORIZED
    })?;

    Ok(claims.sub)
}
