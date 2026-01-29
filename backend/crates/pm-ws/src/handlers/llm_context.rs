use crate::{HandlerContext, Result as WsErrorResult, build_llm_context_list_response, db_read};

use pm_db::LlmContextRepository;
use pm_proto::{GetLlmContextRequest, WebSocketMessage};

pub async fn handle_get_llm_context(
    req: GetLlmContextRequest,
    ctx: HandlerContext,
) -> WsErrorResult<WebSocketMessage> {
    log::debug!("{} GetLlmContext starting", ctx.log_prefix());

    let entries = db_read(&ctx, "list_llm_context", || async {
        LlmContextRepository::list_filtered(
            &ctx.pool,
            req.category.as_deref(),
            req.context_type.as_deref(),
            req.min_priority,
        )
        .await
        .map_err(crate::WsError::from)
    })
    .await?;

    Ok(build_llm_context_list_response(&ctx.message_id, entries))
}
