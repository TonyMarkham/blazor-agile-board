//! Sprint REST API handlers
//!
//! These handlers provide HTTP access to sprints and broadcast changes
//! via WebSocket so connected clients see updates in real-time.

use crate::{
    ApiError, ApiResult, CreateSprintRequest, DeleteResponse, SprintListResponse, SprintResponse,
    UpdateSprintRequest, UserId,
};

use pm_core::{ActivityLog, Sprint, SprintDto, SprintStatus};
use pm_db::{ActivityLogRepository, SprintRepository};
use pm_ws::{
    AppState, build_activity_log_created_event, build_sprint_created_response,
    build_sprint_deleted_response, build_sprint_updated_response, sanitize_string,
};

use std::panic::Location;
use std::str::FromStr;

use axum::{
    Json,
    extract::{Path, State, ws::Message},
};
use chrono::{DateTime, Utc};
use error_location::ErrorLocation;
use prost::Message as ProstMessage;
use uuid::Uuid;

// =============================================================================
// Handlers
// =============================================================================

/// GET /api/v1/projects/:project_id/sprints
///
/// List all sprints for a project
pub async fn list_sprints(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
) -> ApiResult<Json<SprintListResponse>> {
    let project_uuid = Uuid::parse_str(&project_id)?;

    let repo = SprintRepository::new(state.pool.clone());
    let sprints = repo.find_by_project(project_uuid).await?;

    Ok(Json(SprintListResponse {
        sprints: sprints.into_iter().map(SprintDto::from).collect(),
    }))
}

/// GET /api/v1/sprints/:id
///
/// Get a single sprint by ID
pub async fn get_sprint(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<SprintResponse>> {
    let sprint_id = Uuid::parse_str(&id)?;

    let repo = SprintRepository::new(state.pool.clone());
    let sprint = repo
        .find_by_id(sprint_id)
        .await?
        .ok_or_else(|| ApiError::NotFound {
            message: format!("Sprint {} not found", id),
            location: ErrorLocation::from(Location::caller()),
        })?;

    Ok(Json(SprintResponse {
        sprint: sprint.into(),
    }))
}

/// POST /api/v1/sprints
///
/// Create a new sprint. Broadcasts activity to WebSocket clients.
pub async fn create_sprint(
    State(state): State<AppState>,
    UserId(user_id): UserId,
    Json(req): Json<CreateSprintRequest>,
) -> ApiResult<Json<SprintResponse>> {
    // 1. Sanitize and validate name
    let name = sanitize_string(&req.name).trim().to_string();
    if name.is_empty() {
        return Err(ApiError::Validation {
            message: "Sprint name cannot be empty".to_string(),
            field: Some("name".into()),
            location: ErrorLocation::from(Location::caller()),
        });
    }

    // 2. Sanitize optional goal
    let goal = req.goal.as_ref().and_then(|g| {
        let sanitized = sanitize_string(g);
        if sanitized.trim().is_empty() {
            None
        } else {
            Some(sanitized)
        }
    });

    // 3. Parse and validate project_id
    let project_id = Uuid::parse_str(&req.project_id).map_err(|_| ApiError::Validation {
        message: format!("Invalid project_id: '{}'", req.project_id),
        field: Some("project_id".into()),
        location: ErrorLocation::from(Location::caller()),
    })?;

    // 4. Convert timestamps to DateTime with validation
    let start_date =
        DateTime::from_timestamp(req.start_date, 0).ok_or_else(|| ApiError::Validation {
            message: format!("Invalid start_date timestamp: {}", req.start_date),
            field: Some("start_date".into()),
            location: ErrorLocation::from(Location::caller()),
        })?;

    let end_date =
        DateTime::from_timestamp(req.end_date, 0).ok_or_else(|| ApiError::Validation {
            message: format!("Invalid end_date timestamp: {}", req.end_date),
            field: Some("end_date".into()),
            location: ErrorLocation::from(Location::caller()),
        })?;

    // 5. Validate date range
    if end_date <= start_date {
        return Err(ApiError::Validation {
            message: "end_date must be after start_date".to_string(),
            field: Some("end_date".into()),
            location: ErrorLocation::from(Location::caller()),
        });
    }

    // 6. Create sprint
    let sprint = Sprint::new(project_id, name, goal, start_date, end_date, user_id);

    // 7. Execute transaction
    let activity = ActivityLog::created("sprint", sprint.id, user_id);
    let activity_clone = activity.clone();
    let sprint_clone = sprint.clone();

    let repo = SprintRepository::new(state.pool.clone());
    let mut tx = state.pool.begin().await?;
    repo.create(&sprint_clone).await?;
    ActivityLogRepository::create(&mut *tx, &activity_clone).await?;
    tx.commit().await?;

    // 8. Broadcast ActivityLogCreated
    let event = build_activity_log_created_event(&activity);
    let bytes = event.encode_to_vec();
    let message = Message::Binary(bytes.into());
    if let Err(e) = state
        .registry
        .broadcast_activity_log_created(
            &project_id.to_string(),
            None,
            Some(&sprint.id.to_string()),
            message,
        )
        .await
    {
        log::warn!(
            "Failed to broadcast sprint creation activity log to WebSocket clients: {}",
            e
        );
    }

    // 9. Broadcast SprintCreated
    let broadcast = build_sprint_created_response(&Uuid::new_v4().to_string(), &sprint, user_id);
    let broadcast_bytes = broadcast.encode_to_vec();
    if let Err(e) = state
        .registry
        .broadcast_to_project(
            &project_id.to_string(),
            Message::Binary(broadcast_bytes.into()),
        )
        .await
    {
        log::warn!("Failed to broadcast SprintCreated via REST: {}", e);
    }

    log::info!(
        "Created sprint {} ({}) via REST API",
        sprint.id,
        sprint.name
    );

    Ok(Json(SprintResponse {
        sprint: sprint.into(),
    }))
}

/// PUT /api/v1/sprints/:id
///
/// Update a sprint. Uses optimistic locking via expected_version.
pub async fn update_sprint(
    State(state): State<AppState>,
    UserId(user_id): UserId,
    Path(id): Path<String>,
    Json(req): Json<UpdateSprintRequest>,
) -> ApiResult<Json<SprintResponse>> {
    // 1. Parse sprint ID
    let sprint_id = Uuid::parse_str(&id)?;

    // 2. Load existing sprint
    let repo = SprintRepository::new(state.pool.clone());
    let mut sprint = repo
        .find_by_id(sprint_id)
        .await?
        .ok_or_else(|| ApiError::NotFound {
            message: format!("Sprint {} not found", id),
            location: ErrorLocation::from(Location::caller()),
        })?;

    // 3. Check optimistic locking
    if sprint.version != req.expected_version {
        return Err(ApiError::Conflict {
            message: format!(
                "Version mismatch: expected {}, found {}",
                req.expected_version, sprint.version
            ),
            current_version: sprint.version,
            location: ErrorLocation::from(Location::caller()),
        });
    }

    // 4. Apply updates
    let mut changed = false;

    if let Some(name) = &req.name {
        let sanitized = sanitize_string(name).trim().to_string();
        if sanitized.is_empty() {
            return Err(ApiError::Validation {
                message: "Sprint name cannot be empty".to_string(),
                field: Some("name".into()),
                location: ErrorLocation::from(Location::caller()),
            });
        }
        if sprint.name != sanitized {
            sprint.name = sanitized;
            changed = true;
        }
    }

    if let Some(goal) = &req.goal {
        let sanitized = sanitize_string(goal);
        let new_goal = if sanitized.trim().is_empty() {
            None
        } else {
            Some(sanitized)
        };
        if sprint.goal != new_goal {
            sprint.goal = new_goal;
            changed = true;
        }
    }

    if let Some(start_ts) = req.start_date {
        let new_start =
            DateTime::from_timestamp(start_ts, 0).ok_or_else(|| ApiError::Validation {
                message: format!("Invalid start_date timestamp: {}", start_ts),
                field: Some("start_date".into()),
                location: ErrorLocation::from(Location::caller()),
            })?;
        if sprint.start_date != new_start {
            sprint.start_date = new_start;
            changed = true;
        }
    }

    if let Some(end_ts) = req.end_date {
        let new_end = DateTime::from_timestamp(end_ts, 0).ok_or_else(|| ApiError::Validation {
            message: format!("Invalid end_date timestamp: {}", end_ts),
            field: Some("end_date".into()),
            location: ErrorLocation::from(Location::caller()),
        })?;
        if sprint.end_date != new_end {
            sprint.end_date = new_end;
            changed = true;
        }
    }

    // Validate date range if either date changed
    if sprint.end_date <= sprint.start_date {
        return Err(ApiError::Validation {
            message: "end_date must be after start_date".to_string(),
            field: Some("end_date".into()),
            location: ErrorLocation::from(Location::caller()),
        });
    }

    if let Some(status_str) = &req.status {
        let new_status = SprintStatus::from_str(status_str).map_err(|_| ApiError::Validation {
            message: format!(
                "Invalid status: '{}'. Valid values: planned, active, completed",
                status_str
            ),
            field: Some("status".into()),
            location: ErrorLocation::from(Location::caller()),
        })?;
        if sprint.status != new_status {
            sprint.status = new_status;
            changed = true;
        }
    }

    if !changed {
        // No changes, return current state
        return Ok(Json(SprintResponse {
            sprint: sprint.into(),
        }));
    }

    // 5. Update metadata
    sprint.version += 1;
    sprint.updated_at = Utc::now();
    sprint.updated_by = user_id;

    // 6. Execute transaction
    let activity = ActivityLog::updated("sprint", sprint.id, user_id, &[]);
    let activity_clone = activity.clone();
    let sprint_clone = sprint.clone();

    let mut tx = state.pool.begin().await?;
    repo.update(&sprint_clone).await?;
    ActivityLogRepository::create(&mut *tx, &activity_clone).await?;
    tx.commit().await?;

    // 7. Broadcast ActivityLogCreated
    let event = build_activity_log_created_event(&activity);
    let bytes = event.encode_to_vec();
    let message = Message::Binary(bytes.into());
    if let Err(e) = state
        .registry
        .broadcast_activity_log_created(
            &sprint.project_id.to_string(),
            None,
            Some(&sprint.id.to_string()),
            message,
        )
        .await
    {
        log::warn!(
            "Failed to broadcast sprint update activity log to WebSocket clients: {}",
            e
        );
    }

    // 8. Broadcast SprintUpdated
    let broadcast = build_sprint_updated_response(
        &Uuid::new_v4().to_string(),
        &sprint,
        &[], // field_changes - keeping it simple for now
        user_id,
    );
    let broadcast_bytes = broadcast.encode_to_vec();
    if let Err(e) = state
        .registry
        .broadcast_to_project(
            &sprint.project_id.to_string(),
            Message::Binary(broadcast_bytes.into()),
        )
        .await
    {
        log::warn!("Failed to broadcast SprintUpdated via REST: {}", e);
    }

    log::info!(
        "Updated sprint {} ({}) via REST API",
        sprint.id,
        sprint.name
    );

    Ok(Json(SprintResponse {
        sprint: sprint.into(),
    }))
}

/// DELETE /api/v1/sprints/:id
///
/// Soft delete a sprint. Broadcasts activity to WebSocket clients.
pub async fn delete_sprint(
    State(state): State<AppState>,
    UserId(user_id): UserId,
    Path(id): Path<String>,
) -> ApiResult<Json<DeleteResponse>> {
    // 1. Parse sprint ID
    let sprint_id = Uuid::parse_str(&id)?;

    // 2. Load existing sprint
    let repo = SprintRepository::new(state.pool.clone());
    let sprint = repo
        .find_by_id(sprint_id)
        .await?
        .ok_or_else(|| ApiError::NotFound {
            message: format!("Sprint {} not found", id),
            location: ErrorLocation::from(Location::caller()),
        })?;

    // 3. Execute transaction
    let now = Utc::now();
    let activity = ActivityLog::deleted("sprint", sprint.id, user_id);
    let activity_clone = activity.clone();

    let mut tx = state.pool.begin().await?;
    repo.delete(sprint_id, now.timestamp()).await?;
    ActivityLogRepository::create(&mut *tx, &activity_clone).await?;
    tx.commit().await?;

    // 4. Broadcast ActivityLogCreated
    let event = build_activity_log_created_event(&activity);
    let bytes = event.encode_to_vec();
    let message = Message::Binary(bytes.into());
    if let Err(e) = state
        .registry
        .broadcast_activity_log_created(
            &sprint.project_id.to_string(),
            None,
            Some(&sprint.id.to_string()),
            message,
        )
        .await
    {
        log::warn!(
            "Failed to broadcast sprint deletion activity log to WebSocket clients: {}",
            e
        );
    }

    // 5. Broadcast SprintDeleted
    let broadcast = build_sprint_deleted_response(&Uuid::new_v4().to_string(), sprint_id, user_id);
    let broadcast_bytes = broadcast.encode_to_vec();
    if let Err(e) = state
        .registry
        .broadcast_to_project(
            &sprint.project_id.to_string(),
            Message::Binary(broadcast_bytes.into()),
        )
        .await
    {
        log::warn!("Failed to broadcast SprintDeleted via REST: {}", e);
    }

    log::info!(
        "Deleted sprint {} ({}) via REST API",
        sprint.id,
        sprint.name
    );

    Ok(Json(DeleteResponse {
        deleted_id: sprint_id.to_string(),
    }))
}
