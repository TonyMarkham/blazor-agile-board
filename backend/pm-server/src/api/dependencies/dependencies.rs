//! Dependency REST API handlers
//!
//! These handlers provide HTTP access to dependencies and broadcast changes
//! via WebSocket so connected clients see updates in real-time.

use crate::{
    ApiError, ApiResult, CreateDependencyRequest, DeleteResponse, DependencyListResponse, UserId,
};

use pm_core::{ActivityLog, Dependency, DependencyDto, DependencyType};
use pm_db::{ActivityLogRepository, DependencyRepository, WorkItemRepository};
use pm_ws::{
    AppState, build_activity_log_created_event, build_dependency_created_response,
    build_dependency_deleted_response,
};

use std::panic::Location;
use std::str::FromStr;

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

/// GET /api/v1/work-items/:id/dependencies
///
/// List all dependencies for a work item (both blocking and blocked)
pub async fn list_dependencies(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<DependencyListResponse>> {
    let work_item_id = Uuid::parse_str(&id)?;

    let repo = DependencyRepository::new(state.pool.clone());

    // Get items that block this one
    let blocking = repo.find_blocking(work_item_id).await?;

    // Get items that this one blocks
    let blocked = repo.find_blocked(work_item_id).await?;

    // Combine both lists and deduplicate by ID
    let mut all_deps = blocking;
    all_deps.extend(blocked);
    all_deps.sort_by_key(|d| d.id);
    all_deps.dedup_by_key(|d| d.id);

    let dtos: Vec<DependencyDto> = all_deps.into_iter().map(DependencyDto::from).collect();

    Ok(Json(DependencyListResponse { dependencies: dtos }))
}

/// POST /api/v1/dependencies
///
/// Create a dependency link. Broadcasts activity to WebSocket clients.
pub async fn create_dependency(
    State(state): State<AppState>,
    UserId(user_id): UserId,
    Json(req): Json<CreateDependencyRequest>,
) -> ApiResult<Json<DependencyDto>> {
    // 1. Parse UUIDs
    let blocking_id = Uuid::parse_str(&req.blocking_item_id)?;
    let blocked_id = Uuid::parse_str(&req.blocked_item_id)?;

    // 2. Validate dependency_type
    let dep_type =
        DependencyType::from_str(&req.dependency_type).map_err(|_| ApiError::Validation {
            message: format!(
                "Invalid dependency_type: {}. Valid values: blocks, relates_to",
                req.dependency_type
            ),
            location: ErrorLocation::from(Location::caller()),
            field: Some("dependency_type".into()),
        })?;

    // 3. Validate no self-reference
    if blocking_id == blocked_id {
        return Err(ApiError::Validation {
            message: "A work item cannot depend on itself".to_string(),
            location: ErrorLocation::from(Location::caller()),
            field: Some("blocked_item_id".into()),
        });
    }

    // 4. Validate both work items exist
    let blocking_item = WorkItemRepository::find_by_id(&state.pool, blocking_id)
        .await?
        .ok_or_else(|| ApiError::NotFound {
            message: format!("Work item {} not found", req.blocking_item_id),
            location: ErrorLocation::from(Location::caller()),
        })?;

    WorkItemRepository::find_by_id(&state.pool, blocked_id)
        .await?
        .ok_or_else(|| ApiError::NotFound {
            message: format!("Work item {} not found", req.blocked_item_id),
            location: ErrorLocation::from(Location::caller()),
        })?;

    // 5. Check for duplicate
    let dep_repo = DependencyRepository::new(state.pool.clone());
    if let Some(_existing) = dep_repo.find_by_pair(blocking_id, blocked_id).await? {
        return Err(ApiError::Validation {
            message: "This dependency already exists".to_string(),
            location: ErrorLocation::from(Location::caller()),
            field: None,
        });
    }

    // 6. Cycle detection (for Blocks type only)
    if dep_type == DependencyType::Blocks
        && let Some(cycle_path) = dep_repo
            .detect_cycle(blocking_id, blocked_id)
            .await
            .map_err(|e| ApiError::Internal {
                message: e.to_string(),
                location: ErrorLocation::from(Location::caller()),
            })?
    {
        let path_str = cycle_path
            .iter()
            .map(|id| id.to_string()[..8].to_string())
            .collect::<Vec<_>>()
            .join(" → ");
        return Err(ApiError::Validation {
            message: format!(
                "Circular dependency detected: {}. This would create a cycle.",
                path_str
            ),
            field: Some("blocking_item_id".into()),
            location: ErrorLocation::from(Location::caller()),
        });
    }

    // 7. Create dependency
    let now = Utc::now();
    let dependency = Dependency {
        id: Uuid::new_v4(),
        blocking_item_id: blocking_id,
        blocked_item_id: blocked_id,
        dependency_type: dep_type.clone(),
        created_at: now,
        created_by: user_id,
        deleted_at: None,
    };

    // 8. Execute transaction
    let activity = ActivityLog::created("dependency", dependency.id, user_id);
    let activity_clone = activity.clone();
    let dependency_clone = dependency.clone();

    let mut tx = state.pool.begin().await?;
    dep_repo.create(&dependency).await?;
    ActivityLogRepository::create(&mut *tx, &activity_clone).await?;
    tx.commit().await?;

    // 9. Broadcast to WebSocket clients
    let project_id = blocking_item.project_id; // Get project_id from blocking item
    let event = build_activity_log_created_event(&activity);
    let bytes = event.encode_to_vec();
    let message = Message::Binary(bytes.into());
    if let Err(e) = state
        .registry
        .broadcast_activity_log_created(&project_id.to_string(), None, None, message)
        .await
    {
        log::warn!(
            "Failed to broadcast dependency creation to WebSocket clients: {}",
            e
        );
    }

    // 9b. Broadcast DependencyCreated to all project subscribers
    let broadcast =
        build_dependency_created_response(&Uuid::new_v4().to_string(), &dependency_clone, user_id);
    let broadcast_bytes = broadcast.encode_to_vec();
    if let Err(e) = state
        .registry
        .broadcast_to_project(
            &project_id.to_string(),
            Message::Binary(broadcast_bytes.into()),
        )
        .await
    {
        log::warn!("Failed to broadcast DependencyCreated via REST: {}", e);
    }

    Ok(Json(DependencyDto::from(dependency)))
}

/// DELETE /api/v1/dependencies/:id
///
/// Delete a dependency
pub async fn delete_dependency(
    State(state): State<AppState>,
    UserId(user_id): UserId,
    Path(id): Path<String>,
) -> ApiResult<Json<DeleteResponse>> {
    let dependency_id = Uuid::parse_str(&id)?;

    let dep_repo = DependencyRepository::new(state.pool.clone());

    // 1. Fetch existing dependency
    let dependency =
        dep_repo
            .find_by_id(dependency_id)
            .await?
            .ok_or_else(|| ApiError::NotFound {
                message: format!("Dependency {} not found", id),
                location: ErrorLocation::from(Location::caller()),
            })?;

    // 2. Get project_id from blocking work item (for broadcasts)
    let blocking_item = WorkItemRepository::find_by_id(&state.pool, dependency.blocking_item_id)
        .await?
        .ok_or_else(|| ApiError::Internal {
            message: "Blocking work item not found".to_string(),
            location: ErrorLocation::from(Location::caller()),
        })?;

    // 3. Execute transaction (soft delete)
    let activity = ActivityLog::deleted("dependency", dependency_id, user_id);
    let activity_clone = activity.clone();

    let mut tx = state.pool.begin().await?;
    let deleted_at = Utc::now().timestamp();
    dep_repo.delete(dependency_id, deleted_at).await?;
    ActivityLogRepository::create(&mut *tx, &activity_clone).await?;
    tx.commit().await?;

    // 4. Broadcast to WebSocket clients
    let project_id = blocking_item.project_id;
    let event = build_activity_log_created_event(&activity);
    let bytes = event.encode_to_vec();
    let message = Message::Binary(bytes.into());
    if let Err(e) = state
        .registry
        .broadcast_activity_log_created(&project_id.to_string(), None, None, message)
        .await
    {
        log::warn!(
            "Failed to broadcast dependency deletion to WebSocket clients: {}",
            e
        );
    }

    // 4b. Broadcast DependencyDeleted to all project subscribers
    let broadcast = build_dependency_deleted_response(
        &Uuid::new_v4().to_string(),
        dependency_id,
        dependency.blocking_item_id,
        dependency.blocked_item_id,
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
        log::warn!("Failed to broadcast DependencyDeleted via REST: {}", e);
    }

    Ok(Json(DeleteResponse { deleted_id: id }))
}
