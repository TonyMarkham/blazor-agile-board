use crate::{
    HandlerContext, Result as WsErrorResult, WsError, build_work_items_list_response,
    check_permission, db_read,
};

use error_location::ErrorLocation;
use pm_core::Permission;
use pm_db::WorkItemRepository;
use pm_proto::{GetWorkItemsRequest, WebSocketMessage};

use std::panic::Location;

use tracing::{debug, info, instrument};
use uuid::Uuid;

/// Handle GetWorkItemsRequest
#[instrument(skip(ctx), fields(project_id))]
pub async fn handle_get_work_items(
    req: GetWorkItemsRequest,
    ctx: HandlerContext,
) -> WsErrorResult<WebSocketMessage> {
    debug!("{} GetWorkItems starting", ctx.log_prefix());

    // 1. Parse project ID
    let project_id = Uuid::parse_str(&req.project_id).map_err(|_| WsError::ValidationError {
        message: format!("Invalid project_id: {}", req.project_id),
        field: Some("project_id".to_string()),
        location: ErrorLocation::from(Location::caller()),
    })?;

    // 2. Authorization - View permission required
    db_read(&ctx, "check_permission", || async {
        check_permission(&ctx, project_id, Permission::View).await
    })
    .await?;

    // 3. Fetch work items with circuit breaker
    let work_items = db_read(&ctx, "find_work_items", || async {
        WorkItemRepository::find_by_project(&ctx.pool, project_id, true)
            .await
            .map_err(WsError::from)
    })
    .await?;

    info!(
        count = work_items.len(),
        project_id = %project_id,
        "{} Found work items",
        ctx.log_prefix(),
    );

    // Pass &work_items twice: this handler fetches ALL project items via
    // find_by_project(pool, project_id, true) — no filtering applied.
    // The response set and the hierarchy computation set are identical.
    // PONE-172's debug_assert validates this subset contract at dev time.
    Ok(build_work_items_list_response(
        &ctx.message_id,
        &work_items,
        &work_items,
        chrono::Utc::now().timestamp(),
    ))
}
