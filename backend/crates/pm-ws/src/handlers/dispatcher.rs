use crate::{
    HandlerContext, WsError, build_error_response, handle_create, handle_create_comment,
    handle_create_dependency, handle_create_project, handle_create_sprint,
    handle_create_time_entry, handle_delete, handle_delete_comment, handle_delete_dependency,
    handle_delete_project, handle_delete_sprint, handle_delete_time_entry, handle_get_comments,
    handle_get_dependencies, handle_get_running_timer, handle_get_sprints, handle_get_time_entries,
    handle_get_work_items, handle_list, handle_start_timer, handle_stop_timer, handle_update,
    handle_update_comment, handle_update_project, handle_update_sprint, handle_update_time_entry,
    log_handler_entry,
};

use pm_proto::{Pong, WebSocketMessage, web_socket_message::Payload};

use std::panic::Location;

use crate::handlers::activity_log::handle_get_activity_log;
use crate::handlers::llm_context::handle_get_llm_context;
use error_location::ErrorLocation;
use log::{error, info, warn};

/// Dispatch incoming WebSocket message to appropriate handler.
/// Includes:
/// - Correlation ID tracking
/// - Structured logging
/// - Timeout protection
/// - Circuit breaker awareness
pub async fn dispatch(msg: WebSocketMessage, ctx: HandlerContext) -> WebSocketMessage {
    let message_id = msg.message_id.clone();
    let handler_name = payload_to_handler_name(&msg.payload);

    log_handler_entry!(ctx.request_ctx, handler_name);

    // Wrap handler execution with timeout
    let response = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        dispatch_inner(msg, ctx.clone()),
    )
    .await;

    let final_response = match response {
        Ok(resp) => resp,
        Err(_elapsed) => {
            error!(
                "{} Handler {} timed out after 30s",
                ctx.log_prefix(),
                handler_name
            );
            build_error_response(
                &message_id,
                pm_proto::Error {
                    code: "TIMEOUT".to_string(),
                    message: "Request timed out. Please try again.".to_string(),
                    field: None,
                },
            )
        }
    };

    info!(
        "{} <- {} completed in {}ms",
        ctx.log_prefix(),
        handler_name,
        ctx.request_ctx.elapsed_ms()
    );

    final_response
}

async fn dispatch_inner(msg: WebSocketMessage, ctx: HandlerContext) -> WebSocketMessage {
    let message_id = msg.message_id.clone();
    let handler_name = payload_to_handler_name(&msg.payload);
    let log_prefix = ctx.log_prefix();

    let result = match msg.payload {
        // Work Item handlers
        Some(Payload::CreateWorkItemRequest(req)) => handle_create(req, ctx).await,
        Some(Payload::UpdateWorkItemRequest(req)) => handle_update(req, ctx).await,
        Some(Payload::DeleteWorkItemRequest(req)) => handle_delete(req, ctx).await,
        Some(Payload::GetWorkItemsRequest(req)) => handle_get_work_items(req, ctx).await,

        // Project handlers
        Some(Payload::CreateProjectRequest(req)) => handle_create_project(req, ctx).await,
        Some(Payload::UpdateProjectRequest(req)) => handle_update_project(req, ctx).await,
        Some(Payload::DeleteProjectRequest(req)) => handle_delete_project(req, ctx).await,
        Some(Payload::ListProjectsRequest(req)) => handle_list(req, ctx).await,

        // Sprint handlers
        Some(Payload::CreateSprintRequest(req)) => handle_create_sprint(req, ctx).await,
        Some(Payload::UpdateSprintRequest(req)) => handle_update_sprint(req, ctx).await,
        Some(Payload::DeleteSprintRequest(req)) => handle_delete_sprint(req, ctx).await,
        Some(Payload::GetSprintsRequest(req)) => handle_get_sprints(req, ctx).await,

        // Comment handlers
        Some(Payload::CreateCommentRequest(req)) => handle_create_comment(req, ctx).await,
        Some(Payload::UpdateCommentRequest(req)) => handle_update_comment(req, ctx).await,
        Some(Payload::DeleteCommentRequest(req)) => handle_delete_comment(req, ctx).await,
        Some(Payload::GetCommentsRequest(req)) => handle_get_comments(req, ctx).await,

        // Time Entry handlers
        Some(Payload::StartTimerRequest(req)) => handle_start_timer(req, ctx).await,
        Some(Payload::StopTimerRequest(req)) => handle_stop_timer(req, ctx).await,
        Some(Payload::CreateTimeEntryRequest(req)) => handle_create_time_entry(req, ctx).await,
        Some(Payload::UpdateTimeEntryRequest(req)) => handle_update_time_entry(req, ctx).await,
        Some(Payload::DeleteTimeEntryRequest(req)) => handle_delete_time_entry(req, ctx).await,
        Some(Payload::GetTimeEntriesRequest(req)) => handle_get_time_entries(req, ctx).await,
        Some(Payload::GetRunningTimerRequest(req)) => handle_get_running_timer(req, ctx).await,

        // Dependency handlers
        Some(Payload::CreateDependencyRequest(req)) => handle_create_dependency(req, ctx).await,
        Some(Payload::DeleteDependencyRequest(req)) => handle_delete_dependency(req, ctx).await,
        Some(Payload::GetDependenciesRequest(req)) => handle_get_dependencies(req, ctx).await,

        // Activity Log handlers
        Some(Payload::GetActivityLogRequest(req)) => handle_get_activity_log(req, ctx).await,

        // LLM Context handlers
        Some(Payload::GetLlmContextRequest(req)) => handle_get_llm_context(req, ctx).await,

        // Ping/Pong
        Some(Payload::Ping(ping)) => {
            return WebSocketMessage {
                message_id,
                timestamp: chrono::Utc::now().timestamp(),
                payload: Some(Payload::Pong(Pong {
                    timestamp: ping.timestamp,
                })),
            };
        }

        // Not yet implemented
        Some(Payload::Subscribe(_)) | Some(Payload::Unsubscribe(_)) => {
            return build_error_response(
                &message_id,
                pm_proto::Error {
                    code: "NOT_IMPLEMENTED".to_string(),
                    message: "Subscription handling coming in Session 20".to_string(),
                    field: None,
                },
            );
        }

        // Unknown payload
        _ => Err(WsError::InvalidMessage {
            message: "Unsupported or missing message payload".to_string(),
            location: ErrorLocation::from(Location::caller()),
        }),
    };

    match result {
        Ok(response) => response,
        Err(e) => {
            let proto_error = e.to_proto_error();
            warn!(
                "{} Handler {} failed: {} (code: {})",
                log_prefix, // Change from ctx.log_prefix() to log_prefix
                handler_name,
                e,
                proto_error.code
            );
            build_error_response(&message_id, proto_error)
        }
    }
}

fn payload_to_handler_name(payload: &Option<Payload>) -> &'static str {
    match payload {
        // Work Items
        Some(Payload::CreateWorkItemRequest(_)) => "CreateWorkItem",
        Some(Payload::UpdateWorkItemRequest(_)) => "UpdateWorkItem",
        Some(Payload::DeleteWorkItemRequest(_)) => "DeleteWorkItem",
        Some(Payload::GetWorkItemsRequest(_)) => "GetWorkItems",

        // Projects
        Some(Payload::CreateProjectRequest(_)) => "CreateProject",
        Some(Payload::UpdateProjectRequest(_)) => "UpdateProject",
        Some(Payload::DeleteProjectRequest(_)) => "DeleteProject",
        Some(Payload::ListProjectsRequest(_)) => "ListProjects",

        // Control
        Some(Payload::Subscribe(_)) => "Subscribe",
        Some(Payload::Unsubscribe(_)) => "Unsubscribe",
        Some(Payload::Ping(_)) => "Ping",

        // Time Entry
        Some(Payload::StartTimerRequest(_)) => "StartTimer",
        Some(Payload::StopTimerRequest(_)) => "StopTimer",
        Some(Payload::CreateTimeEntryRequest(_)) => "CreateTimeEntry",
        Some(Payload::UpdateTimeEntryRequest(_)) => "UpdateTimeEntry",
        Some(Payload::DeleteTimeEntryRequest(_)) => "DeleteTimeEntry",
        Some(Payload::GetTimeEntriesRequest(_)) => "GetTimeEntries",
        Some(Payload::GetRunningTimerRequest(_)) => "GetRunningTimer",

        // Dependency
        Some(Payload::CreateDependencyRequest(_)) => "CreateDependency",
        Some(Payload::DeleteDependencyRequest(_)) => "DeleteDependency",
        Some(Payload::GetDependenciesRequest(_)) => "GetDependencies",

        // Activity Log
        Some(Payload::GetActivityLogRequest(_)) => "GetActivityLog",

        // LLM Context
        Some(Payload::GetLlmContextRequest(_)) => "GetLlmContext",

        _ => "Unknown",
    }
}
