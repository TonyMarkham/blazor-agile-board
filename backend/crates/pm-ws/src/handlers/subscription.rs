use crate::{HandlerContext, Result as WsErrorResult};

use pm_proto::{Subscribe, Unsubscribe, WebSocketMessage};

use chrono::Utc;
use log::info;

pub async fn handle_subscribe(
    req: Subscribe,
    ctx: HandlerContext,
) -> WsErrorResult<WebSocketMessage> {
    let connection_id = ctx.request_ctx.connection_id.clone();

    ctx.registry
        .subscribe(&connection_id, &req.project_ids, &req.sprint_ids)
        .await?;

    info!(
        "{} Subscribe: projects={} sprints={}",
        ctx.log_prefix(),
        req.project_ids.len(),
        req.sprint_ids.len()
    );

    Ok(WebSocketMessage {
        message_id: ctx.message_id,
        timestamp: Utc::now().timestamp(),
        payload: None,
    })
}

pub async fn handle_unsubscribe(
    req: Unsubscribe,
    ctx: HandlerContext,
) -> WsErrorResult<WebSocketMessage> {
    let connection_id = ctx.request_ctx.connection_id.clone();

    ctx.registry
        .unsubscribe(&connection_id, &req.project_ids, &req.sprint_ids)
        .await?;

    info!(
        "{} Unsubscribe: projects={} sprints={}",
        ctx.log_prefix(),
        req.project_ids.len(),
        req.sprint_ids.len()
    );

    Ok(WebSocketMessage {
        message_id: ctx.message_id,
        timestamp: Utc::now().timestamp(),
        payload: None,
    })
}
