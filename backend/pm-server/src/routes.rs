use crate::{
    admin, create_comment, create_project, create_sprint, create_work_item, delete_comment,
    delete_project, delete_sprint, delete_work_item, get_project, get_sprint, get_work_item,
    health, list_comments, list_projects, list_sprints, list_work_items, update_comment,
    update_project, update_sprint, update_work_item,
};

use pm_ws::AppState;

use axum::{
    Router,
    routing::{delete, get, post, put},
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
        // REST API v1 - Projects
        .route("/api/v1/projects", get(list_projects))
        .route("/api/v1/projects", post(create_project))
        .route("/api/v1/projects/{id}", get(get_project))
        .route("/api/v1/projects/{id}", put(update_project))
        .route("/api/v1/projects/{id}", delete(delete_project))
        // REST API v1 - Sprints
        .route("/api/v1/projects/{project_id}/sprints", get(list_sprints))
        .route("/api/v1/sprints", post(create_sprint))
        .route("/api/v1/sprints/{id}", get(get_sprint))
        .route("/api/v1/sprints/{id}", put(update_sprint))
        .route("/api/v1/sprints/{id}", delete(delete_sprint))
        // REST API v1 - Work Items
        .route(
            "/api/v1/projects/{project_id}/work-items",
            get(list_work_items),
        )
        .route("/api/v1/work-items", post(create_work_item))
        .route("/api/v1/work-items/{id}", get(get_work_item))
        .route("/api/v1/work-items/{id}", put(update_work_item))
        .route("/api/v1/work-items/{id}", delete(delete_work_item))
        // REST API v1 - Comments
        .route(
            "/api/v1/work-items/{work_item_id}/comments",
            get(list_comments),
        )
        .route(
            "/api/v1/work-items/{work_item_id}/comments",
            post(create_comment),
        )
        .route("/api/v1/comments/{id}", put(update_comment))
        .route("/api/v1/comments/{id}", delete(delete_comment))
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
