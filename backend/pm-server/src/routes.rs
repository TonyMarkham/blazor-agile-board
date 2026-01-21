use crate::{admin, health};

use pm_ws::AppState;

use axum::{
    Router,
    routing::{get, post},
};
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
        // Admin endpoints
        .route("/admin/checkpoint", post(admin::checkpoint_handler))
        .route("/admin/shutdown", post(admin::shutdown_handler))
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
