//! Work Item REST API handlers
//!
//! These handlers provide HTTP access to work items and broadcast changes
//! via WebSocket so connected clients see updates in real-time.

use crate::{
    ApiError, ApiResult, CreateWorkItemRequest, DeleteResponse, ListWorkItemsQuery,
    UpdateWorkItemRequest, UserId, WorkItemDto, WorkItemListResponse, WorkItemResponse,
};

use pm_core::{ActivityLog, WorkItem, WorkItemType};
use pm_db::{ActivityLogRepository, ProjectRepository, WorkItemRepository};
use pm_ws::{
    AppState, MessageValidator, build_activity_log_created_event, build_work_item_created_response,
    build_work_item_deleted_response, build_work_item_updated_response, sanitize_string,
    validate_hierarchy, validate_priority, validate_status,
};

use axum::{
    Json,
    extract::{Path, Query, State, ws::Message},
};
use chrono::Utc;
use error_location::ErrorLocation;
use prost::Message as ProstMessage;
use std::panic::Location;
use std::str::FromStr;
use uuid::Uuid;

// =============================================================================
// Handlers
// =============================================================================

/// GET /api/v1/work-items/:id
///
/// Retrieve a single work item by ID
pub async fn get_work_item(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<WorkItemResponse>> {
    let work_item_id = Uuid::parse_str(&id)?;

    let work_item = WorkItemRepository::find_by_id(&state.pool, work_item_id)
        .await?
        .ok_or_else(|| ApiError::NotFound {
            message: format!("Work item {} not found", id),
            location: ErrorLocation::from(Location::caller()),
        })?;

    // Get project key for display_key
    let repo = ProjectRepository::new(state.pool.clone());
    let project = repo
        .find_by_id(work_item.project_id)
        .await?
        .ok_or_else(|| ApiError::Internal {
            message: "Project not found for work item".to_string(),
            location: ErrorLocation::from(Location::caller()),
        })?;

    Ok(Json(WorkItemResponse {
        work_item: WorkItemDto::from_work_item(work_item, &project.key),
    }))
}

/// GET /api/v1/projects/:project_id/work-items
///
/// List work items in a project with optional filters
#[allow(clippy::unnecessary_map_or)]
pub async fn list_work_items(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    Query(query): Query<ListWorkItemsQuery>,
) -> ApiResult<Json<WorkItemListResponse>> {
    let project_uuid = Uuid::parse_str(&project_id)?;

    // Get project for key
    let repo = ProjectRepository::new(state.pool.clone());
    let project = repo
        .find_by_id(project_uuid)
        .await?
        .ok_or_else(|| ApiError::NotFound {
            message: format!("Project {} not found", project_id),
            location: ErrorLocation::from(Location::caller()),
        })?;

    let work_items = WorkItemRepository::find_by_project(&state.pool, project_uuid).await?;

    // Apply filters and convert to DTOs
    let filtered: Vec<WorkItemDto> = work_items
        .into_iter()
        .filter(|w| {
            query
                .item_type
                .as_ref()
                .map_or(true, |t| w.item_type.as_str() == t)
                && query.status.as_ref().map_or(true, |s| &w.status == s)
                && query.sprint_id.as_ref().map_or(true, |sid| {
                    w.sprint_id.map_or(false, |ws| ws.to_string() == *sid)
                })
        })
        .map(|w| WorkItemDto::from_work_item(w, &project.key))
        .collect();

    Ok(Json(WorkItemListResponse {
        work_items: filtered,
    }))
}

/// POST /api/v1/work-items
///
/// Create a new work item. Broadcasts activity to WebSocket clients.
pub async fn create_work_item(
    State(state): State<AppState>,
    UserId(user_id): UserId,
    Json(req): Json<CreateWorkItemRequest>,
) -> ApiResult<Json<WorkItemResponse>> {
    // 1. Parse and validate item_type
    let item_type = WorkItemType::from_str(&req.item_type).map_err(|_| ApiError::Validation {
        message: format!(
            "Invalid item_type: {}. Valid values: epic, story, task",
            req.item_type
        ),
        location: ErrorLocation::from(Location::caller()),
        field: Some("item_type".into()),
    })?;

    // 2. Validate input fields
    MessageValidator::validate_work_item_create(
        &req.title,
        req.description.as_deref(),
        item_type.as_str(),
    )
    .map_err(|e| ApiError::Validation {
        message: e.to_string(),
        field: None,
        location: ErrorLocation::from(Location::caller()),
    })?;

    // 3. Parse IDs
    let project_id = Uuid::parse_str(&req.project_id)?;
    let parent_id = req
        .parent_id
        .as_ref()
        .filter(|s| !s.is_empty())
        .map(|s| Uuid::parse_str(s))
        .transpose()?;

    // 4. Validate hierarchy if parent specified
    if let Some(pid) = parent_id {
        validate_hierarchy(&state.pool, item_type.clone(), pid)
            .await
            .map_err(|e| ApiError::Validation {
                message: e.to_string(),
                field: Some("parent_id".into()),
                location: ErrorLocation::from(Location::caller()),
            })?;
    }

    // 5. Get project (for key and item number)
    let repo = ProjectRepository::new(state.pool.clone());
    let project = repo
        .find_by_id(project_id)
        .await?
        .ok_or_else(|| ApiError::NotFound {
            message: format!("Project {} not found", project_id),
            location: ErrorLocation::from(Location::caller()),
        })?;

    // 6. Get next position
    let max_position =
        WorkItemRepository::find_max_position(&state.pool, project_id, parent_id).await?;

    // 7. Build work item
    let now = Utc::now();
    let mut work_item = WorkItem {
        id: Uuid::new_v4(),
        item_type,
        parent_id,
        project_id,
        position: max_position + 1,
        title: sanitize_string(&req.title),
        description: req.description.as_ref().map(|d| sanitize_string(d)),
        status: req.status.unwrap_or_else(|| "backlog".to_string()),
        priority: req.priority.unwrap_or_else(|| "medium".to_string()),
        assignee_id: None,
        story_points: None,
        sprint_id: None,
        item_number: 0, // Will be set by transaction
        version: 1,
        created_at: now,
        updated_at: now,
        created_by: user_id,
        updated_by: user_id,
        deleted_at: None,
    };

    // 8. Execute transaction
    let activity = ActivityLog::created("work_item", work_item.id, user_id);
    let activity_clone = activity.clone();
    let work_item_for_tx = work_item.clone();

    let mut tx = state.pool.begin().await?;

    // Get and increment item number atomically
    let item_number = ProjectRepository::get_and_increment_work_item_number(&mut tx, project_id)
        .await
        .map_err(|e| ApiError::Internal {
            message: e.to_string(),
            location: ErrorLocation::from(Location::caller()),
        })?;

    work_item.item_number = item_number;
    let mut wi_to_insert = work_item_for_tx;
    wi_to_insert.item_number = item_number;

    WorkItemRepository::create(&mut *tx, &wi_to_insert).await?;
    ActivityLogRepository::create(&mut *tx, &activity_clone).await?;
    tx.commit().await?;

    // 9. Broadcast to WebSocket clients
    let event = build_activity_log_created_event(&activity);
    let bytes = event.encode_to_vec();
    let message = Message::Binary(bytes.into());
    if let Err(e) = state
        .registry
        .broadcast_activity_log_created(
            &project_id.to_string(),
            Some(&work_item.id.to_string()),
            None,
            message,
        )
        .await
    {
        log::warn!(
            "Failed to broadcast work item creation to WebSocket clients: {}",
            e
        );
        // This is OK - database operation succeeded, UI will update on next refresh
    }

    // 9b. Broadcast WorkItemCreated to all project subscribers
    let broadcast =
        build_work_item_created_response(&Uuid::new_v4().to_string(), &work_item, user_id);
    let broadcast_bytes = broadcast.encode_to_vec();
    if let Err(e) = state
        .registry
        .broadcast_to_project(
            &project_id.to_string(),
            Message::Binary(broadcast_bytes.into()),
        )
        .await
    {
        log::warn!("Failed to broadcast WorkItemCreated via REST: {}", e);
    }

    log::info!(
        "Created work item {} ({}) via REST API",
        work_item.id,
        work_item.item_number
    );

    Ok(Json(WorkItemResponse {
        work_item: WorkItemDto::from_work_item(work_item, &project.key),
    }))
}

/// PUT /api/v1/work-items/:id
///
/// Update an existing work item. Uses optimistic locking via expected_version.
pub async fn update_work_item(
    State(state): State<AppState>,
    UserId(user_id): UserId,
    Path(id): Path<String>,
    Json(req): Json<UpdateWorkItemRequest>,
) -> ApiResult<Json<WorkItemResponse>> {
    let work_item_id = Uuid::parse_str(&id)?;

    // 1. Fetch existing work item
    let mut work_item = WorkItemRepository::find_by_id(&state.pool, work_item_id)
        .await?
        .ok_or_else(|| ApiError::NotFound {
            message: format!("Work item {} not found", id),
            location: ErrorLocation::from(Location::caller()),
        })?;

    // 2. Check version (optimistic locking)
    if work_item.version != req.expected_version {
        return Err(ApiError::Conflict {
            message: "Version mismatch - work item was modified by another user".into(),
            current_version: work_item.version,
            location: ErrorLocation::from(Location::caller()),
        });
    }

    // 3. Get project for response
    let repo = ProjectRepository::new(state.pool.clone());
    let project = repo
        .find_by_id(work_item.project_id)
        .await?
        .ok_or_else(|| ApiError::Internal {
            message: "Project not found".to_string(),
            location: ErrorLocation::from(Location::caller()),
        })?;

    // 4. Apply updates with validation
    if let Some(ref title) = req.title {
        MessageValidator::validate_string(title, "title", 1, 200).map_err(|e| {
            ApiError::Validation {
                message: e.to_string(),
                field: Some("title".into()),
                location: ErrorLocation::from(Location::caller()),
            }
        })?;
        work_item.title = sanitize_string(title);
    }
    if let Some(ref desc) = req.description {
        work_item.description = Some(sanitize_string(desc));
    }
    if let Some(ref status) = req.status {
        validate_status(status).map_err(|e| ApiError::Validation {
            message: e.to_string(),
            field: Some("status".into()),
            location: ErrorLocation::from(Location::caller()),
        })?;
        work_item.status = status.clone();
    }
    if let Some(ref priority) = req.priority {
        validate_priority(priority).map_err(|e| ApiError::Validation {
            message: e.to_string(),
            field: Some("priority".into()),
            location: ErrorLocation::from(Location::caller()),
        })?;
        work_item.priority = priority.clone();
    }
    if let Some(ref assignee_id) = req.assignee_id {
        work_item.assignee_id = if assignee_id.is_empty() {
            None
        } else {
            Some(Uuid::parse_str(assignee_id)?)
        };
    }
    if let Some(ref sprint_id) = req.sprint_id {
        work_item.sprint_id = if sprint_id.is_empty() {
            None
        } else {
            Some(Uuid::parse_str(sprint_id)?)
        };
    }
    if let Some(sp) = req.story_points {
        work_item.story_points = Some(sp);
    }
    if req.update_parent {
        work_item.parent_id = req
            .parent_id
            .as_ref()
            .filter(|s| !s.is_empty())
            .map(|s| Uuid::parse_str(s))
            .transpose()?;
    }

    // 5. Update metadata
    work_item.updated_at = Utc::now();
    work_item.updated_by = user_id;
    work_item.version += 1;

    // 6. Execute transaction
    let activity = ActivityLog::updated("work_item", work_item.id, user_id, &[]);
    let work_item_clone = work_item.clone();
    let activity_clone = activity.clone();

    let mut tx = state.pool.begin().await?;
    WorkItemRepository::update(&mut *tx, &work_item_clone).await?;
    ActivityLogRepository::create(&mut *tx, &activity_clone).await?;
    tx.commit().await?;

    // 7. Broadcast to WebSocket clients
    let event = build_activity_log_created_event(&activity);
    let bytes = event.encode_to_vec();
    let message = Message::Binary(bytes.into());
    if let Err(e) = state
        .registry
        .broadcast_activity_log_created(
            &work_item.project_id.to_string(),
            Some(&work_item.id.to_string()),
            None,
            message,
        )
        .await
    {
        log::warn!(
            "Failed to broadcast work item update to WebSocket clients: {}",
            e
        );
        // This is OK - database operation succeeded, UI will update on next refresh
    }

    // 7b. Broadcast WorkItemUpdated to all project subscribers
    let broadcast = build_work_item_updated_response(
        &Uuid::new_v4().to_string(),
        &work_item,
        &[], // REST API doesn't track field changes currently
        user_id,
    );
    let broadcast_bytes = broadcast.encode_to_vec();
    if let Err(e) = state
        .registry
        .broadcast_to_project(
            &work_item.project_id.to_string(),
            Message::Binary(broadcast_bytes.into()),
        )
        .await
    {
        log::warn!("Failed to broadcast WorkItemUpdated via REST: {}", e);
    }

    log::info!(
        "Updated work item {} to version {} via REST API",
        work_item.id,
        work_item.version
    );

    Ok(Json(WorkItemResponse {
        work_item: WorkItemDto::from_work_item(work_item, &project.key),
    }))
}

/// DELETE /api/v1/work-items/:id
///
/// Soft-delete a work item. Fails if work item has children.
pub async fn delete_work_item(
    State(state): State<AppState>,
    UserId(user_id): UserId,
    Path(id): Path<String>,
) -> ApiResult<Json<DeleteResponse>> {
    let work_item_id = Uuid::parse_str(&id)?;

    // 1. Fetch existing work item
    let work_item = WorkItemRepository::find_by_id(&state.pool, work_item_id)
        .await?
        .ok_or_else(|| ApiError::NotFound {
            message: format!("Work item {} not found", id),
            location: ErrorLocation::from(Location::caller()),
        })?;

    // 2. Check for children
    let children = WorkItemRepository::find_children(&state.pool, work_item_id).await?;
    if !children.is_empty() {
        return Err(ApiError::Validation {
            message: format!(
                "Cannot delete work item with {} child item(s). Delete children first.",
                children.len()
            ),
            field: None,
            location: ErrorLocation::from(Location::caller()),
        });
    }

    // 3. Execute transaction (soft delete)
    let activity = ActivityLog::deleted("work_item", work_item_id, user_id);
    let activity_clone = activity.clone();

    let mut tx = state.pool.begin().await?;
    WorkItemRepository::soft_delete(&mut *tx, work_item_id, user_id).await?;
    ActivityLogRepository::create(&mut *tx, &activity_clone).await?;
    tx.commit().await?;

    // 4. Broadcast to WebSocket clients
    let event = build_activity_log_created_event(&activity);
    let bytes = event.encode_to_vec();
    let message = Message::Binary(bytes.into());
    if let Err(e) = state
        .registry
        .broadcast_activity_log_created(
            &work_item.project_id.to_string(),
            Some(&work_item_id.to_string()),
            None,
            message,
        )
        .await
    {
        log::warn!(
            "Failed to broadcast work item deletion to WebSocket clients: {}",
            e
        );
        // This is OK - database operation succeeded, UI will update on next refresh
    }

    // 4b. Broadcast WorkItemDeleted to all project subscribers
    let broadcast =
        build_work_item_deleted_response(&Uuid::new_v4().to_string(), work_item_id, user_id);
    let broadcast_bytes = broadcast.encode_to_vec();
    if let Err(e) = state
        .registry
        .broadcast_to_project(
            &work_item.project_id.to_string(),
            Message::Binary(broadcast_bytes.into()),
        )
        .await
    {
        log::warn!("Failed to broadcast WorkItemDeleted via REST: {}", e);
    }

    log::info!("Deleted work item {} via REST API", work_item_id);

    Ok(Json(DeleteResponse {
        deleted_id: work_item_id.to_string(),
    }))
}
