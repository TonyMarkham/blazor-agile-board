mod app_state;
mod circuit_breaker;
mod client_subscriptions;
mod connection_config;
mod connection_id;
mod connection_info;
mod connection_limits;
mod connection_registry;
mod error;
mod handlers;
mod message_validator;
mod metrics;
mod metrics_timer;
mod request_context;
mod request_logging;
mod retry;
mod shutdown_coordinator;
mod shutdown_guard;
mod subscription_filter;
mod web_socket_connection;

pub use app_state::{AppState, handler};
pub use circuit_breaker::{CircuitBreaker, CircuitBreakerConfig, CircuitState};
pub use client_subscriptions::ClientSubscriptions;
pub use connection_config::ConnectionConfig;
pub use connection_id::ConnectionId;
pub use connection_info::ConnectionInfo;
pub use connection_limits::ConnectionLimits;
pub use connection_registry::ConnectionRegistry;
pub use error::{Result, WsError};
pub use handlers::{
    authorization::check_permission,
    change_tracker::track_changes,
    connection::extract_user_id,
    context::HandlerContext,
    db_ops::{db_read, db_transaction, db_write},
    dispatcher::dispatch,
    error_boundary::{sanitize_error_message, with_error_boundary},
    error_codes::{
        CONFLICT, DELETE_BLOCKED, INTERNAL_ERROR, INVALID_MESSAGE, NOT_FOUND, RATE_LIMITED,
        UNAUTHORIZED, VALIDATION_ERROR,
    },
    hierarchy_validator::validate_hierarchy,
    idempotency::{check_idempotency, store_idempotency},
    project::{
        handle_create as handle_create_project, handle_delete as handle_delete_project,
        handle_list, handle_update as handle_update_project,
    },
    query::handle_get_work_items,
    response_builder::{
        build_error_response, build_work_item_created_response, build_work_item_deleted_response,
        build_work_item_updated_response, build_work_items_list_response,
    },
    work_item::{
        handle_create, handle_delete, handle_update, sanitize_string, validate_priority,
        validate_status,
    },
};
pub use message_validator::MessageValidator;
pub use metrics::Metrics;
pub use metrics_timer::MetricsTimer;
pub use request_context::RequestContext;
pub use request_logging::RequestLogger;
pub use retry::{IsRetryable, RetryConfig, with_retry};
pub use shutdown_coordinator::ShutdownCoordinator;
pub use shutdown_guard::ShutdownGuard;
pub use subscription_filter::SubscriptionFilter;
pub use web_socket_connection::{MAX_VIOLATIONS, WebSocketConnection};

#[cfg(test)]
mod tests;

use tracing::info_span;

/// Create a tracing span for a WebSocket request.
/// All log entries within the handler will include these fields.
pub fn create_request_span(message_id: &str, user_id: &str, operation: &str) -> tracing::Span {
    info_span!(
        "ws_request",
        message_id = %message_id,
        user_id = %user_id,
        operation = %operation,
    )
}
