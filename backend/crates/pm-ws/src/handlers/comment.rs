#![allow(dead_code)]

use crate::{
    HandlerContext, MessageValidator, Result as WsErrorResult, WsError,
    build_activity_log_created_event, build_comment_created_response,
    build_comment_deleted_response, build_comment_updated_response, build_comments_list_response,
    check_idempotency, check_permission, db_read, db_write, sanitize_string, store_idempotency,
};

use pm_core::{ActivityLog, Comment, Permission};
use pm_db::{ActivityLogRepository, CommentRepository, WorkItemRepository};
use pm_proto::{
    CreateCommentRequest, DeleteCommentRequest, GetCommentsRequest, UpdateCommentRequest,
    WebSocketMessage,
};

use std::panic::Location;

use axum::extract::ws::Message;
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
    let activity = ActivityLog::created("comment", comment.id, ctx.user_id);
    let comment_clone = comment.clone();
    let activity_clone = activity.clone();
    db_write(&ctx, "create_comment_tx", || async {
        let repo = CommentRepository::new(ctx.pool.clone());
        repo.create(&comment_clone).await?;
        ActivityLogRepository::create(&ctx.pool, &activity_clone).await?;
        Ok::<_, WsError>(())
    })
    .await?;

    // 8. Broadcast ActivityLogCreated
    let event = build_activity_log_created_event(&activity);
    let bytes = event.encode_to_vec();
    let message = Message::Binary(bytes.into());
    let project_id_str = work_item.project_id.to_string();
    let work_item_id_str = work_item.id.to_string();
    ctx.registry
        .broadcast_activity_log_created(&project_id_str, Some(&work_item_id_str), None, message)
        .await?;

    // 8b. Broadcast CommentCreated to all project subscribers
    let broadcast =
        build_comment_created_response(&Uuid::new_v4().to_string(), &comment, ctx.user_id);
    let broadcast_bytes = broadcast.encode_to_vec();
    if let Err(e) = ctx
        .registry
        .broadcast_to_project(&project_id_str, Message::Binary(broadcast_bytes.into()))
        .await
    {
        warn!(
            "{} Failed to broadcast CommentCreated: {}",
            ctx.log_prefix(),
            e
        );
    }

    // 9. Build response
    let response = build_comment_created_response(&ctx.message_id, &comment, ctx.user_id);

    // 10. Store idempotency
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

    // 4b. Fetch work item for project_id (needed for broadcast)
    let work_item = db_read(&ctx, "find_work_item_for_comment", || async {
        WorkItemRepository::find_by_id(&ctx.pool, comment.work_item_id)
            .await
            .map_err(WsError::from)
    })
    .await?
    .ok_or_else(|| WsError::NotFound {
        message: format!("Work item {} not found", comment.work_item_id),
        location: ErrorLocation::from(Location::caller()),
    })?;

    // 5. Apply update
    comment.content = sanitize_string(&req.content);
    comment.updated_at = Utc::now();
    comment.updated_by = ctx.user_id;

    // 6. Transaction
    let activity = ActivityLog::updated("comment", comment.id, ctx.user_id, &[]);
    let comment_clone = comment.clone();
    let activity_clone = activity.clone();
    db_write(&ctx, "update_comment_tx", || async {
        let repo = CommentRepository::new(ctx.pool.clone());
        repo.update(&comment_clone).await?;
        ActivityLogRepository::create(&ctx.pool, &activity_clone).await?;
        Ok::<_, WsError>(())
    })
    .await?;

    // 7. Broadcast ActivityLogCreated
    let event = build_activity_log_created_event(&activity);
    let bytes = event.encode_to_vec();
    let message = Message::Binary(bytes.into());
    let project_id_str = work_item.project_id.to_string();
    let work_item_id_str = work_item.id.to_string();
    ctx.registry
        .broadcast_activity_log_created(&project_id_str, Some(&work_item_id_str), None, message)
        .await?;

    // 7b. Broadcast CommentUpdated to all project subscribers
    let broadcast =
        build_comment_updated_response(&Uuid::new_v4().to_string(), &comment, ctx.user_id);
    let broadcast_bytes = broadcast.encode_to_vec();
    if let Err(e) = ctx
        .registry
        .broadcast_to_project(&project_id_str, Message::Binary(broadcast_bytes.into()))
        .await
    {
        warn!(
            "{} Failed to broadcast CommentUpdated: {}",
            ctx.log_prefix(),
            e
        );
    }

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

    // Fetch work item for project_id (needed for broadcast)
    let work_item = db_read(&ctx, "find_work_item_for_comment", || async {
        WorkItemRepository::find_by_id(&ctx.pool, comment.work_item_id)
            .await
            .map_err(WsError::from)
    })
    .await?
    .ok_or_else(|| WsError::NotFound {
        message: format!("Work item {} not found", comment.work_item_id),
        location: ErrorLocation::from(Location::caller()),
    })?;

    // Soft delete
    let activity = ActivityLog::deleted("comment", comment_id, ctx.user_id);
    let activity_clone = activity.clone();
    db_write(&ctx, "delete_comment_tx", || async {
        let repo = CommentRepository::new(ctx.pool.clone());
        repo.delete(comment_id, Utc::now().timestamp()).await?;
        ActivityLogRepository::create(&ctx.pool, &activity_clone).await?;
        Ok::<_, WsError>(())
    })
    .await?;

    // Broadcast ActivityLogCreated
    let event = build_activity_log_created_event(&activity);
    let bytes = event.encode_to_vec();
    let message = Message::Binary(bytes.into());
    let project_id_str = work_item.project_id.to_string();
    let work_item_id_str = work_item.id.to_string();
    ctx.registry
        .broadcast_activity_log_created(&project_id_str, Some(&work_item_id_str), None, message)
        .await?;

    // Broadcast CommentDeleted to all project subscribers
    let broadcast =
        build_comment_deleted_response(&Uuid::new_v4().to_string(), comment_id, ctx.user_id);
    let broadcast_bytes = broadcast.encode_to_vec();
    if let Err(e) = ctx
        .registry
        .broadcast_to_project(&project_id_str, Message::Binary(broadcast_bytes.into()))
        .await
    {
        warn!(
            "{} Failed to broadcast CommentDeleted: {}",
            ctx.log_prefix(),
            e
        );
    }

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
