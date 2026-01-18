use crate::health;

use pm_ws::AppState;

use axum::{Router, routing::get};
use tower_http::cors::{Any, CorsLayer};

/// Build the application router with all endpoints
pub fn build_router(state: AppState) -> Router {
    Router::new()
        // WebSocket endpoint
        .route("/ws", get(pm_ws::handler))
        // Health check endpoints
        .route("/health", get(health::health))
        .route("/live", get(health::liveness))
        .route("/ready", get(health::readiness))
        // Add shared state
        .with_state(state)
        // CORS middleware (allow all origins for WebSocket)
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
}
