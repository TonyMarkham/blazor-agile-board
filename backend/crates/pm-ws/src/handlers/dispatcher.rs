use crate::{
    HandlerContext, WsError, build_error_response, handle_create, handle_create_project,
    handle_delete, handle_delete_project, handle_get_work_items, handle_list, handle_update,
    handle_update_project, log_handler_entry,
};

use pm_proto::{Pong, WebSocketMessage, web_socket_message::Payload};

use std::panic::Location;

use error_location::ErrorLocation;

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
            log::error!(
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

    log::info!(
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
            log::warn!(
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

        _ => "Unknown",
    }
}
