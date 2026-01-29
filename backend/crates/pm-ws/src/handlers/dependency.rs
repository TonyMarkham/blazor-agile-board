#![allow(dead_code)]

use crate::{
    HandlerContext, MessageValidator, Result as WsErrorResult, WsError,
    build_activity_log_created_event, build_dependencies_list_response,
    build_dependency_created_response, build_dependency_deleted_response, check_idempotency,
    check_permission, db_read, db_write, decode_cached_response, store_idempotency_non_fatal,
};

use pm_config::{MAX_BLOCKED_DEPENDENCIES_PER_ITEM, MAX_BLOCKING_DEPENDENCIES_PER_ITEM};
use pm_core::{ActivityLog, Dependency, DependencyType, Permission};
use pm_db::{ActivityLogRepository, DependencyRepository, WorkItemRepository};
use pm_proto::{
    CreateDependencyRequest, DeleteDependencyRequest, GetDependenciesRequest, WebSocketMessage,
};

use std::collections::{HashMap, HashSet, VecDeque};
use std::panic::Location;

use axum::extract::ws::Message;
use chrono::Utc;
use error_location::ErrorLocation;
use log::{debug, info};
use prost::Message as ProstMessage;
use uuid::Uuid;

fn parse_uuid(s: &str, field: &str) -> WsErrorResult<Uuid> {
    Uuid::parse_str(s).map_err(|_| WsError::ValidationError {
        message: format!("Invalid UUID format for {}", field),
        field: Some(field.to_string()),
        location: ErrorLocation::from(Location::caller()),
    })
}

/// Detect circular dependencies using BFS.
///
/// If adding `blocking_id -> blocked_id`, we need to check whether
/// `blocked_id` can eventually reach `blocking_id` through existing
/// Blocks-type dependencies.
///
/// # Algorithm
///
/// 1. Start BFS from `blocked_id`
/// 2. Follow all outgoing Blocks edges (items that blocked_id blocks)
/// 3. If we reach `blocking_id`, we have a cycle
/// 4. Return error with the cycle path for debugging
///
/// # Returns
///
/// - `Ok(())` if no cycle detected
/// - `Err` with cycle path if cycle would be created
async fn detect_circular_dependency(
    repo: &DependencyRepository,
    blocking_id: Uuid,
    blocked_id: Uuid,
) -> WsErrorResult<()> {
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    let mut parent_map: HashMap<Uuid, Uuid> = HashMap::new();

    queue.push_back(blocked_id);
    visited.insert(blocked_id);

    while let Some(current) = queue.pop_front() {
        // Get all items that `current` blocks (outgoing edges)
        let blocked_by_current =
            repo.find_blocked(current)
                .await
                .map_err(|e| WsError::Database {
                    message: e.to_string(),
                    location: ErrorLocation::from(Location::caller()),
                })?;

        for dep in blocked_by_current {
            // Only follow Blocks edges (RelatesTo doesn't create cycles)
            if dep.dependency_type != DependencyType::Blocks {
                continue;
            }

            if dep.blocked_item_id == blocking_id {
                // Found a cycle! Build the path for error message
                let mut path = vec![blocking_id];
                let mut node = current;

                // Walk back through parent_map to reconstruct path
                while let Some(&parent) = parent_map.get(&node) {
                    path.push(node);
                    node = parent;
                }
                path.push(blocked_id);
                path.reverse();

                // Format path with short UUIDs for readability
                let path_str = path
                    .iter()
                    .map(|id| id.to_string()[..8].to_string())
                    .collect::<Vec<_>>()
                    .join(" → ");

                return Err(WsError::ValidationError {
                    message: format!(
                        "Circular dependency detected: {}. This would create a cycle.",
                        path_str
                    ),
                    field: Some("blocking_item_id".into()),
                    location: ErrorLocation::from(Location::caller()),
                });
            }

            if !visited.contains(&dep.blocked_item_id) {
                visited.insert(dep.blocked_item_id);
                parent_map.insert(dep.blocked_item_id, current);
                queue.push_back(dep.blocked_item_id);
            }
        }
    }

    Ok(())
}

/// Create a dependency between two work items.
///
/// # Validation
///
/// 1. Self-reference check (item cannot block itself)
/// 2. Same-project check (both items must be in same project)
/// 3. Duplicate check (pair cannot already exist)
/// 4. Limit check (max 50 blocking/blocked per item)
/// 5. Cycle check (for Blocks type only)
pub async fn handle_create_dependency(
    req: CreateDependencyRequest,
    ctx: HandlerContext,
) -> WsErrorResult<WebSocketMessage> {
    debug!("{} CreateDependency starting", ctx.log_prefix());

    // 1. Parse IDs
    let blocking_id = parse_uuid(&req.blocking_item_id, "blocking_item_id")?;
    let blocked_id = parse_uuid(&req.blocked_item_id, "blocked_item_id")?;

    // 2. Self-reference check
    if blocking_id == blocked_id {
        return Err(WsError::ValidationError {
            message: "Work item cannot block itself".into(),
            field: Some("blocked_item_id".into()),
            location: ErrorLocation::from(Location::caller()),
        });
    }

    // 3. Parse and validate dependency type
    let dep_type = MessageValidator::validate_dependency_type(req.dependency_type)?;

    // 4. Check idempotency
    let cached = db_read(&ctx, "check_idempotency", || async {
        check_idempotency(&ctx.pool, &ctx.message_id).await
    })
    .await?;
    if let Some(cached_response) = cached {
        info!("{} Returning cached idempotent response", ctx.log_prefix());
        return decode_cached_response(&cached_response);
    }

    // 5. Verify both work items exist
    let blocking_item = db_read(&ctx, "find_blocking_item", || async {
        WorkItemRepository::find_by_id(&ctx.pool, blocking_id)
            .await
            .map_err(WsError::from)
    })
    .await?
    .ok_or_else(|| WsError::NotFound {
        message: format!("Work item {} not found (blocking)", blocking_id),
        location: ErrorLocation::from(Location::caller()),
    })?;

    let blocked_item = db_read(&ctx, "find_blocked_item", || async {
        WorkItemRepository::find_by_id(&ctx.pool, blocked_id)
            .await
            .map_err(WsError::from)
    })
    .await?
    .ok_or_else(|| WsError::NotFound {
        message: format!("Work item {} not found (blocked)", blocked_id),
        location: ErrorLocation::from(Location::caller()),
    })?;

    // 6. SAME-PROJECT CHECK
    if blocking_item.project_id != blocked_item.project_id {
        return Err(WsError::ValidationError {
            message: "Dependencies can only be created between items in the same project".into(),
            field: Some("blocking_item_id".into()),
            location: ErrorLocation::from(Location::caller()),
        });
    }

    // 7. Check Edit permission on the project
    db_read(&ctx, "check_permission", || async {
        check_permission(&ctx, blocking_item.project_id, Permission::Edit).await
    })
    .await?;

    let dep_repo = DependencyRepository::new(ctx.pool.clone());

    // 8. Check for duplicate
    let existing = db_read(&ctx, "check_duplicate", || async {
        dep_repo
            .find_by_pair(blocking_id, blocked_id)
            .await
            .map_err(WsError::from)
    })
    .await?;

    if existing.is_some() {
        return Err(WsError::ValidationError {
            message: "Dependency already exists between these items".into(),
            field: Some("blocked_item_id".into()),
            location: ErrorLocation::from(Location::caller()),
        });
    }

    // 9. Check dependency limits
    let blocking_count = db_read(&ctx, "count_blocking", || async {
        dep_repo
            .count_blocking(blocked_id)
            .await
            .map_err(WsError::from)
    })
    .await?;

    if blocking_count >= MAX_BLOCKING_DEPENDENCIES_PER_ITEM {
        return Err(WsError::ValidationError {
            message: format!(
                "Item already has {} blocking dependencies (max {})",
                blocking_count, MAX_BLOCKING_DEPENDENCIES_PER_ITEM
            ),
            field: Some("blocked_item_id".into()),
            location: ErrorLocation::from(Location::caller()),
        });
    }

    let blocked_count = db_read(&ctx, "count_blocked", || async {
        dep_repo
            .count_blocked(blocking_id)
            .await
            .map_err(WsError::from)
    })
    .await?;

    if blocked_count >= MAX_BLOCKED_DEPENDENCIES_PER_ITEM {
        return Err(WsError::ValidationError {
            message: format!(
                "Item already blocks {} items (max {})",
                blocked_count, MAX_BLOCKED_DEPENDENCIES_PER_ITEM
            ),
            field: Some("blocking_item_id".into()),
            location: ErrorLocation::from(Location::caller()),
        });
    }

    // 10. CIRCULAR DEPENDENCY CHECK (only for Blocks type)
    if dep_type == DependencyType::Blocks {
        detect_circular_dependency(&dep_repo, blocking_id, blocked_id).await?;
    }

    // 11. Create dependency
    let now = Utc::now();
    let dependency = Dependency {
        id: Uuid::new_v4(),
        blocking_item_id: blocking_id,
        blocked_item_id: blocked_id,
        dependency_type: dep_type.clone(),
        created_at: now,
        created_by: ctx.user_id,
        deleted_at: None,
    };

    let activity = ActivityLog::created("dependency", dependency.id, ctx.user_id);
    let activity_clone = activity.clone();
    db_write(&ctx, "create_dependency", || async {
        dep_repo.create(&dependency).await?;
        ActivityLogRepository::create(&ctx.pool, &activity_clone).await?;
        Ok::<_, WsError>(())
    })
    .await?;

    let event = build_activity_log_created_event(&activity);
    let bytes = event.encode_to_vec();
    let message = Message::Binary(bytes.into());
    let project_id_str = blocking_item.project_id.to_string();
    let work_item_id_str = blocking_item.id.to_string();
    ctx.registry
        .broadcast_activity_log_created(&project_id_str, Some(&work_item_id_str), None, message)
        .await?;

    // 12. Build response
    let response = build_dependency_created_response(&ctx.message_id, &dependency, ctx.user_id);

    // 13. Store idempotency
    store_idempotency_non_fatal(&ctx.pool, &ctx.message_id, "create_dependency", &response).await;

    info!(
        "{} Created dependency: {} {:?} {}",
        ctx.log_prefix(),
        blocking_id,
        dep_type,
        blocked_id
    );

    Ok(response)
}

/// Delete a dependency.
///
/// # Authorization
///
/// Requires Edit permission on the project containing the work items.
pub async fn handle_delete_dependency(
    req: DeleteDependencyRequest,
    ctx: HandlerContext,
) -> WsErrorResult<WebSocketMessage> {
    debug!("{} DeleteDependency starting", ctx.log_prefix());

    let dependency_id = parse_uuid(&req.dependency_id, "dependency_id")?;

    // Find dependency
    let dep_repo = DependencyRepository::new(ctx.pool.clone());
    let dependency = db_read(&ctx, "find_dependency", || async {
        dep_repo
            .find_by_id(dependency_id)
            .await
            .map_err(WsError::from)
    })
    .await?
    .ok_or_else(|| WsError::NotFound {
        message: format!("Dependency {} not found", dependency_id),
        location: ErrorLocation::from(Location::caller()),
    })?;

    // Get work item to check project permission
    let work_item = db_read(&ctx, "find_work_item", || async {
        WorkItemRepository::find_by_id(&ctx.pool, dependency.blocking_item_id)
            .await
            .map_err(WsError::from)
    })
    .await?
    .ok_or_else(|| WsError::NotFound {
        message: format!("Work item {} not found", dependency.blocking_item_id),
        location: ErrorLocation::from(Location::caller()),
    })?;

    // Check Edit permission
    db_read(&ctx, "check_permission", || async {
        check_permission(&ctx, work_item.project_id, Permission::Edit).await
    })
    .await?;

    // Soft delete
    let now = Utc::now().timestamp();
    let activity = ActivityLog::deleted("dependency", dependency_id, ctx.user_id);
    let activity_clone = activity.clone();
    db_write(&ctx, "delete_dependency", || async {
        dep_repo.delete(dependency_id, now).await?;
        ActivityLogRepository::create(&ctx.pool, &activity_clone).await?;
        Ok::<_, WsError>(())
    })
    .await?;

    let event = build_activity_log_created_event(&activity);
    let bytes = event.encode_to_vec();
    let message = Message::Binary(bytes.into());
    let project_id_str = work_item.project_id.to_string();
    let work_item_id_str = work_item.id.to_string();
    ctx.registry
        .broadcast_activity_log_created(&project_id_str, Some(&work_item_id_str), None, message)
        .await?;

    let response = build_dependency_deleted_response(
        &ctx.message_id,
        dependency_id,
        dependency.blocking_item_id,
        dependency.blocked_item_id,
        ctx.user_id,
    );

    info!("{} Deleted dependency {}", ctx.log_prefix(), dependency_id);

    Ok(response)
}

/// Get dependencies for a work item.
///
/// Returns both:
/// - `blocking`: Items that are blocking this work item
/// - `blocked`: Items that this work item blocks
pub async fn handle_get_dependencies(
    req: GetDependenciesRequest,
    ctx: HandlerContext,
) -> WsErrorResult<WebSocketMessage> {
    debug!("{} GetDependencies starting", ctx.log_prefix());

    let work_item_id = parse_uuid(&req.work_item_id, "work_item_id")?;

    // Verify work item exists, check View permission
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

    db_read(&ctx, "check_permission", || async {
        check_permission(&ctx, work_item.project_id, Permission::View).await
    })
    .await?;

    let dep_repo = DependencyRepository::new(ctx.pool.clone());

    // Get both directions
    let blocking = db_read(&ctx, "find_blocking", || async {
        dep_repo
            .find_blocking(work_item_id)
            .await
            .map_err(WsError::from)
    })
    .await?;

    let blocked = db_read(&ctx, "find_blocked", || async {
        dep_repo
            .find_blocked(work_item_id)
            .await
            .map_err(WsError::from)
    })
    .await?;

    debug!(
        "{} Found {} blocking, {} blocked for {}",
        ctx.log_prefix(),
        blocking.len(),
        blocked.len(),
        work_item_id
    );

    Ok(build_dependencies_list_response(
        &ctx.message_id,
        &blocking,
        &blocked,
    ))
}
