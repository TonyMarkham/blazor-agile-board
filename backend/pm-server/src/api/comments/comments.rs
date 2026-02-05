//! Comment REST API handlers

use crate::{
    ApiError, ApiResult, CommentDto, CommentListResponse, CommentResponse, CreateCommentRequest,
    DeleteResponse, UpdateCommentRequest, UserId,
};

use pm_core::{ActivityLog, Comment};
use pm_db::{ActivityLogRepository, CommentRepository, WorkItemRepository};
use pm_ws::{AppState, MessageValidator, build_activity_log_created_event, sanitize_string};

use std::panic::Location;

use axum::{
    Json,
    extract::{Path, State, ws::Message},
};
use chrono::Utc;
use error_location::ErrorLocation;
use prost::Message as ProstMessage;
use uuid::Uuid;

/// GET /api/v1/work-items/:work_item_id/comments
pub async fn list_comments(
    State(state): State<AppState>,
    Path(work_item_id): Path<String>,
) -> ApiResult<Json<CommentListResponse>> {
    let work_item_uuid = Uuid::parse_str(&work_item_id)?;

    // Verify work item exists
    WorkItemRepository::find_by_id(&state.pool, work_item_uuid)
        .await?
        .ok_or_else(|| ApiError::NotFound {
            message: format!("Work item {} not found", work_item_id),
            location: ErrorLocation::from(Location::caller()),
        })?;

    let repo = CommentRepository::new(state.pool.clone());
    let comments = repo.find_by_work_item(work_item_uuid).await?;

    Ok(Json(CommentListResponse {
        comments: comments.into_iter().map(CommentDto::from).collect(),
    }))
}

/// POST /api/v1/work-items/:work_item_id/comments
pub async fn create_comment(
    State(state): State<AppState>,
    UserId(user_id): UserId,
    Path(work_item_id): Path<String>,
    Json(req): Json<CreateCommentRequest>,
) -> ApiResult<Json<CommentResponse>> {
    let work_item_uuid = Uuid::parse_str(&work_item_id)?;

    // 1. Validate content
    MessageValidator::validate_comment_create(&req.content).map_err(|e| ApiError::Validation {
        message: e.to_string(),
        field: Some("content".into()),
        location: ErrorLocation::from(Location::caller()),
    })?;

    // 2. Verify work item exists and get project_id for broadcast
    let work_item = WorkItemRepository::find_by_id(&state.pool, work_item_uuid)
        .await?
        .ok_or_else(|| ApiError::NotFound {
            message: format!("Work item {} not found", work_item_id),
            location: ErrorLocation::from(Location::caller()),
        })?;

    // 3. Create comment
    let comment = Comment::new(work_item_uuid, sanitize_string(&req.content), user_id);

    // 4. Save to database
    let activity = ActivityLog::created("comment", comment.id, user_id);
    let comment_clone = comment.clone();
    let activity_clone = activity.clone();

    let repo = CommentRepository::new(state.pool.clone());
    repo.create(&comment_clone).await?;
    ActivityLogRepository::create(&state.pool, &activity_clone).await?;

    // 5. Broadcast to WebSocket clients
    let event = build_activity_log_created_event(&activity);
    let bytes = event.encode_to_vec();
    let message = Message::Binary(bytes.into());
    if let Err(e) = state
        .registry
        .broadcast_activity_log_created(
            &work_item.project_id.to_string(),
            Some(&work_item_uuid.to_string()),
            None,
            message,
        )
        .await
    {
        log::warn!(
            "Failed to broadcast comment creation to WebSocket clients: {}",
            e
        );
    }

    log::info!(
        "Created comment {} on work item {} via REST API",
        comment.id,
        work_item_id
    );

    Ok(Json(CommentResponse {
        comment: comment.into(),
    }))
}

/// PUT /api/v1/comments/:id
pub async fn update_comment(
    State(state): State<AppState>,
    UserId(user_id): UserId,
    Path(comment_id): Path<String>,
    Json(req): Json<UpdateCommentRequest>,
) -> ApiResult<Json<CommentResponse>> {
    let comment_uuid = Uuid::parse_str(&comment_id)?;

    // 1. Validate content
    MessageValidator::validate_comment_create(&req.content).map_err(|e| ApiError::Validation {
        message: e.to_string(),
        field: Some("content".into()),
        location: ErrorLocation::from(Location::caller()),
    })?;

    // 2. Fetch existing comment
    let repo = CommentRepository::new(state.pool.clone());
    let mut comment = repo
        .find_by_id(comment_uuid)
        .await?
        .ok_or_else(|| ApiError::NotFound {
            message: format!("Comment {} not found", comment_id),
            location: ErrorLocation::from(Location::caller()),
        })?;

    // 3. Get work item for broadcast
    let work_item = WorkItemRepository::find_by_id(&state.pool, comment.work_item_id)
        .await?
        .ok_or_else(|| ApiError::Internal {
            message: "Work item not found for comment".into(),
            location: ErrorLocation::from(Location::caller()),
        })?;

    // 4. Update comment
    comment.content = sanitize_string(&req.content);
    comment.updated_at = Utc::now();
    comment.updated_by = user_id;

    // 5. Save and create activity
    let activity = ActivityLog::updated("comment", comment.id, user_id, &[]);
    let activity_clone = activity.clone();
    repo.update(&comment).await?;
    ActivityLogRepository::create(&state.pool, &activity_clone).await?;

    // 6. Broadcast
    let event = build_activity_log_created_event(&activity);
    let bytes = event.encode_to_vec();
    let message = Message::Binary(bytes.into());
    if let Err(e) = state
        .registry
        .broadcast_activity_log_created(
            &work_item.project_id.to_string(),
            Some(&comment.work_item_id.to_string()),
            None,
            message,
        )
        .await
    {
        log::warn!(
            "Failed to broadcast comment update to WebSocket clients: {}",
            e
        );
    }

    log::info!("Updated comment {} via REST API", comment_uuid);

    Ok(Json(CommentResponse {
        comment: comment.into(),
    }))
}

/// DELETE /api/v1/comments/:id
pub async fn delete_comment(
    State(state): State<AppState>,
    UserId(user_id): UserId,
    Path(comment_id): Path<String>,
) -> ApiResult<Json<DeleteResponse>> {
    let comment_uuid = Uuid::parse_str(&comment_id)?;

    // 1. Fetch existing comment
    let repo = CommentRepository::new(state.pool.clone());
    let comment = repo
        .find_by_id(comment_uuid)
        .await?
        .ok_or_else(|| ApiError::NotFound {
            message: format!("Comment {} not found", comment_id),
            location: ErrorLocation::from(Location::caller()),
        })?;

    // 2. Get work item for broadcast
    let work_item = WorkItemRepository::find_by_id(&state.pool, comment.work_item_id)
        .await?
        .ok_or_else(|| ApiError::Internal {
            message: "Work item not found for comment".into(),
            location: ErrorLocation::from(Location::caller()),
        })?;

    // 3. Soft delete and create activity
    let activity = ActivityLog::deleted("comment", comment_uuid, user_id);
    let activity_clone = activity.clone();
    let now = Utc::now().timestamp();

    // NOTE: CommentRepository.delete() signature is: delete(id: Uuid, deleted_at: i64)
    // Unlike other repositories, it only tracks deletion timestamp (user_id in ActivityLog)
    repo.delete(comment_uuid, now).await?;
    ActivityLogRepository::create(&state.pool, &activity_clone).await?;

    // 4. Broadcast
    let event = build_activity_log_created_event(&activity);
    let bytes = event.encode_to_vec();
    let message = Message::Binary(bytes.into());
    if let Err(e) = state
        .registry
        .broadcast_activity_log_created(
            &work_item.project_id.to_string(),
            Some(&comment.work_item_id.to_string()),
            None,
            message,
        )
        .await
    {
        log::warn!(
            "Failed to broadcast comment deletion to WebSocket clients: {}",
            e
        );
    }

    log::info!("Deleted comment {} via REST API", comment_uuid);

    Ok(Json(DeleteResponse {
        deleted_id: comment_uuid.to_string(),
    }))
}
