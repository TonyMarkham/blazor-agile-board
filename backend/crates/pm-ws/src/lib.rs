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
    comment::{
        handle_create_comment, handle_delete_comment, handle_get_comments, handle_update_comment,
    },
    connection::extract_user_id,
    context::HandlerContext,
    db_ops::{db_read, db_transaction, db_write},
    dependency::{handle_create_dependency, handle_delete_dependency, handle_get_dependencies},
    dispatcher::dispatch,
    error_boundary::{sanitize_error_message, with_error_boundary},
    error_codes::{
        CONFLICT, DELETE_BLOCKED, INTERNAL_ERROR, INVALID_MESSAGE, NOT_FOUND, RATE_LIMITED,
        UNAUTHORIZED, VALIDATION_ERROR,
    },
    field_change_builder::FieldChangeBuilder,
    hierarchy_validator::validate_hierarchy,
    idempotency::{
        check_idempotency, decode_cached_response, store_idempotency, store_idempotency_non_fatal,
    },
    project::{
        handle_create as handle_create_project, handle_delete as handle_delete_project,
        handle_list, handle_update as handle_update_project,
    },
    query::handle_get_work_items,
    response_builder::{
        build_activity_log_created_event, build_activity_log_list_response,
        build_comment_created_response, build_comment_deleted_response,
        build_comment_updated_response, build_comments_list_response,
        build_dependencies_list_response, build_dependency_created_response,
        build_dependency_deleted_response, build_error_response, build_llm_context_list_response,
        build_project_created_response, build_project_deleted_response,
        build_project_list_response, build_project_updated_response, build_running_timer_response,
        build_sprint_created_response, build_sprint_deleted_response,
        build_sprint_updated_response, build_sprints_list_response,
        build_time_entries_list_response, build_time_entry_created_response,
        build_time_entry_deleted_response, build_time_entry_updated_response,
        build_timer_started_response, build_timer_stopped_response,
        build_work_item_created_response, build_work_item_deleted_response,
        build_work_item_updated_response, build_work_items_list_response,
    },
    sprint::{
        handle_create_sprint, handle_delete_sprint, handle_get_sprints, handle_update_sprint,
    },
    subscription::{handle_subscribe, handle_unsubscribe},
    time_entry::{
        handle_create_time_entry, handle_delete_time_entry, handle_get_running_timer,
        handle_get_time_entries, handle_start_timer, handle_stop_timer, handle_update_time_entry,
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
pub use web_socket_connection::{MAX_VIOLATIONS, WebSocketConnection, WebSocketConnectionParams};

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
