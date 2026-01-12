use crate::health;
use axum::{
    routing::get,
    Router,
};
use pm_ws::AppState;
use tower_http::cors::{CorsLayer, Any};

/// Build the application router with all endpoints
pub fn build_router(state: AppState) -> Router {
    Router::new()
        // WebSocket endpoint
        .route("/ws", get(pm_ws::handler))

        // Health check endpoints
        .route("/health", get(health::health_check))
        .route("/live", get(health::liveness_check))
        .route("/ready", get(health::readiness_check))

        // Add shared state
        .with_state(state)

        // CORS middleware (allow all origins for WebSocket)
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any)
        )
}