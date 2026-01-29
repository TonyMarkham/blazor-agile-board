use crate::{
    HandlerContext, Result as WsErrorResult, WsError, build_activity_log_list_response,
    check_permission, db_read,
};

use pm_core::Permission;
use pm_db::{
    ActivityLogRepository, CommentRepository, DependencyRepository, ProjectRepository,
    SprintRepository, TimeEntryRepository, WorkItemRepository,
};
use pm_proto::{GetActivityLogRequest, WebSocketMessage};

use std::panic::Location;

use error_location::ErrorLocation;
use log::debug;
use uuid::Uuid;

/// Valid entity types for activity log queries
const VALID_ENTITY_TYPES: &[&str] = &[
    "work_item",
    "sprint",
    "comment",
    "time_entry",
    "dependency",
    "project",
];

pub async fn handle_get_activity_log(
    req: GetActivityLogRequest,
    ctx: HandlerContext,
) -> WsErrorResult<WebSocketMessage> {
    debug!("{} GetActivityLog starting", ctx.log_prefix());

    // 1. Validate entity_id
    let entity_id = Uuid::parse_str(&req.entity_id).map_err(|_| WsError::ValidationError {
        message: format!("Invalid entity_id: {}", req.entity_id),
        field: Some("entity_id".to_string()),
        location: ErrorLocation::from(Location::caller()),
    })?;

    // 2. Validate entity_type
    if !VALID_ENTITY_TYPES.contains(&req.entity_type.as_str()) {
        return Err(WsError::ValidationError {
            message: format!(
                "Invalid entity_type '{}'. Must be one of: {:?}",
                req.entity_type, VALID_ENTITY_TYPES
            ),
            field: Some("entity_type".to_string()),
            location: ErrorLocation::from(Location::caller()),
        });
    }

    // 3. Resolve project_id for access control
    let project_id = get_entity_project_id(&ctx, &req.entity_type, entity_id).await?;

    // 4. Authorization: View permission required
    check_permission(&ctx, project_id, Permission::View).await?;

    // 3. Pagination defaults
    let limit = req.limit.min(100).max(1) as i64;
    let offset = req.offset.max(0) as i64;

    // 4. Query activity log
    let (entries, total_count) = db_read(&ctx, "get_activity_log", || async {
        ActivityLogRepository::find_by_entity_paginated(
            &ctx.pool,
            &req.entity_type,
            entity_id,
            limit,
            offset,
        )
        .await
        .map_err(WsError::from)
    })
    .await?;

    // 5. Build response (builder added in Step 6)
    Ok(build_activity_log_list_response(
        &ctx.message_id,
        entries,
        total_count,
        limit,
        offset,
    ))
}

async fn get_entity_project_id(
    ctx: &HandlerContext,
    entity_type: &str,
    entity_id: Uuid,
) -> WsErrorResult<Uuid> {
    match entity_type {
        "work_item" => {
            let work_item = WorkItemRepository::find_by_id(&ctx.pool, entity_id)
                .await
                .map_err(WsError::from)?;
            work_item
                .map(|wi| wi.project_id)
                .ok_or_else(|| WsError::NotFound {
                    message: format!("Work item {} not found", entity_id),
                    location: ErrorLocation::from(Location::caller()),
                })
        }
        "project" => {
            let repo = ProjectRepository::new(ctx.pool.clone());
            let project = repo.find_by_id(entity_id).await.map_err(WsError::from)?;
            project.map(|p| p.id).ok_or_else(|| WsError::NotFound {
                message: format!("Project {} not found", entity_id),
                location: ErrorLocation::from(Location::caller()),
            })
        }
        "sprint" => {
            let repo = SprintRepository::new(ctx.pool.clone());
            let sprint = repo.find_by_id(entity_id).await.map_err(WsError::from)?;
            sprint
                .map(|s| s.project_id)
                .ok_or_else(|| WsError::NotFound {
                    message: format!("Sprint {} not found", entity_id),
                    location: ErrorLocation::from(Location::caller()),
                })
        }
        "comment" => {
            let repo = CommentRepository::new(ctx.pool.clone());
            let comment = repo.find_by_id(entity_id).await.map_err(WsError::from)?;
            let comment = comment.ok_or_else(|| WsError::NotFound {
                message: format!("Comment {} not found", entity_id),
                location: ErrorLocation::from(Location::caller()),
            })?;

            let work_item = WorkItemRepository::find_by_id(&ctx.pool, comment.work_item_id)
                .await
                .map_err(WsError::from)?;
            work_item
                .map(|wi| wi.project_id)
                .ok_or_else(|| WsError::NotFound {
                    message: format!("Work item {} not found", comment.work_item_id),
                    location: ErrorLocation::from(Location::caller()),
                })
        }
        "time_entry" => {
            let repo = TimeEntryRepository::new(ctx.pool.clone());
            let entry = repo.find_by_id(entity_id).await.map_err(WsError::from)?;
            let entry = entry.ok_or_else(|| WsError::NotFound {
                message: format!("Time entry {} not found", entity_id),
                location: ErrorLocation::from(Location::caller()),
            })?;

            let work_item = WorkItemRepository::find_by_id(&ctx.pool, entry.work_item_id)
                .await
                .map_err(WsError::from)?;
            work_item
                .map(|wi| wi.project_id)
                .ok_or_else(|| WsError::NotFound {
                    message: format!("Work item {} not found", entry.work_item_id),
                    location: ErrorLocation::from(Location::caller()),
                })
        }
        "dependency" => {
            let repo = DependencyRepository::new(ctx.pool.clone());
            let dep = repo.find_by_id(entity_id).await.map_err(WsError::from)?;
            let dep = dep.ok_or_else(|| WsError::NotFound {
                message: format!("Dependency {} not found", entity_id),
                location: ErrorLocation::from(Location::caller()),
            })?;

            let work_item = WorkItemRepository::find_by_id(&ctx.pool, dep.blocked_item_id)
                .await
                .map_err(WsError::from)?;
            work_item
                .map(|wi| wi.project_id)
                .ok_or_else(|| WsError::NotFound {
                    message: format!("Work item {} not found", dep.blocked_item_id),
                    location: ErrorLocation::from(Location::caller()),
                })
        }
        _ => Err(WsError::ValidationError {
            message: format!("Invalid entity_type: {}", entity_type),
            field: Some("entity_type".to_string()),
            location: ErrorLocation::from(Location::caller()),
        }),
    }
}
