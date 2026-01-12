use crate::{
    ConnectionConfig, ConnectionRegistry, Metrics, ShutdownCoordinator, TenantBroadcaster,
    WebSocketConnection,
};
use axum::{
    extract::{
        ws::{WebSocket, WebSocketUpgrade},
        State,
    },
    http::{HeaderMap, StatusCode},
    response::Response,
};
use pm_auth::{JwtValidator, RateLimiterFactory, TenantContext};
use std::sync::Arc;

/// Shared application state for WebSocket handlers
#[derive(Clone)]
pub struct AppState {
    pub jwt_validator: Arc<JwtValidator>,
    pub rate_limiter_factory: RateLimiterFactory,
    pub broadcaster: TenantBroadcaster,
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
    let tenant_context = extract_tenant_context(&headers, &state.jwt_validator)?;

    log::debug!(                                                                                                                                                                 
          "WebSocket upgrade request from tenant {} (user {})",                                                                                                                    
          tenant_context.tenant_id,                                                                                                                                                
          tenant_context.user_id                                                                                                                                                   
      );

    // Register connection (enforces connection limits)                                                                                                                          
    let connection_id = state
        .registry
        .register(
            tenant_context.tenant_id.clone(),
            tenant_context.user_id.clone(),
        )
        .await
        .map_err(|e| {
            log::error!("Failed to register connection: {}", e);
            StatusCode::SERVICE_UNAVAILABLE
        })?;

    log::info!(                                                                                                                                                                  
          "Registered connection {} for tenant {}",                                                                                                                                
          connection_id,                                                                                                                                                           
          tenant_context.tenant_id                                                                                                                                                 
      );

    // Create rate limiter for this connection                                                                                                                                   
    let rate_limiter = state.rate_limiter_factory.create();

    // Upgrade to WebSocket                                                                                                                                                      
    Ok(ws.on_upgrade(move |socket| {
        handle_socket(
            socket,
            connection_id,
            tenant_context,
            state,
            rate_limiter,
        )
    }))
}

/// Handle WebSocket connection after upgrade                                                                                                                                    
async fn handle_socket(
    socket: WebSocket,
    connection_id: crate::ConnectionId,
    tenant_context: TenantContext,
    state: AppState,
    rate_limiter: pm_auth::ConnectionRateLimiter,
) {
    let shutdown_guard = state.shutdown.subscribe_guard();

    let connection = WebSocketConnection::new(
        connection_id,
        tenant_context.clone(),
        state.config,
        state.metrics.clone(),
        rate_limiter,
        state.broadcaster.clone(),
    );

    // Handle connection lifecycle                                                                                                                                               
    let result = connection.handle(socket, shutdown_guard).await;

    // Unregister on disconnect                                                                                                                                                  
    state.registry.unregister(connection_id).await;

    if let Err(e) = result {
        log::error!("Connection {} error: {}", connection_id, e);
    }
}

/// Extract and validate tenant context from JWT in Authorization header                                                                                                         
fn extract_tenant_context(
    headers: &HeaderMap,
    validator: &JwtValidator,
) -> Result<TenantContext, StatusCode> {
    // Get Authorization header                                                                                                                                                  
    let auth_header = headers
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| {
            log::warn!("Missing Authorization header");
            StatusCode::UNAUTHORIZED
        })?;

    // Check Bearer scheme                                                                                                                                                       
    if !auth_header.starts_with("Bearer ") {
        log::warn!("Invalid authorization scheme: expected 'Bearer'");
        return Err(StatusCode::UNAUTHORIZED);
    }

    // Extract token                                                                                                                                                             
    let token = &auth_header[7..]; // Skip "Bearer "                                                                                                                             

    // Validate JWT                                                                                                                                                              
    let claims = validator.validate(token).map_err(|e| {
        log::warn!("JWT validation failed: {}", e);
        StatusCode::UNAUTHORIZED
    })?;

    // Convert to TenantContext                                                                                                                                                  
    Ok(TenantContext::from_claims(claims))
} 