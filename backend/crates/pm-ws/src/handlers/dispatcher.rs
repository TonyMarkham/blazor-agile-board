use crate::{
    HandlerContext, WsError, build_error_response, handle_create, handle_delete,
    handle_get_work_items, handle_update, log_handler_entry,
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

    let result = match msg.payload {
        Some(Payload::CreateWorkItemRequest(req)) => handle_create(req, ctx).await,
        Some(Payload::UpdateWorkItemRequest(req)) => handle_update(req, ctx).await,
        Some(Payload::DeleteWorkItemRequest(req)) => handle_delete(req, ctx).await,
        Some(Payload::GetWorkItemsRequest(req)) => handle_get_work_items(req, ctx).await,
        Some(Payload::Ping(ping)) => {
            return WebSocketMessage {
                message_id,
                timestamp: chrono::Utc::now().timestamp(),
                payload: Some(Payload::Pong(Pong {
                    timestamp: ping.timestamp,
                })),
            };
        }
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
        _ => Err(WsError::InvalidMessage {
            message: "Unsupported or missing message payload".to_string(),
            location: ErrorLocation::from(Location::caller()),
        }),
    };

    match result {
        Ok(response) => response,
        Err(e) => build_error_response(&message_id, e.to_proto_error()),
    }
}

fn payload_to_handler_name(payload: &Option<Payload>) -> &'static str {
    match payload {
        Some(Payload::CreateWorkItemRequest(_)) => "CreateWorkItem",
        Some(Payload::UpdateWorkItemRequest(_)) => "UpdateWorkItem",
        Some(Payload::DeleteWorkItemRequest(_)) => "DeleteWorkItem",
        Some(Payload::GetWorkItemsRequest(_)) => "GetWorkItems",
        Some(Payload::Subscribe(_)) => "Subscribe",
        Some(Payload::Unsubscribe(_)) => "Unsubscribe",
        Some(Payload::Ping(_)) => "Ping",
        _ => "Unknown",
    }
}
