use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};

use serde_json::json;

/// GET /health - Comprehensive health check with component status                                                                                                               
pub async fn health_check() -> Response {
    // TODO: Add actual component checks in future sessions:
    // - Database connection pool status
    // - WebSocket connection count
    // - Broadcast channel status
    // - Memory usage

    let health = json!({
        "status": "healthy",
        "version": env!("CARGO_PKG_VERSION"),
        "components": {
            "websocket": "operational",
            "auth": "operational",
            "database": "not_implemented",  // TODO: Session 30+
        },
        "timestamp": chrono::Utc::now().to_rfc3339(),
    });

    (StatusCode::OK, Json(health)).into_response()
}

/// GET /live - Kubernetes liveness probe (is the process alive?)                                                                                                                
pub async fn liveness_check() -> Response {
    // Simple check: if we can respond, we're alive
    (StatusCode::OK, "OK").into_response()
}

/// GET /ready - Kubernetes readiness probe (ready to accept traffic?)                                                                                                           
pub async fn readiness_check() -> Response {
    // TODO: Add actual readiness checks:
    // - Can we accept WebSocket connections?
    // - Is database pool ready?
    // - Are broadcast channels initialized?

    // For now, if server is running, it's ready
    (StatusCode::OK, "Ready").into_response()
}
