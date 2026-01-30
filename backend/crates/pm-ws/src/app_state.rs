use crate::{
    ConnectionConfig, ConnectionRegistry, Metrics, ShutdownCoordinator, WebSocketConnection,
    WebSocketConnectionParams, circuit_breaker::CircuitBreaker,
};

use pm_auth::{JwtValidator, RateLimiterFactory};

use std::sync::Arc;

use axum::{
    extract::{
        Query, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::{HeaderMap, StatusCode},
    response::Response,
};
use log::{debug, error, warn};
use sqlx::SqlitePool;
use tokio::sync::mpsc;

/// Shared application state for WebSocket handlers
#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,                     // NEW
    pub circuit_breaker: Arc<CircuitBreaker>, // NEW
    pub jwt_validator: Option<Arc<JwtValidator>>,
    pub desktop_user_id: String,
    pub rate_limiter_factory: RateLimiterFactory,
    pub registry: ConnectionRegistry,
    pub metrics: Metrics,
    pub shutdown: ShutdownCoordinator,
    pub config: ConnectionConfig,
}

/// WebSocket upgrade handler
pub async fn handler(
    State(state): State<AppState>,
    Query(params): Query<std::collections::HashMap<String, String>>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> Result<Response, StatusCode> {
    // Extract and validate user ID (JWT or desktop mode)
    let user_id = extract_user_id(
        &headers,
        &params,
        &state.jwt_validator,
        &state.desktop_user_id,
    )?;
    debug!("WebSocket upgrade request from user {}", user_id);

    if state.registry.is_at_total_limit().await {
        warn!("WebSocket connection rejected: total limit reached");
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }

    // Create rate limiter for this connection
    let rate_limiter = state.rate_limiter_factory.create();

    // Parse user_id to Uuid for handlers
    let user_uuid = uuid::Uuid::parse_str(&user_id).unwrap_or_else(|_| uuid::Uuid::new_v4());

    // Ensure user exists in database (desktop mode)
    if state.jwt_validator.is_none() {
        let pool = state.pool.clone();
        let user_id_str = user_uuid.to_string();
        tokio::spawn(async move {
            let _ = sqlx::query("INSERT OR IGNORE INTO users (id, email) VALUES (?, ?)")
                .bind(&user_id_str)
                .bind(format!("user-{}@localhost", &user_id_str[..8]))
                .execute(&pool)
                .await;
        });
    }

    // Upgrade to WebSocket
    Ok(ws.on_upgrade(move |socket| handle_socket(socket, state, rate_limiter, user_uuid)))
}

/// Handle WebSocket connection after upgrade
async fn handle_socket(
    socket: WebSocket,
    state: AppState,
    rate_limiter: pm_auth::ConnectionRateLimiter,
    user_id: uuid::Uuid,
) {
    let shutdown_guard = state.shutdown.subscribe_guard();

    // Create bounded channel for outgoing messages (backpressure handling)
    let (tx, rx) = mpsc::channel::<Message>(state.config.send_buffer_size);

    // Register connection (enforces connection limits)
    let connection_id = state
        .registry
        .register(user_id.to_string(), tx.clone())
        .await
        .unwrap();

    let connection = WebSocketConnection::new(WebSocketConnectionParams {
        connection_id,
        config: state.config,
        metrics: state.metrics.clone(),
        rate_limiter,
        pool: state.pool.clone(),
        circuit_breaker: state.circuit_breaker.clone(),
        user_id,
        registry: state.registry.clone(),
        outgoing_rx: rx,
        outgoing_tx: tx,
    });

    // Handle connection lifecycle
    let result = connection.handle(socket, shutdown_guard).await;

    // Unregister on disconnect
    state.registry.unregister(connection_id).await;

    if let Err(e) = result {
        error!("Connection {connection_id} error: {e}");
    }
}

/// Extract and validate user ID from Authorization header or query params (desktop mode)
fn extract_user_id(
    headers: &HeaderMap,
    query_params: &std::collections::HashMap<String, String>,
    validator: &Option<Arc<JwtValidator>>,
    desktop_user_id_fallback: &str,
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
            // Auth disabled - check query params for user_id first
            if let Some(user_id) = query_params.get("user_id") {
                debug!("Desktop mode: using user_id from query params: {}", user_id);
                Ok(user_id.clone())
            } else {
                // Fallback to configured desktop user ID for legacy clients
                debug!(
                    "Desktop mode: no user_id in query params, using fallback: {}",
                    desktop_user_id_fallback
                );
                Ok(desktop_user_id_fallback.to_string())
            }
        }
    }
}
