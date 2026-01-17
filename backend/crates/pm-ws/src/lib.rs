pub mod app_state;
pub mod broadcast_config;
pub mod broadcast_info;
pub mod broadcast_message;
pub mod client_subscriptions;
pub mod connection_config;
pub mod connection_id;
pub mod connection_info;
pub mod connection_limits;
pub mod connection_registry;
pub mod error;
pub mod handlers;
pub mod message_validator;
pub mod metrics;
pub mod metrics_timer;
pub mod shutdown_coordinator;
pub mod shutdown_guard;
pub mod subscription_filter;
pub mod tenant_broadcaster;
pub mod web_socket_connection;

pub use app_state::{AppState, handler};
pub use broadcast_config::BroadcastConfig;
pub use broadcast_info::BroadcastInfo;
pub use broadcast_message::BroadcastMessage;
pub use client_subscriptions::ClientSubscriptions;
pub use connection_config::ConnectionConfig;
pub use connection_id::ConnectionId;
pub use connection_info::ConnectionInfo;
pub use connection_limits::ConnectionLimits;
pub use connection_registry::ConnectionRegistry;
pub use error::{Result, WsError};
pub use handlers::authorization::check_permission;
pub use handlers::change_tracker::track_changes;
pub use handlers::context::HandlerContext;
pub use handlers::error_codes::{
    CONFLICT, DELETE_BLOCKED, INTERNAL_ERROR, INVALID_MESSAGE, NOT_FOUND, RATE_LIMITED,
    UNAUTHORIZED, VALIDATION_ERROR,
};
pub use handlers::hierarchy_validator::validate_hierarchy;
pub use handlers::idempotency::{check_idempotency, store_idempotency};
pub use handlers::response_builder::{
    build_work_item_created_response, build_work_item_deleted_response,
    build_work_item_updated_response, build_work_items_list_response,
};
pub use message_validator::MessageValidator;
pub use metrics::Metrics;
pub use metrics_timer::MetricsTimer;
pub use shutdown_coordinator::ShutdownCoordinator;
pub use shutdown_guard::ShutdownGuard;
pub use subscription_filter::SubscriptionFilter;
pub use tenant_broadcaster::TenantBroadcaster;
pub use web_socket_connection::{MAX_VIOLATIONS, WebSocketConnection};

#[cfg(test)]
mod tests;

use tracing::info_span;

/// Create a tracing span for a WebSocket request.
/// All log entries within the handler will include these fields.
pub fn create_request_span(
    message_id: &str,
    tenant_id: &str,
    user_id: &str,
    operation: &str,
) -> tracing::Span {
    info_span!(
        "ws_request",
        message_id = %message_id,
        tenant_id = %tenant_id,
        user_id = %user_id,
        operation = %operation,
    )
}
