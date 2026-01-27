#![allow(dead_code)]

use crate::{
    HandlerContext, MessageValidator, Result as WsErrorResult, WsError,
    build_comment_created_response, build_comment_deleted_response, build_comment_updated_response,
    build_comments_list_response, check_idempotency, check_permission, db_read, db_write,
    sanitize_string, store_idempotency,
};

use pm_core::{ActivityLog, Comment, Permission};
use pm_db::{ActivityLogRepository, CommentRepository, WorkItemRepository};
use pm_proto::{
    CreateCommentRequest, DeleteCommentRequest, GetCommentsRequest, UpdateCommentRequest,
    WebSocketMessage,
};

use std::panic::Location;

use base64::Engine;
use chrono::Utc;
use error_location::ErrorLocation;
use log::{debug, info, warn};
use prost::Message as ProstMessage;
use uuid::Uuid;

fn parse_uuid(s: &str, field: &str) -> WsErrorResult<Uuid> {
    Uuid::parse_str(s).map_err(|_| WsError::ValidationError {
        message: format!("Invalid UUID format for {}", field),
        field: Some(field.to_string()),
        location: ErrorLocation::from(Location::caller()),
    })
}

/// Handle CreateCommentRequest
///
/// Comments are attached to work items. Authorization is based on the
/// work item's project - if you can edit the project, you can comment.
pub async fn handle_create_comment(
    req: CreateCommentRequest,
    ctx: HandlerContext,
) -> WsErrorResult<WebSocketMessage> {
    debug!("{} CreateComment starting", ctx.log_prefix());

    // 1. Validate content
    MessageValidator::validate_comment_create(&req.content)?;

    // 2. Check idempotency
    let cached = db_read(&ctx, "check_idempotency", || async {
        check_idempotency(&ctx.pool, &ctx.message_id).await
    })
    .await?;

    if let Some(cached_response) = cached {
        info!("{} Returning cached idempotent response", ctx.log_prefix());
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(&cached_response)
            .map_err(|e| WsError::Internal {
                message: format!("Failed to decode cached response: {e}"),
                location: ErrorLocation::from(Location::caller()),
            })?;
        return WebSocketMessage::decode(&*bytes).map_err(|e| WsError::Internal {
            message: format!("Failed to decode cached protobuf: {e}"),
            location: ErrorLocation::from(Location::caller()),
        });
    }

    // 3. Parse work_item_id
    let work_item_id = parse_uuid(&req.work_item_id, "work_item_id")?;

    // 4. Verify work item exists and get project_id for authorization
    let work_item = db_read(&ctx, "find_work_item", || async {
        WorkItemRepository::find_by_id(&ctx.pool, work_item_id)
            .await
            .map_err(WsError::from)
    })
    .await?
    .ok_or_else(|| WsError::NotFound {
        message: format!("Work item {} not found", work_item_id),
        location: ErrorLocation::from(Location::caller()),
    })?;

    // 5. Authorization - Edit permission on the project
    db_read(&ctx, "check_permission", || async {
        check_permission(&ctx, work_item.project_id, Permission::Edit).await
    })
    .await?;

    // 6. Create comment
    let comment = Comment::new(work_item_id, sanitize_string(&req.content), ctx.user_id);

    // 7. Execute transaction
    let comment_clone = comment.clone();
    db_write(&ctx, "create_comment_tx", || async {
        let repo = CommentRepository::new(ctx.pool.clone());
        repo.create(&comment_clone).await?;

        let activity = ActivityLog::created("comment", comment_clone.id, ctx.user_id);
        ActivityLogRepository::create(&ctx.pool, &activity).await?;

        Ok::<_, WsError>(())
    })
    .await?;

    // 8. Build response
    let response = build_comment_created_response(&ctx.message_id, &comment, ctx.user_id);

    // 9. Store idempotency
    let response_bytes = response.encode_to_vec();
    let response_b64 = base64::engine::general_purpose::STANDARD.encode(&response_bytes);

    if let Err(e) =
        store_idempotency(&ctx.pool, &ctx.message_id, "create_comment", &response_b64).await
    {
        warn!(
            "{} Failed to store idempotency (non-fatal): {}",
            ctx.log_prefix(),
            e
        );
    }

    info!(
        "{} Created comment {} on work item {}",
        ctx.log_prefix(),
        comment.id,
        work_item_id
    );

    Ok(response)
}

/// Handle UpdateCommentRequest
///
/// IMPORTANT: Only the author can edit their own comments.
/// This is a common pattern for user-generated content.
pub async fn handle_update_comment(
    req: UpdateCommentRequest,
    ctx: HandlerContext,
) -> WsErrorResult<WebSocketMessage> {
    debug!("{} UpdateComment starting", ctx.log_prefix());

    // 1. Validate content
    MessageValidator::validate_comment_create(&req.content)?;

    // 2. Parse comment ID
    let comment_id = parse_uuid(&req.comment_id, "comment_id")?;

    // 3. Fetch existing
    let repo = CommentRepository::new(ctx.pool.clone());
    let mut comment = db_read(&ctx, "find_comment", || async {
        repo.find_by_id(comment_id).await.map_err(WsError::from)
    })
    .await?
    .ok_or_else(|| WsError::NotFound {
        message: format!("Comment {} not found", comment_id),
        location: ErrorLocation::from(Location::caller()),
    })?;

    // 4. AUTHOR-ONLY: Only the original author can edit their comment
    if comment.created_by != ctx.user_id {
        return Err(WsError::Unauthorized {
            message: "Cannot edit another user's comment".to_string(),
            location: ErrorLocation::from(Location::caller()),
        });
    }

    // 5. Apply update
    comment.content = sanitize_string(&req.content);
    comment.updated_at = Utc::now();
    comment.updated_by = ctx.user_id;

    // 6. Transaction
    let comment_clone = comment.clone();
    db_write(&ctx, "update_comment_tx", || async {
        let repo = CommentRepository::new(ctx.pool.clone());
        repo.update(&comment_clone).await?;

        let activity = ActivityLog::updated("comment", comment_clone.id, ctx.user_id, &[]);
        ActivityLogRepository::create(&ctx.pool, &activity).await?;

        Ok::<_, WsError>(())
    })
    .await?;

    info!("{} Updated comment {}", ctx.log_prefix(), comment.id);

    Ok(build_comment_updated_response(
        &ctx.message_id,
        &comment,
        ctx.user_id,
    ))
}

/// Handle DeleteCommentRequest
///
/// IMPORTANT: Only the author can delete their own comments.
/// Uses soft delete to preserve audit trail.
pub async fn handle_delete_comment(
    req: DeleteCommentRequest,
    ctx: HandlerContext,
) -> WsErrorResult<WebSocketMessage> {
    debug!("{} DeleteComment starting", ctx.log_prefix());

    let comment_id = parse_uuid(&req.comment_id, "comment_id")?;

    let repo = CommentRepository::new(ctx.pool.clone());
    let comment = db_read(&ctx, "find_comment", || async {
        repo.find_by_id(comment_id).await.map_err(WsError::from)
    })
    .await?
    .ok_or_else(|| WsError::NotFound {
        message: format!("Comment {} not found", comment_id),
        location: ErrorLocation::from(Location::caller()),
    })?;

    // AUTHOR-ONLY: Only the original author can delete their comment
    if comment.created_by != ctx.user_id {
        return Err(WsError::Unauthorized {
            message: "Cannot delete another user's comment".to_string(),
            location: ErrorLocation::from(Location::caller()),
        });
    }

    // Soft delete
    db_write(&ctx, "delete_comment_tx", || async {
        let repo = CommentRepository::new(ctx.pool.clone());
        repo.delete(comment_id, Utc::now().timestamp()).await?;

        let activity = ActivityLog::deleted("comment", comment_id, ctx.user_id);
        ActivityLogRepository::create(&ctx.pool, &activity).await?;

        Ok::<_, WsError>(())
    })
    .await?;

    info!("{} Deleted comment {}", ctx.log_prefix(), comment_id);

    Ok(build_comment_deleted_response(
        &ctx.message_id,
        comment_id,
        ctx.user_id,
    ))
}

/// Handle GetCommentsRequest - list comments for a work item
pub async fn handle_get_comments(
    req: GetCommentsRequest,
    ctx: HandlerContext,
) -> WsErrorResult<WebSocketMessage> {
    debug!("{} GetComments starting", ctx.log_prefix());

    let work_item_id = parse_uuid(&req.work_item_id, "work_item_id")?;

    // Verify work item exists and get project_id for authorization
    let work_item = db_read(&ctx, "find_work_item", || async {
        WorkItemRepository::find_by_id(&ctx.pool, work_item_id)
            .await
            .map_err(WsError::from)
    })
    .await?
    .ok_or_else(|| WsError::NotFound {
        message: format!("Work item {} not found", work_item_id),
        location: ErrorLocation::from(Location::caller()),
    })?;

    // Authorization - View permission on the project
    db_read(&ctx, "check_permission", || async {
        check_permission(&ctx, work_item.project_id, Permission::View).await
    })
    .await?;

    let repo = CommentRepository::new(ctx.pool.clone());
    let comments = db_read(&ctx, "find_comments", || async {
        repo.find_by_work_item(work_item_id)
            .await
            .map_err(WsError::from)
    })
    .await?;

    info!(
        "{} Found {} comments for work item {}",
        ctx.log_prefix(),
        comments.len(),
        work_item_id
    );

    Ok(build_comments_list_response(&ctx.message_id, comments))
}
