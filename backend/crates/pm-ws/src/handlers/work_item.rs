use crate::{
    HandlerContext, MessageValidator, Result as WsErrorResult, WsError,
    build_activity_log_created_event, build_work_item_created_response,
    build_work_item_deleted_response, build_work_item_updated_response, check_idempotency,
    check_permission, db_read, db_write, store_idempotency, track_changes, validate_hierarchy,
};

use pm_config::ValidationConfig;
use pm_core::{ActivityLog, Permission, WorkItem, WorkItemType};
use pm_db::{ActivityLogRepository, ProjectRepository, WorkItemRepository};
use pm_proto::{
    CreateWorkItemRequest, DeleteWorkItemRequest, UpdateWorkItemRequest, WebSocketMessage,
    WorkItemType as ProtoWorkItemType,
};

use std::panic::Location;

use axum::extract::ws::Message;
use base64::Engine;
use chrono::Utc;
use error_location::ErrorLocation;
use log::{debug, info, warn};
use prost::Message as ProstMessage;
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
    debug!("{} CreateWorkItem starting", ctx.log_prefix());

    // 1. Convert and validate item_type
    let item_type = proto_to_domain_item_type(req.item_type)?;

    // 2. Validate input fields
    MessageValidator::validate_work_item_create(
        &req.title,
        req.description.as_deref(),
        item_type.as_str(),
        &ctx.validation,
    )?;

    // 3. Check idempotency BEFORE any mutations (with circuit breaker)
    let cached = db_read(&ctx, "check_idempotency", || async {
        check_idempotency(&ctx.pool, &ctx.message_id).await
    })
    .await?;

    if let Some(cached_response) = cached {
        info!("{} Returning cached idempotent response", ctx.log_prefix());
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
    let mut work_item = WorkItem {
        id: Uuid::new_v4(),
        item_type: item_type.clone(),
        parent_id,
        project_id,
        position: (max_position + 1) as i32,
        title: sanitize_string(&req.title),
        description: req.description.as_ref().map(|d| sanitize_string(d)),
        status: req.status.clone().unwrap_or_else(|| "backlog".to_string()),
        priority: req.priority.clone().unwrap_or_else(|| "medium".to_string()),
        assignee_id: None,
        story_points: None,
        sprint_id: None,
        item_number: 0,
        version: 1,
        created_at: now,
        updated_at: now,
        created_by: ctx.user_id,
        updated_by: ctx.user_id,
        deleted_at: None,
    };

    // 9. Execute transaction with atomic item_number assignment
    let activity = ActivityLog::created("work_item", work_item.id, ctx.user_id);
    let activity_clone = activity.clone();

    let work_item_for_tx = work_item.clone();

    let item_number = db_write(&ctx, "create_work_item_tx", || async {
        let mut tx = ctx.pool.begin().await?;

        // Atomically get and increment the work item number
        let item_num = ProjectRepository::get_and_increment_work_item_number(&mut tx, project_id)
            .await
            .map_err(WsError::from)?;

        // Create work item with assigned number
        let mut wi = work_item_for_tx.clone();
        wi.item_number = item_num;
        WorkItemRepository::create(&mut *tx, &wi).await?;

        // Create activity log
        ActivityLogRepository::create(&mut *tx, &activity_clone).await?;

        tx.commit().await?;
        Ok::<_, WsError>(item_num)
    })
    .await?;

    // Update our local copy with the assigned number
    work_item.item_number = item_number;

    // 10. Broadcast ActivityLogCreated
    let event = build_activity_log_created_event(&activity);
    let bytes = event.encode_to_vec();
    let message = Message::Binary(bytes.into());
    let project_id_str = work_item.project_id.to_string();
    let work_item_id_str = work_item.id.to_string();
    ctx.registry
        .broadcast_activity_log_created(&project_id_str, Some(&work_item_id_str), None, message)
        .await?;

    // 10b. Broadcast WorkItemCreated to all project subscribers
    let broadcast =
        build_work_item_created_response(&Uuid::new_v4().to_string(), &work_item, ctx.user_id);
    let broadcast_bytes = broadcast.encode_to_vec();
    if let Err(e) = ctx
        .registry
        .broadcast_to_project(&project_id_str, Message::Binary(broadcast_bytes.into()))
        .await
    {
        warn!(
            "{} Failed to broadcast WorkItemCreated: {}",
            ctx.log_prefix(),
            e
        );
    }

    // 11. Build response
    let response = build_work_item_created_response(&ctx.message_id, &work_item, ctx.user_id);

    // 12. Store idempotency (after commit, failure here is non-fatal)
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
        warn!(
            "{} Failed to store idempotency (non-fatal): {}",
            ctx.log_prefix(),
            e
        );
    }

    info!(
        "{} Created work item {} ({:?}) #{} in project {}",
        ctx.log_prefix(),
        work_item.id,
        item_type,
        item_number,
        project_id
    );

    Ok(response)
}

/// Handle UpdateWorkItemRequest
pub async fn handle_update(
    req: UpdateWorkItemRequest,
    ctx: HandlerContext,
) -> WsErrorResult<WebSocketMessage> {
    debug!("{} UpdateWorkItem starting", ctx.log_prefix());

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

    // 4b. Validate new parent if changing (uses update_parent flag)
    #[allow(clippy::collapsible_if)]
    if req.update_parent {
        if let Some(ref new_parent_id) = req.parent_id {
            if !new_parent_id.is_empty() {
                let parent_uuid = parse_uuid(new_parent_id, "parent_id")?;

                // Can't be your own parent
                if parent_uuid == work_item.id {
                    return Err(WsError::ValidationError {
                        message: format!("Work item {} cannot be its own parent", work_item.id),
                        field: Some("parent_id".to_string()),
                        location: ErrorLocation::from(Location::caller()),
                    });
                }

                // Validate hierarchy rules
                db_read(&ctx, "validate_hierarchy", || async {
                    validate_hierarchy(&ctx.pool, work_item.item_type.clone(), parent_uuid).await
                })
                .await?;

                // Prevent circular references (parent can't be a descendant)
                db_read(&ctx, "check_circular", || async {
                    check_circular_reference(&ctx.pool, work_item.id, parent_uuid).await
                })
                .await?;
            }
        }
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
    apply_updates(&mut work_item, &req, &ctx.validation)?;

    // 7. Update metadata
    let now = Utc::now();
    work_item.updated_at = now;
    work_item.updated_by = ctx.user_id;
    work_item.version += 1;

    // 8. Transaction with circuit breaker
    let activity = ActivityLog::updated("work_item", work_item.id, ctx.user_id, &changes);
    let work_item_clone = work_item.clone();
    let activity_clone = activity.clone();
    db_write(&ctx, "update_work_item_tx", || async {
        let mut tx = ctx.pool.begin().await?;

        WorkItemRepository::update(&mut *tx, &work_item_clone).await?;
        ActivityLogRepository::create(&mut *tx, &activity_clone).await?;

        tx.commit().await?;
        Ok::<_, WsError>(())
    })
    .await?;

    // 9. Broadcast ActivityLogCreated
    let event = build_activity_log_created_event(&activity);
    let bytes = event.encode_to_vec();
    let message = Message::Binary(bytes.into());
    let project_id_str = work_item.project_id.to_string();
    let work_item_id_str = work_item.id.to_string();
    ctx.registry
        .broadcast_activity_log_created(&project_id_str, Some(&work_item_id_str), None, message)
        .await?;

    // 9b. Broadcast WorkItemUpdated to all project subscribers
    let broadcast = build_work_item_updated_response(
        &Uuid::new_v4().to_string(),
        &work_item,
        &changes,
        ctx.user_id,
    );
    let broadcast_bytes = broadcast.encode_to_vec();
    if let Err(e) = ctx
        .registry
        .broadcast_to_project(&project_id_str, Message::Binary(broadcast_bytes.into()))
        .await
    {
        warn!(
            "{} Failed to broadcast WorkItemUpdated: {}",
            ctx.log_prefix(),
            e
        );
    }

    info!(
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
    debug!("{} DeleteWorkItem starting", ctx.log_prefix());

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
    let activity = ActivityLog::deleted("work_item", work_item_id, ctx.user_id);
    let activity_clone = activity.clone();
    db_write(&ctx, "delete_work_item_tx", || async {
        let mut tx = ctx.pool.begin().await?;

        WorkItemRepository::soft_delete(&mut *tx, work_item_id, ctx.user_id).await?;
        ActivityLogRepository::create(&mut *tx, &activity_clone).await?;

        tx.commit().await?;
        Ok::<_, WsError>(())
    })
    .await?;

    // 6. Broadcast ActivityLogCreated
    let event = build_activity_log_created_event(&activity);
    let bytes = event.encode_to_vec();
    let message = Message::Binary(bytes.into());
    let project_id_str = work_item.project_id.to_string();
    let work_item_id_str = work_item_id.to_string();
    ctx.registry
        .broadcast_activity_log_created(&project_id_str, Some(&work_item_id_str), None, message)
        .await?;

    // 6b. Broadcast WorkItemDeleted to all project subscribers
    let broadcast =
        build_work_item_deleted_response(&Uuid::new_v4().to_string(), work_item_id, ctx.user_id);
    let broadcast_bytes = broadcast.encode_to_vec();
    if let Err(e) = ctx
        .registry
        .broadcast_to_project(&project_id_str, Message::Binary(broadcast_bytes.into()))
        .await
    {
        warn!(
            "{} Failed to broadcast WorkItemDeleted: {}",
            ctx.log_prefix(),
            e
        );
    }

    info!("{} Deleted work item {}", ctx.log_prefix(), work_item_id);

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

fn apply_updates(
    work_item: &mut WorkItem,
    req: &UpdateWorkItemRequest,
    validation: &ValidationConfig,
) -> Result<(), WsError> {
    if let Some(ref title) = req.title {
        MessageValidator::validate_string(title, "title", 1, validation.max_title_length)?;
        work_item.title = sanitize_string(title);
    }
    if let Some(ref desc) = req.description {
        if desc.chars().count() > validation.max_description_length {
            return Err(WsError::ValidationError {
                message: format!(
                    "Description exceeds {} characters",
                    validation.max_description_length
                ),
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
    // Handle parent_id change (uses update_parent flag)
    if req.update_parent {
        work_item.parent_id = if let Some(parent_id) = req.parent_id.as_ref() {
            if parent_id.is_empty() {
                None // Clear parent (make orphan)
            } else {
                Some(parse_uuid(parent_id, "parent_id")?) // Set parent
            }
        } else {
            None // No value means clear parent
        };
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
    s.trim().to_string()
}

/// Check that new_parent is not a descendant of work_item (prevent cycles)
///
/// Walks up the parent chain from new_parent_id to ensure work_item_id is not
/// in the ancestry. This prevents circular references like:
/// - A -> B -> C -> A (would create a cycle)
///
/// Also prevents excessively deep hierarchies (max 100 levels).
async fn check_circular_reference(
    pool: &sqlx::SqlitePool,
    work_item_id: Uuid,
    new_parent_id: Uuid,
) -> WsErrorResult<()> {
    // Walk up the tree from new_parent to ensure we don't hit work_item_id
    let mut current = Some(new_parent_id);
    let mut depth = 0;
    const MAX_DEPTH: i32 = 100; // Prevent infinite loops

    while let Some(id) = current {
        if depth > MAX_DEPTH {
            return Err(WsError::ValidationError {
                message: format!(
                    "Hierarchy too deep: work item {} has parent chain exceeding {} levels. \
                       Max depth is {}. This may indicate corrupted data or a cycle.",
                    new_parent_id, depth, MAX_DEPTH
                ),
                field: Some("parent_id".to_string()),
                location: ErrorLocation::from(Location::caller()),
            });
        }

        if id == work_item_id {
            return Err(WsError::ValidationError {
                message: format!(
                    "Cannot set parent {} for work item {}: would create circular reference. \
                       Work item {} is a descendant of work item {}, so {} cannot be its parent.",
                    new_parent_id, work_item_id, new_parent_id, work_item_id, new_parent_id
                ),
                field: Some("parent_id".to_string()),
                location: ErrorLocation::from(Location::caller()),
            });
        }

        let item = WorkItemRepository::find_by_id(pool, id).await?;
        current = item.and_then(|i| i.parent_id);
        depth += 1;
    }

    Ok(())
}
