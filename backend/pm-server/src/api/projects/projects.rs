//! Project REST API handlers
//!
//! Read-only handlers for listing and retrieving projects.

use crate::{
    ApiError, ApiResult, CreateProjectRequest, DeleteResponse, ProjectListResponse,
    ProjectResponse, UpdateProjectRequest, UserId, api::resolve::resolve_project,
};

use pm_core::{ActivityLog, Project, ProjectDto, ProjectStatus};
use pm_db::{ActivityLogRepository, ProjectRepository};
use pm_ws::{
    AppState, build_activity_log_created_event, build_project_created_response,
    build_project_deleted_response, build_project_updated_response, sanitize_string,
};

use std::{panic::Location, str::FromStr};

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

/// GET /api/v1/projects
///
/// List all projects
pub async fn list_projects(State(state): State<AppState>) -> ApiResult<Json<ProjectListResponse>> {
    let repo = ProjectRepository::new(state.pool.clone());
    let projects = repo.find_all().await?;

    Ok(Json(ProjectListResponse {
        projects: projects.into_iter().map(ProjectDto::from).collect(),
    }))
}

/// GET /api/v1/projects/:id
///
/// Get a single project by ID
pub async fn get_project(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ProjectResponse>> {
    let project = resolve_project(&state.pool, &id).await?;

    Ok(Json(ProjectResponse {
        project: project.into(),
    }))
}

/// POST /api/v1/projects
///
/// Create a new project. Broadcasts activity to WebSocket clients.
pub async fn create_project(
    State(state): State<AppState>,
    UserId(user_id): UserId,
    Json(req): Json<CreateProjectRequest>,
) -> ApiResult<Json<ProjectResponse>> {
    // 1. Sanitize and validate title
    let title = sanitize_string(&req.title);
    let title = title.trim();
    if title.is_empty() {
        return Err(ApiError::Validation {
            message: "Project title cannot be empty".to_string(),
            field: Some("title".into()),
            location: ErrorLocation::from(Location::caller()),
        });
    }

    // 2. Sanitize and validate key
    let key = req.key.trim().to_uppercase();
    if key.is_empty() {
        return Err(ApiError::Validation {
            message: "Project key cannot be empty".to_string(),
            field: Some("key".into()),
            location: ErrorLocation::from(Location::caller()),
        });
    }

    // 3. Check key uniqueness
    let repo = ProjectRepository::new(state.pool.clone());
    if let Some(existing) = repo.find_by_key(&key).await? {
        return Err(ApiError::Conflict {
            message: format!("Project with key '{}' already exists", key),
            current_version: existing.version,
            location: ErrorLocation::from(Location::caller()),
        });
    }

    // 4. Build project
    let mut project = Project::new(title.to_string(), key.clone(), user_id);
    if let Some(desc) = &req.description {
        let sanitized_desc = sanitize_string(desc);
        if !sanitized_desc.trim().is_empty() {
            project.description = Some(sanitized_desc);
        }
    }

    // 5. Execute transaction
    let activity = ActivityLog::created("project", project.id, user_id);
    let activity_clone = activity.clone();
    let project_clone = project.clone();

    let mut tx = state.pool.begin().await?;
    repo.create(&project_clone).await?;
    ActivityLogRepository::create(&mut *tx, &activity_clone).await?;
    tx.commit().await?;

    // 6. Broadcast ActivityLogCreated to WebSocket clients
    let event = build_activity_log_created_event(&activity);
    let bytes = event.encode_to_vec();
    let message = Message::Binary(bytes.into());
    if let Err(e) = state
        .registry
        .broadcast_activity_log_created(&project.id.to_string(), None, None, message)
        .await
    {
        log::warn!(
            "Failed to broadcast project creation activity log to WebSocket clients: {}",
            e
        );
    }

    // 7. Broadcast ProjectCreated to all project subscribers
    let broadcast = build_project_created_response(&Uuid::new_v4().to_string(), &project, user_id);
    let broadcast_bytes = broadcast.encode_to_vec();
    if let Err(e) = state
        .registry
        .broadcast_to_project(
            &project.id.to_string(),
            Message::Binary(broadcast_bytes.into()),
        )
        .await
    {
        log::warn!("Failed to broadcast ProjectCreated via REST: {}", e);
    }

    log::info!(
        "Created project {} ({}) via REST API",
        project.id,
        project.key
    );

    Ok(Json(ProjectResponse {
        project: project.into(),
    }))
}

/// PUT /api/v1/projects/:id
///
/// Update a project. Uses optimistic locking via expected_version.
pub async fn update_project(
    State(state): State<AppState>,
    UserId(user_id): UserId,
    Path(id): Path<String>,
    Json(req): Json<UpdateProjectRequest>,
) -> ApiResult<Json<ProjectResponse>> {
    // 1. Load existing project
    let mut project = resolve_project(&state.pool, &id).await?;
    let repo = ProjectRepository::new(state.pool.clone());

    // 2. refactored out

    // 3. Check optimistic locking
    if project.version != req.expected_version {
        return Err(ApiError::Conflict {
            message: format!(
                "Version mismatch: expected {}, found {}",
                req.expected_version, project.version
            ),
            current_version: project.version,
            location: ErrorLocation::from(Location::caller()),
        });
    }

    // 4. Apply updates
    let mut changed = false;

    if let Some(title) = &req.title {
        let sanitized = sanitize_string(title).trim().to_string();
        if sanitized.is_empty() {
            return Err(ApiError::Validation {
                message: "Project title cannot be empty".to_string(),
                field: Some("title".into()),
                location: ErrorLocation::from(Location::caller()),
            });
        }
        if project.title != sanitized {
            project.title = sanitized;
            changed = true;
        }
    }

    if let Some(description) = &req.description {
        let sanitized = sanitize_string(description);
        let new_desc = if sanitized.trim().is_empty() {
            None
        } else {
            Some(sanitized)
        };
        if project.description != new_desc {
            project.description = new_desc;
            changed = true;
        }
    }

    if let Some(status_str) = &req.status {
        let new_status = ProjectStatus::from_str(status_str).map_err(|_| ApiError::Validation {
            message: format!(
                "Invalid status: '{}'. Valid values: active, archived",
                status_str
            ),
            field: Some("status".into()),
            location: ErrorLocation::from(Location::caller()),
        })?;
        if project.status != new_status {
            project.status = new_status;
            changed = true;
        }
    }

    if !changed {
        // No changes, return current state
        return Ok(Json(ProjectResponse {
            project: project.into(),
        }));
    }

    // 5. Update metadata
    project.version += 1;
    project.updated_at = Utc::now();
    project.updated_by = user_id;

    // 6. Execute transaction
    let activity = ActivityLog::updated("project", project.id, user_id, &[]);
    let activity_clone = activity.clone();
    let project_clone = project.clone();

    let mut tx = state.pool.begin().await?;
    repo.update(&project_clone).await?;
    ActivityLogRepository::create(&mut *tx, &activity_clone).await?;
    tx.commit().await?;

    // 7. Broadcast ActivityLogCreated
    let event = build_activity_log_created_event(&activity);
    let bytes = event.encode_to_vec();
    let message = Message::Binary(bytes.into());
    if let Err(e) = state
        .registry
        .broadcast_activity_log_created(&project.id.to_string(), None, None, message)
        .await
    {
        log::warn!(
            "Failed to broadcast project update activity log to WebSocket clients: {}",
            e
        );
    }

    // 8. Broadcast ProjectUpdated
    let broadcast = build_project_updated_response(
        &Uuid::new_v4().to_string(),
        &project,
        &[], // field_changes - we could track these but keeping it simple
        user_id,
    );
    let broadcast_bytes = broadcast.encode_to_vec();
    if let Err(e) = state
        .registry
        .broadcast_to_project(
            &project.id.to_string(),
            Message::Binary(broadcast_bytes.into()),
        )
        .await
    {
        log::warn!("Failed to broadcast ProjectUpdated via REST: {}", e);
    }

    log::info!(
        "Updated project {} ({}) via REST API",
        project.id,
        project.key
    );

    Ok(Json(ProjectResponse {
        project: project.into(),
    }))
}

/// DELETE /api/v1/projects/:id
///
/// Soft delete a project. Broadcasts activity to WebSocket clients.
pub async fn delete_project(
    State(state): State<AppState>,
    UserId(user_id): UserId,
    Path(id): Path<String>,
) -> ApiResult<Json<DeleteResponse>> {
    // 1. Load existing project
    let project = resolve_project(&state.pool, &id).await?;
    let project_id = project.id;
    let repo = ProjectRepository::new(state.pool.clone());

    // 2. refactored out

    // 3. Execute transaction
    let now = Utc::now();
    let activity = ActivityLog::deleted("project", project.id, user_id);
    let activity_clone = activity.clone();

    let mut tx = state.pool.begin().await?;
    repo.delete(project_id, now.timestamp()).await?;
    ActivityLogRepository::create(&mut *tx, &activity_clone).await?;
    tx.commit().await?;

    // 4. Broadcast ActivityLogCreated
    let event = build_activity_log_created_event(&activity);
    let bytes = event.encode_to_vec();
    let message = Message::Binary(bytes.into());
    if let Err(e) = state
        .registry
        .broadcast_activity_log_created(&project.id.to_string(), None, None, message)
        .await
    {
        log::warn!(
            "Failed to broadcast project deletion activity log to WebSocket clients: {}",
            e
        );
    }

    // 5. Broadcast ProjectDeleted
    let broadcast =
        build_project_deleted_response(&Uuid::new_v4().to_string(), project_id, user_id);
    let broadcast_bytes = broadcast.encode_to_vec();
    if let Err(e) = state
        .registry
        .broadcast_to_project(
            &project.id.to_string(),
            Message::Binary(broadcast_bytes.into()),
        )
        .await
    {
        log::warn!("Failed to broadcast ProjectDeleted via REST: {}", e);
    }

    log::info!(
        "Deleted project {} ({}) via REST API",
        project.id,
        project.key
    );

    Ok(Json(DeleteResponse {
        deleted_id: project_id.to_string(),
    }))
}
