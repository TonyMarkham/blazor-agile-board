//! Time Entry REST API handlers
//!
//! These handlers provide HTTP access to time entries and broadcast changes
//! via WebSocket so connected clients see updates in real-time.

use crate::{
    ApiError, ApiResult, CreateTimeEntryRequest, DeleteResponse, TimeEntryDto,
    TimeEntryListResponse, TimeEntryResponse, UpdateTimeEntryRequest, UserId,
};

use pm_core::{ActivityLog, TimeEntry};
use pm_db::{ActivityLogRepository, TimeEntryRepository, WorkItemRepository};
use pm_ws::{
    AppState, build_activity_log_created_event, build_time_entry_created_response,
    build_time_entry_deleted_response, build_time_entry_updated_response, sanitize_string,
};

use std::panic::Location;

use axum::{
    Json,
    extract::{Path, State, ws::Message},
};
use chrono::Utc;
use error_location::ErrorLocation;
use prost::Message as ProstMessage;
use uuid::Uuid;

// =============================================================================
// Handlers
// =============================================================================

/// GET /api/v1/work-items/:id/time-entries
///
/// List all time entries for a work item, ordered by started_at DESC (newest first)
pub async fn list_time_entries(
    State(state): State<AppState>,
    Path(work_item_id): Path<String>,
) -> ApiResult<Json<TimeEntryListResponse>> {
    let work_item_uuid = Uuid::parse_str(&work_item_id)?;

    let repo = TimeEntryRepository::new(state.pool.clone());
    let entries = repo.find_by_work_item(work_item_uuid).await?;

    Ok(Json(TimeEntryListResponse {
        time_entries: entries.into_iter().map(TimeEntryDto::from).collect(),
    }))
}

/// GET /api/v1/time-entries/:id
///
/// Get a single time entry by ID
pub async fn get_time_entry(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<TimeEntryResponse>> {
    let time_entry_id = Uuid::parse_str(&id)?;

    let repo = TimeEntryRepository::new(state.pool.clone());
    let entry = repo
        .find_by_id(time_entry_id)
        .await?
        .ok_or_else(|| ApiError::NotFound {
            message: format!("Time entry {} not found", id),
            location: ErrorLocation::from(Location::caller()),
        })?;

    Ok(Json(TimeEntryResponse {
        time_entry: entry.into(),
    }))
}

/// POST /api/v1/time-entries
///
/// Create a new time entry (start a timer). Broadcasts activity to WebSocket clients.
pub async fn create_time_entry(
    State(state): State<AppState>,
    UserId(user_id): UserId,
    Json(req): Json<CreateTimeEntryRequest>,
) -> ApiResult<Json<TimeEntryResponse>> {
    // 1. Parse and validate work_item_id
    let work_item_id = Uuid::parse_str(&req.work_item_id).map_err(|_| ApiError::Validation {
        message: format!("Invalid work_item_id: '{}'", req.work_item_id),
        field: Some("work_item_id".into()),
        location: ErrorLocation::from(Location::caller()),
    })?;

    // 2. Validate work item exists and get project_id for broadcasts
    let work_item = WorkItemRepository::find_by_id(&state.pool, work_item_id)
        .await?
        .ok_or_else(|| ApiError::NotFound {
            message: format!("Work item {} not found", req.work_item_id),
            location: ErrorLocation::from(Location::caller()),
        })?;

    let project_id = work_item.project_id;

    // 3. Sanitize optional description
    let description = req.description.as_ref().and_then(|d| {
        let sanitized = sanitize_string(d);
        if sanitized.trim().is_empty() {
            None
        } else {
            Some(sanitized)
        }
    });

    // 4. Create time entry (auto-sets started_at to now)
    let time_entry = TimeEntry::new(work_item_id, user_id, description);

    // 5. Execute transaction
    let activity = ActivityLog::created("time_entry", time_entry.id, user_id);
    let activity_clone = activity.clone();
    let time_entry_clone = time_entry.clone();

    let repo = TimeEntryRepository::new(state.pool.clone());
    let mut tx = state.pool.begin().await?;
    repo.create(&time_entry_clone).await?;
    ActivityLogRepository::create(&mut *tx, &activity_clone).await?;
    tx.commit().await?;

    // 6. Broadcast ActivityLogCreated
    let event = build_activity_log_created_event(&activity);
    let bytes = event.encode_to_vec();
    let message = Message::Binary(bytes.into());
    if let Err(e) = state
        .registry
        .broadcast_activity_log_created(
            &project_id.to_string(),
            Some(&work_item_id.to_string()),
            None,
            message,
        )
        .await
    {
        log::warn!(
            "Failed to broadcast time entry creation activity log to WebSocket clients: {}",
            e
        );
    }

    // 7. Broadcast TimeEntryCreated
    let broadcast =
        build_time_entry_created_response(&Uuid::new_v4().to_string(), &time_entry, user_id);
    let broadcast_bytes = broadcast.encode_to_vec();
    if let Err(e) = state
        .registry
        .broadcast_to_project(
            &project_id.to_string(),
            Message::Binary(broadcast_bytes.into()),
        )
        .await
    {
        log::warn!("Failed to broadcast TimeEntryCreated via REST: {}", e);
    }

    Ok(Json(TimeEntryResponse {
        time_entry: time_entry.into(),
    }))
}

/// PUT /api/v1/time-entries/:id
///
/// Update a time entry (stop timer or edit description). Broadcasts activity to WebSocket clients.
pub async fn update_time_entry(
    State(state): State<AppState>,
    UserId(user_id): UserId,
    Path(id): Path<String>,
    Json(req): Json<UpdateTimeEntryRequest>,
) -> ApiResult<Json<TimeEntryResponse>> {
    // 1. Parse ID
    let time_entry_id = Uuid::parse_str(&id)?;

    // 2. Load existing time entry
    let repo = TimeEntryRepository::new(state.pool.clone());
    let mut time_entry =
        repo.find_by_id(time_entry_id)
            .await?
            .ok_or_else(|| ApiError::NotFound {
                message: format!("Time entry {} not found", id),
                location: ErrorLocation::from(Location::caller()),
            })?;

    // 3. Fetch work item to get project_id for broadcasts
    let work_item = WorkItemRepository::find_by_id(&state.pool, time_entry.work_item_id)
        .await?
        .ok_or_else(|| ApiError::NotFound {
            message: format!("Work item {} not found", time_entry.work_item_id),
            location: ErrorLocation::from(Location::caller()),
        })?;

    let project_id = work_item.project_id;

    // 4. Apply stop if requested
    if req.stop == Some(true) && time_entry.is_running() {
        time_entry.stop();
    }

    // 5. Apply description update if provided
    if let Some(desc) = req.description.as_ref() {
        let sanitized = sanitize_string(desc);
        time_entry.description = if sanitized.trim().is_empty() {
            None
        } else {
            Some(sanitized)
        };
    }

    // 6. Execute transaction
    let activity = ActivityLog::updated("time_entry", time_entry.id, user_id, &[]);
    let activity_clone = activity.clone();
    let time_entry_clone = time_entry.clone();

    let mut tx = state.pool.begin().await?;
    repo.update(&time_entry_clone).await?;
    ActivityLogRepository::create(&mut *tx, &activity_clone).await?;
    tx.commit().await?;

    // 7. Broadcast ActivityLogCreated
    let event = build_activity_log_created_event(&activity);
    let bytes = event.encode_to_vec();
    let message = Message::Binary(bytes.into());
    if let Err(e) = state
        .registry
        .broadcast_activity_log_created(
            &project_id.to_string(),
            Some(&time_entry.work_item_id.to_string()),
            None,
            message,
        )
        .await
    {
        log::warn!(
            "Failed to broadcast time entry update activity log to WebSocket clients: {}",
            e
        );
    }

    // 8. Broadcast TimeEntryUpdated
    let broadcast =
        build_time_entry_updated_response(&Uuid::new_v4().to_string(), &time_entry, user_id);
    let broadcast_bytes = broadcast.encode_to_vec();
    if let Err(e) = state
        .registry
        .broadcast_to_project(
            &project_id.to_string(),
            Message::Binary(broadcast_bytes.into()),
        )
        .await
    {
        log::warn!("Failed to broadcast TimeEntryUpdated via REST: {}", e);
    }

    Ok(Json(TimeEntryResponse {
        time_entry: time_entry.into(),
    }))
}

/// DELETE /api/v1/time-entries/:id
///
/// Delete a time entry (soft delete). Broadcasts activity to WebSocket clients.
pub async fn delete_time_entry(
    State(state): State<AppState>,
    UserId(user_id): UserId,
    Path(id): Path<String>,
) -> ApiResult<Json<DeleteResponse>> {
    // 1. Parse ID
    let time_entry_id = Uuid::parse_str(&id)?;

    // 2. Load time entry
    let repo = TimeEntryRepository::new(state.pool.clone());
    let time_entry = repo
        .find_by_id(time_entry_id)
        .await?
        .ok_or_else(|| ApiError::NotFound {
            message: format!("Time entry {} not found", id),
            location: ErrorLocation::from(Location::caller()),
        })?;

    // 3. Fetch work item to get project_id for broadcasts
    let work_item = WorkItemRepository::find_by_id(&state.pool, time_entry.work_item_id)
        .await?
        .ok_or_else(|| ApiError::NotFound {
            message: format!("Work item {} not found", time_entry.work_item_id),
            location: ErrorLocation::from(Location::caller()),
        })?;

    let project_id = work_item.project_id;

    // 4. Execute transaction
    let now = Utc::now().timestamp();
    let activity = ActivityLog::deleted("time_entry", time_entry.id, user_id);
    let activity_clone = activity.clone();

    let mut tx = state.pool.begin().await?;
    repo.delete(time_entry_id, now).await?;
    ActivityLogRepository::create(&mut *tx, &activity_clone).await?;
    tx.commit().await?;

    // 5. Broadcast ActivityLogCreated
    let event = build_activity_log_created_event(&activity);
    let bytes = event.encode_to_vec();
    let message = Message::Binary(bytes.into());
    if let Err(e) = state
        .registry
        .broadcast_activity_log_created(
            &project_id.to_string(),
            Some(&time_entry.work_item_id.to_string()),
            None,
            message,
        )
        .await
    {
        log::warn!(
            "Failed to broadcast time entry deletion activity log to WebSocket clients: {}",
            e
        );
    }

    // 6. Broadcast TimeEntryDeleted
    let broadcast = build_time_entry_deleted_response(
        &Uuid::new_v4().to_string(),
        time_entry.id,
        time_entry.work_item_id,
        user_id,
    );
    let broadcast_bytes = broadcast.encode_to_vec();
    if let Err(e) = state
        .registry
        .broadcast_to_project(
            &project_id.to_string(),
            Message::Binary(broadcast_bytes.into()),
        )
        .await
    {
        log::warn!("Failed to broadcast TimeEntryDeleted via REST: {}", e);
    }

    Ok(Json(DeleteResponse {
        deleted_id: time_entry.id.to_string(),
    }))
}
