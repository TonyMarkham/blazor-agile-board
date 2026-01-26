use crate::handlers::{
    authorization::check_permission,
    change_tracker::track_changes,
    context::HandlerContext,
    db_ops::{db_read, db_write},
    hierarchy_validator::validate_hierarchy,
    idempotency::{check_idempotency, store_idempotency},
    response_builder::*,
};
use crate::{MessageValidator, Result as WsErrorResult, WsError};

use pm_core::{ActivityLog, Permission, WorkItem, WorkItemType};
use pm_db::{ActivityLogRepository, WorkItemRepository};
use pm_proto::{
    CreateWorkItemRequest, DeleteWorkItemRequest, UpdateWorkItemRequest, WebSocketMessage,
    WorkItemType as ProtoWorkItemType,
};

use std::panic::Location;

use chrono::Utc;
use error_location::ErrorLocation;
use uuid::Uuid;

/// Convert proto WorkItemType (i32) to domain WorkItemType
fn proto_to_domain_item_type(proto_type: i32) -> Result<WorkItemType, WsError> {
    match proto_type {
        x if x == ProtoWorkItemType::Epic as i32 => Ok(WorkItemType::Epic),
        x if x == ProtoWorkItemType::Story as i32 => Ok(WorkItemType::Story),
        x if x == ProtoWorkItemType::Task as i32 => Ok(WorkItemType::Task),
        _ => Err(WsError::ValidationError {
            message: format!("Invalid item_type: {}", proto_type),
            field: Some("item_type".to_string()),
            location: ErrorLocation::from(Location::caller()),
        }),
    }
}

/// Handle CreateWorkItemRequest with full production features
pub async fn handle_create(
    req: CreateWorkItemRequest,
    ctx: HandlerContext,
) -> WsErrorResult<WebSocketMessage> {
    log::debug!("{} CreateWorkItem starting", ctx.log_prefix());

    // 1. Convert and validate item_type
    let item_type = proto_to_domain_item_type(req.item_type)?;

    // 2. Validate input fields
    MessageValidator::validate_work_item_create(
        &req.title,
        req.description.as_deref(),
        item_type.as_str(),
    )?;

    // 3. Check idempotency BEFORE any mutations (with circuit breaker)
    let cached = db_read(&ctx, "check_idempotency", || async {
        check_idempotency(&ctx.pool, &ctx.message_id).await
    })
    .await?;

    if let Some(cached_response) = cached {
        log::info!("{} Returning cached idempotent response", ctx.log_prefix());
        use base64::Engine;
        use prost::Message as ProstMessage;
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

    // 4. Parse and validate IDs
    let project_id = parse_uuid(&req.project_id, "project_id")?;
    let parent_id = match &req.parent_id {
        Some(id) if !id.is_empty() => Some(parse_uuid(id, "parent_id")?),
        _ => None,
    };

    // 5. Authorization with circuit breaker
    db_read(&ctx, "check_permission", || async {
        check_permission(&ctx, project_id, Permission::Edit).await
    })
    .await?;

    // 6. Validate hierarchy rules
    // If parent_id is provided, validate it points to correct type
    if let Some(pid) = parent_id {
        db_read(&ctx, "validate_hierarchy", || async {
            validate_hierarchy(&ctx.pool, item_type.clone(), pid).await
        })
        .await?;
    }

    // 7. Get next position
    let max_position = db_read(&ctx, "find_max_position", || async {
        WorkItemRepository::find_max_position(&ctx.pool, project_id, parent_id)
            .await
            .map_err(WsError::from)
    })
    .await?;

    // 8. Build work item
    let now = Utc::now();
    let work_item = WorkItem {
        id: Uuid::new_v4(),
        item_type: item_type.clone(),
        parent_id,
        project_id,
        position: max_position + 1,
        title: sanitize_string(&req.title),
        description: req.description.as_ref().map(|d| sanitize_string(d)),
        status: req.status.clone().unwrap_or_else(|| "backlog".to_string()),
        priority: req.priority.clone().unwrap_or_else(|| "medium".to_string()),
        assignee_id: None,
        story_points: None,
        sprint_id: None,
        version: 1,
        created_at: now,
        updated_at: now,
        created_by: ctx.user_id,
        updated_by: ctx.user_id,
        deleted_at: None,
    };

    // 9. Execute transaction with circuit breaker
    let work_item_clone = work_item.clone();
    db_write(&ctx, "create_work_item_tx", || async {
        let mut tx = ctx.pool.begin().await?;

        WorkItemRepository::create(&mut *tx, &work_item_clone).await?;

        let activity = ActivityLog::created("work_item", work_item_clone.id, ctx.user_id);
        ActivityLogRepository::create(&mut *tx, &activity).await?;

        tx.commit().await?;
        Ok::<_, WsError>(())
    })
    .await?;

    // 10. Build response
    let response = build_work_item_created_response(&ctx.message_id, &work_item, ctx.user_id);

    // 11. Store idempotency (after commit, failure here is non-fatal)
    use base64::Engine;
    use prost::Message as ProstMessage;
    let response_bytes = response.encode_to_vec();
    let response_b64 = base64::engine::general_purpose::STANDARD.encode(&response_bytes);

    if let Err(e) = store_idempotency(
        &ctx.pool,
        &ctx.message_id,
        "create_work_item",
        &response_b64,
    )
    .await
    {
        log::warn!(
            "{} Failed to store idempotency (non-fatal): {}",
            ctx.log_prefix(),
            e
        );
    }

    log::info!(
        "{} Created work item {} ({:?}) in project {}",
        ctx.log_prefix(),
        work_item.id,
        item_type,
        project_id
    );

    Ok(response)
}

/// Handle UpdateWorkItemRequest
pub async fn handle_update(
    req: UpdateWorkItemRequest,
    ctx: HandlerContext,
) -> WsErrorResult<WebSocketMessage> {
    log::debug!("{} UpdateWorkItem starting", ctx.log_prefix());

    // 1. Parse work item ID
    let work_item_id = parse_uuid(&req.work_item_id, "work_item_id")?;

    // 2. Fetch existing with circuit breaker
    let mut work_item = db_read(&ctx, "find_work_item", || async {
        WorkItemRepository::find_by_id(&ctx.pool, work_item_id)
            .await
            .map_err(WsError::from)
    })
    .await?
    .ok_or_else(|| WsError::NotFound {
        message: format!("Work item {} not found", work_item_id),
        location: ErrorLocation::from(Location::caller()),
    })?;

    // 3. Authorization
    db_read(&ctx, "check_permission", || async {
        check_permission(&ctx, work_item.project_id, Permission::Edit).await
    })
    .await?;

    // 4. Optimistic locking
    if work_item.version != req.expected_version {
        return Err(WsError::ConflictError {
            current_version: work_item.version,
            location: ErrorLocation::from(Location::caller()),
        });
    }

    // 5. Track changes
    let changes = track_changes(&work_item, &req);

    if changes.is_empty() {
        return Ok(build_work_item_updated_response(
            &ctx.message_id,
            &work_item,
            &changes,
            ctx.user_id,
        ));
    }

    // 6. Apply updates with validation
    apply_updates(&mut work_item, &req)?;

    // 7. Update metadata
    let now = Utc::now();
    work_item.updated_at = now;
    work_item.updated_by = ctx.user_id;
    work_item.version += 1;

    // 8. Transaction with circuit breaker
    let work_item_clone = work_item.clone();
    let changes_clone = changes.clone();
    db_write(&ctx, "update_work_item_tx", || async {
        let mut tx = ctx.pool.begin().await?;

        WorkItemRepository::update(&mut *tx, &work_item_clone).await?;

        let activity =
            ActivityLog::updated("work_item", work_item_clone.id, ctx.user_id, &changes_clone);
        ActivityLogRepository::create(&mut *tx, &activity).await?;

        tx.commit().await?;
        Ok::<_, WsError>(())
    })
    .await?;

    log::info!(
        "{} Updated work item {} (version {})",
        ctx.log_prefix(),
        work_item.id,
        work_item.version
    );

    Ok(build_work_item_updated_response(
        &ctx.message_id,
        &work_item,
        &changes,
        ctx.user_id,
    ))
}

/// Handle DeleteWorkItemRequest
pub async fn handle_delete(
    req: DeleteWorkItemRequest,
    ctx: HandlerContext,
) -> WsErrorResult<WebSocketMessage> {
    log::debug!("{} DeleteWorkItem starting", ctx.log_prefix());

    // 1. Parse work item ID
    let work_item_id = parse_uuid(&req.work_item_id, "work_item_id")?;

    // 2. Fetch existing
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

    // 3. Authorization - Admin required for delete
    db_read(&ctx, "check_permission", || async {
        check_permission(&ctx, work_item.project_id, Permission::Admin).await
    })
    .await?;

    // 4. Check for children
    let children = db_read(&ctx, "find_children", || async {
        WorkItemRepository::find_children(&ctx.pool, work_item_id)
            .await
            .map_err(WsError::from)
    })
    .await?;

    if !children.is_empty() {
        return Err(WsError::DeleteBlocked {
            message: format!(
                "Cannot delete: has {} child item(s). Delete children first.",
                children.len()
            ),
            location: ErrorLocation::from(Location::caller()),
        });
    }

    // 5. Transaction
    db_write(&ctx, "delete_work_item_tx", || async {
        let mut tx = ctx.pool.begin().await?;

        WorkItemRepository::soft_delete(&mut *tx, work_item_id, ctx.user_id).await?;

        let activity = ActivityLog::deleted("work_item", work_item_id, ctx.user_id);
        ActivityLogRepository::create(&mut *tx, &activity).await?;

        tx.commit().await?;
        Ok::<_, WsError>(())
    })
    .await?;

    log::info!("{} Deleted work item {}", ctx.log_prefix(), work_item_id);

    Ok(build_work_item_deleted_response(
        &ctx.message_id,
        work_item_id,
        ctx.user_id,
    ))
}

// === Helper Functions ===

fn parse_uuid(s: &str, field: &str) -> Result<Uuid, WsError> {
    Uuid::parse_str(s).map_err(|_| WsError::ValidationError {
        message: format!("Invalid UUID format for {}", field),
        field: Some(field.to_string()),
        location: ErrorLocation::from(Location::caller()),
    })
}

fn apply_updates(work_item: &mut WorkItem, req: &UpdateWorkItemRequest) -> Result<(), WsError> {
    if let Some(ref title) = req.title {
        MessageValidator::validate_string(title, "title", 1, 200)?;
        work_item.title = sanitize_string(title);
    }
    if let Some(ref desc) = req.description {
        if desc.len() > 10000 {
            return Err(WsError::ValidationError {
                message: "Description exceeds 10000 characters".to_string(),
                field: Some("description".to_string()),
                location: ErrorLocation::from(Location::caller()),
            });
        }
        work_item.description = Some(sanitize_string(desc));
    }
    if let Some(ref status) = req.status {
        validate_status(status)?;
        work_item.status = status.clone();
    }
    if let Some(ref priority) = req.priority {
        validate_priority(priority)?;
        work_item.priority = priority.clone();
    }
    if let Some(ref assignee_id) = req.assignee_id {
        work_item.assignee_id = if assignee_id.is_empty() {
            None
        } else {
            Some(parse_uuid(assignee_id, "assignee_id")?)
        };
    }
    if let Some(ref sprint_id) = req.sprint_id {
        work_item.sprint_id = if sprint_id.is_empty() {
            None
        } else {
            Some(parse_uuid(sprint_id, "sprint_id")?)
        };
    }
    if let Some(position) = req.position {
        if position < 0 {
            return Err(WsError::ValidationError {
                message: "Position must be non-negative".to_string(),
                field: Some("position".to_string()),
                location: ErrorLocation::from(Location::caller()),
            });
        }
        work_item.position = position;
    }
    if let Some(story_points) = req.story_points {
        if !(0..=100).contains(&story_points) {
            return Err(WsError::ValidationError {
                message: "Story points must be 0-100".to_string(),
                field: Some("story_points".to_string()),
                location: ErrorLocation::from(Location::caller()),
            });
        }
        work_item.story_points = Some(story_points);
    }
    Ok(())
}

// NOTE: These functions are `pub` so property tests can access them
pub fn validate_status(status: &str) -> Result<(), WsError> {
    match status {
        "backlog" | "todo" | "in_progress" | "review" | "done" | "blocked" => Ok(()),
        _ => Err(WsError::ValidationError {
            message: format!(
                "Invalid status: {}. Valid: backlog, todo, in_progress, review, done, blocked",
                status
            ),
            field: Some("status".to_string()),
            location: ErrorLocation::from(Location::caller()),
        }),
    }
}

pub fn validate_priority(priority: &str) -> Result<(), WsError> {
    match priority {
        "low" | "medium" | "high" | "critical" => Ok(()),
        _ => Err(WsError::ValidationError {
            message: format!(
                "Invalid priority: {}. Valid: low, medium, high, critical",
                priority
            ),
            field: Some("priority".to_string()),
            location: ErrorLocation::from(Location::caller()),
        }),
    }
}

pub fn sanitize_string(s: &str) -> String {
    s.replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
        .trim()
        .to_string()
}
