#![allow(dead_code)]

use crate::{
    FieldChangeBuilder, HandlerContext, build_activity_log_created_event,
    build_sprint_created_response, build_sprint_deleted_response, build_sprint_updated_response,
    build_sprints_list_response, check_idempotency, check_permission, db_read, db_write,
    sanitize_string, store_idempotency,
};
use crate::{MessageValidator, Result as WsErrorResult, WsError};

use pm_core::{ActivityLog, Permission, Sprint, SprintStatus};
use pm_db::{ActivityLogRepository, SprintRepository};
use pm_proto::{
    CreateSprintRequest, DeleteSprintRequest, GetSprintsRequest, SprintStatus as ProtoSprintStatus,
    UpdateSprintRequest, WebSocketMessage,
};

use std::panic::Location;

use axum::extract::ws::Message;
use base64::Engine;
use chrono::{DateTime, Utc};
use error_location::ErrorLocation;
use log::{debug, info, warn};
use prost::Message as ProstMessage;
use uuid::Uuid;

fn parse_uuid(s: &str, field: &str) -> WsErrorResult<Uuid> {
    Uuid::parse_str(s).map_err(|_| WsError::ValidationError {
        message: format!("Invalid UUID format for {}", field),
        field: Some(field.to_string()),
        location: ErrorLocation::from(Location::caller()),
    })
}

fn proto_to_domain_status(proto_status: i32) -> WsErrorResult<SprintStatus> {
    match proto_status {
        x if x == ProtoSprintStatus::Planned as i32 => Ok(SprintStatus::Planned),
        x if x == ProtoSprintStatus::Active as i32 => Ok(SprintStatus::Active),
        x if x == ProtoSprintStatus::Completed as i32 => Ok(SprintStatus::Completed),
        x if x == ProtoSprintStatus::Cancelled as i32 => Ok(SprintStatus::Cancelled),
        _ => Err(WsError::ValidationError {
            message: format!("Invalid sprint status: {}", proto_status),
            field: Some("status".to_string()),
            location: ErrorLocation::from(Location::caller()),
        }),
    }
}

/// Handle CreateSprintRequest with full production features
pub async fn handle_create_sprint(
    req: CreateSprintRequest,
    ctx: HandlerContext,
) -> WsErrorResult<WebSocketMessage> {
    debug!("{} CreateSprint starting", ctx.log_prefix());

    // 1. Validate input
    MessageValidator::validate_sprint_create(&req.name, req.start_date, req.end_date)?;

    // 2. Check idempotency
    let cached = db_read(&ctx, "check_idempotency", || async {
        check_idempotency(&ctx.pool, &ctx.message_id).await
    })
    .await?;

    if let Some(cached_response) = cached {
        info!("{} Returning cached idempotent response", ctx.log_prefix());
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

    // 3. Parse IDs
    let project_id = parse_uuid(&req.project_id, "project_id")?;

    // 4. Authorization
    db_read(&ctx, "check_permission", || async {
        check_permission(&ctx, project_id, Permission::Edit).await
    })
    .await?;

    // 5. Parse dates
    let start_date =
        DateTime::from_timestamp(req.start_date, 0).ok_or_else(|| WsError::ValidationError {
            message: "Invalid start_date timestamp".to_string(),
            field: Some("start_date".to_string()),
            location: ErrorLocation::from(Location::caller()),
        })?;
    let end_date =
        DateTime::from_timestamp(req.end_date, 0).ok_or_else(|| WsError::ValidationError {
            message: "Invalid end_date timestamp".to_string(),
            field: Some("end_date".to_string()),
            location: ErrorLocation::from(Location::caller()),
        })?;

    if start_date >= end_date {
        return Err(WsError::ValidationError {
            message: "start_date must be before end_date".to_string(),
            field: Some("start_date".to_string()),
            location: ErrorLocation::from(Location::caller()),
        });
    }

    // 6. Build sprint
    let sprint = Sprint::new(
        project_id,
        sanitize_string(&req.name),
        req.goal.as_ref().map(|g| sanitize_string(g)),
        start_date,
        end_date,
        ctx.user_id,
    );

    // 7. Execute transaction
    let activity = ActivityLog::created("sprint", sprint.id, ctx.user_id);
    let sprint_clone = sprint.clone();
    let activity_clone = activity.clone();
    db_write(&ctx, "create_sprint_tx", || async {
        let repo = SprintRepository::new(ctx.pool.clone());
        repo.create(&sprint_clone).await?;
        ActivityLogRepository::create(&ctx.pool, &activity_clone).await?;
        Ok::<_, WsError>(())
    })
    .await?;

    // 8. Broadcast ActivityLogCreated
    let event = build_activity_log_created_event(&activity);
    let bytes = event.encode_to_vec();
    let message = Message::Binary(bytes.into());
    let project_id_str = sprint.project_id.to_string();
    let sprint_id_str = sprint.id.to_string();
    ctx.registry
        .broadcast_activity_log_created(&project_id_str, None, Some(&sprint_id_str), message)
        .await?;

    // 8b. Broadcast SprintCreated to all project subscribers
    let broadcast =
        build_sprint_created_response(&Uuid::new_v4().to_string(), &sprint, ctx.user_id);
    let broadcast_bytes = broadcast.encode_to_vec();
    if let Err(e) = ctx
        .registry
        .broadcast_to_project(&project_id_str, Message::Binary(broadcast_bytes.into()))
        .await
    {
        warn!(
            "{} Failed to broadcast SprintCreated: {}",
            ctx.log_prefix(),
            e
        );
    }

    // 9. Build response
    let response = build_sprint_created_response(&ctx.message_id, &sprint, ctx.user_id);
    let response_bytes = response.encode_to_vec();
    let response_b64 = base64::engine::general_purpose::STANDARD.encode(&response_bytes);

    if let Err(e) =
        store_idempotency(&ctx.pool, &ctx.message_id, "create_sprint", &response_b64).await
    {
        warn!(
            "{} Failed to store idempotency (non-fatal): {}",
            ctx.log_prefix(),
            e
        );
    }

    info!(
        "{} Created sprint {} in project {}",
        ctx.log_prefix(),
        sprint.id,
        project_id
    );

    Ok(response)
}

/// Handle UpdateSprintRequest with optimistic locking and status transitions
pub async fn handle_update_sprint(
    req: UpdateSprintRequest,
    ctx: HandlerContext,
) -> WsErrorResult<WebSocketMessage> {
    debug!("{} UpdateSprint starting", ctx.log_prefix());

    // 1. Parse sprint ID
    let sprint_id = parse_uuid(&req.sprint_id, "sprint_id")?;

    // 2. Fetch existing
    let repo = SprintRepository::new(ctx.pool.clone());
    let mut sprint = db_read(&ctx, "find_sprint", || async {
        repo.find_by_id(sprint_id).await.map_err(WsError::from)
    })
    .await?
    .ok_or_else(|| WsError::NotFound {
        message: format!("Sprint {} not found", sprint_id),
        location: ErrorLocation::from(Location::caller()),
    })?;

    // 3. Authorization
    db_read(&ctx, "check_permission", || async {
        check_permission(&ctx, sprint.project_id, Permission::Edit).await
    })
    .await?;

    // 4. Optimistic locking - version must match
    if sprint.version != req.expected_version {
        return Err(WsError::ConflictError {
            current_version: sprint.version,
            location: ErrorLocation::from(Location::caller()),
        });
    }

    // 5. Cannot modify completed sprints
    if sprint.status == SprintStatus::Completed {
        return Err(WsError::ConflictError {
            current_version: sprint.version,
            location: ErrorLocation::from(Location::caller()),
        });
    }

    // 6. Track changes and apply updates
    let mut changes = FieldChangeBuilder::new();

    if let Some(ref name) = req.name {
        MessageValidator::validate_string(name, "name", 1, 100)?;
        changes.track("name", &sprint.name, name);
        sprint.name = sanitize_string(name);
    }
    if let Some(ref goal) = req.goal {
        changes.track_option("goal", &sprint.goal, &Some(goal.clone()));
        sprint.goal = Some(sanitize_string(goal));
    }
    if let Some(start_date) = req.start_date {
        let new_start =
            DateTime::from_timestamp(start_date, 0).ok_or_else(|| WsError::ValidationError {
                message: "Invalid start_date".to_string(),
                field: Some("start_date".to_string()),
                location: ErrorLocation::from(Location::caller()),
            })?;
        changes.track(
            "start_date",
            &sprint.start_date.to_rfc3339(),
            &new_start.to_rfc3339(),
        );
        sprint.start_date = new_start;
    }
    if let Some(end_date) = req.end_date {
        let new_end =
            DateTime::from_timestamp(end_date, 0).ok_or_else(|| WsError::ValidationError {
                message: "Invalid end_date timestamp".to_string(),
                field: Some("end_date".to_string()),
                location: ErrorLocation::from(Location::caller()),
            })?;
        changes.track(
            "end_date",
            &sprint.end_date.to_rfc3339(),
            &new_end.to_rfc3339(),
        );
        sprint.end_date = new_end;
    }
    if let Some(status) = req.status {
        let new_status = proto_to_domain_status(status)?;

        // Validate status transition
        validate_sprint_status_transition(&ctx, &sprint, new_status.clone()).await?;

        changes.track(
            "status",
            &sprint.status.as_str().to_string(),
            &new_status.as_str().to_string(),
        );
        sprint.status = new_status;
    }

    let field_changes = changes.build();
    if field_changes.is_empty() {
        return Ok(build_sprint_updated_response(
            &ctx.message_id,
            &sprint,
            &field_changes,
            ctx.user_id,
        ));
    }

    // 7. Update metadata
    sprint.updated_at = Utc::now();
    sprint.updated_by = ctx.user_id;
    sprint.version += 1;

    // 8. Transaction
    let activity = ActivityLog::updated("sprint", sprint.id, ctx.user_id, &field_changes);
    let sprint_clone = sprint.clone();
    let activity_clone = activity.clone();
    db_write(&ctx, "update_sprint_tx", || async {
        let repo = SprintRepository::new(ctx.pool.clone());
        repo.update(&sprint_clone).await?;
        ActivityLogRepository::create(&ctx.pool, &activity_clone).await?;
        Ok::<_, WsError>(())
    })
    .await?;

    // 9. Broadcast ActivityLogCreated
    let event = build_activity_log_created_event(&activity);
    let bytes = event.encode_to_vec();
    let message = Message::Binary(bytes.into());
    let project_id_str = sprint.project_id.to_string();
    let sprint_id_str = sprint.id.to_string();
    ctx.registry
        .broadcast_activity_log_created(&project_id_str, None, Some(&sprint_id_str), message)
        .await?;

    // 9b. Broadcast SprintUpdated to all project subscribers
    let broadcast = build_sprint_updated_response(
        &Uuid::new_v4().to_string(),
        &sprint,
        &field_changes,
        ctx.user_id,
    );
    let broadcast_bytes = broadcast.encode_to_vec();
    if let Err(e) = ctx
        .registry
        .broadcast_to_project(&project_id_str, Message::Binary(broadcast_bytes.into()))
        .await
    {
        warn!(
            "{} Failed to broadcast SprintUpdated: {}",
            ctx.log_prefix(),
            e
        );
    }

    info!(
        "{} Updated sprint {} (version {})",
        ctx.log_prefix(),
        sprint.id,
        sprint.version
    );

    Ok(build_sprint_updated_response(
        &ctx.message_id,
        &sprint,
        &field_changes,
        ctx.user_id,
    ))
}

/// Validate sprint status transitions following the state machine:
/// Planned -> Active -> Completed
/// Planned -> Cancelled
/// Active -> Cancelled
async fn validate_sprint_status_transition(
    ctx: &HandlerContext,
    sprint: &Sprint,
    new_status: SprintStatus,
) -> WsErrorResult<()> {
    let valid = match (sprint.status.clone(), new_status.clone()) {
        (SprintStatus::Planned, SprintStatus::Active) => true,
        (SprintStatus::Planned, SprintStatus::Cancelled) => true,
        (SprintStatus::Active, SprintStatus::Completed) => true,
        (SprintStatus::Active, SprintStatus::Cancelled) => true,
        (from, to) if from == to => true, // No change is always valid
        _ => false,
    };

    if !valid {
        return Err(WsError::ValidationError {
            message: format!(
                "Invalid status transition: {} -> {}",
                sprint.status.as_str(),
                new_status.as_str()
            ),
            field: Some("status".to_string()),
            location: ErrorLocation::from(Location::caller()),
        });
    }

    // Business rule: Only one active sprint per project at a time
    if new_status == SprintStatus::Active && sprint.status != SprintStatus::Active {
        let repo = SprintRepository::new(ctx.pool.clone());
        let active = db_read(ctx, "find_active_sprint", || async {
            repo.find_active_by_project(sprint.project_id)
                .await
                .map_err(WsError::from)
        })
        .await?;

        if active.is_some() {
            return Err(WsError::ConflictError {
                current_version: sprint.version,
                location: ErrorLocation::from(Location::caller()),
            });
        }
    }

    Ok(())
}

/// Handle DeleteSprintRequest (soft delete)
pub async fn handle_delete_sprint(
    req: DeleteSprintRequest,
    ctx: HandlerContext,
) -> WsErrorResult<WebSocketMessage> {
    debug!("{} DeleteSprint starting", ctx.log_prefix());

    let sprint_id = parse_uuid(&req.sprint_id, "sprint_id")?;

    let repo = SprintRepository::new(ctx.pool.clone());
    let sprint = db_read(&ctx, "find_sprint", || async {
        repo.find_by_id(sprint_id).await.map_err(WsError::from)
    })
    .await?
    .ok_or_else(|| WsError::NotFound {
        message: format!("Sprint {} not found", sprint_id),
        location: ErrorLocation::from(Location::caller()),
    })?;

    // Authorization - Admin required for delete
    db_read(&ctx, "check_permission", || async {
        check_permission(&ctx, sprint.project_id, Permission::Admin).await
    })
    .await?;

    // Cannot delete completed sprints (historical data)
    if sprint.status == SprintStatus::Completed {
        return Err(WsError::DeleteBlocked {
            message: "Cannot delete completed sprint".to_string(),
            location: ErrorLocation::from(Location::caller()),
        });
    }

    // Soft delete
    let activity = ActivityLog::deleted("sprint", sprint_id, ctx.user_id);
    let activity_clone = activity.clone();
    db_write(&ctx, "delete_sprint_tx", || async {
        let repo = SprintRepository::new(ctx.pool.clone());
        repo.delete(sprint_id, Utc::now().timestamp()).await?;
        ActivityLogRepository::create(&ctx.pool, &activity_clone).await?;
        Ok::<_, WsError>(())
    })
    .await?;

    // Broadcast ActivityLogCreated
    let event = build_activity_log_created_event(&activity);
    let bytes = event.encode_to_vec();
    let message = Message::Binary(bytes.into());
    let project_id_str = sprint.project_id.to_string();
    let sprint_id_str = sprint_id.to_string();
    ctx.registry
        .broadcast_activity_log_created(&project_id_str, None, Some(&sprint_id_str), message)
        .await?;

    // Broadcast SprintDeleted to all project subscribers
    let broadcast =
        build_sprint_deleted_response(&Uuid::new_v4().to_string(), sprint_id, ctx.user_id);
    let broadcast_bytes = broadcast.encode_to_vec();
    if let Err(e) = ctx
        .registry
        .broadcast_to_project(&project_id_str, Message::Binary(broadcast_bytes.into()))
        .await
    {
        warn!(
            "{} Failed to broadcast SprintDeleted: {}",
            ctx.log_prefix(),
            e
        );
    }

    info!("{} Deleted sprint {}", ctx.log_prefix(), sprint_id);

    Ok(build_sprint_deleted_response(
        &ctx.message_id,
        sprint_id,
        ctx.user_id,
    ))
}

/// Handle GetSprintsRequest - list sprints for a project
pub async fn handle_get_sprints(
    req: GetSprintsRequest,
    ctx: HandlerContext,
) -> WsErrorResult<WebSocketMessage> {
    debug!("{} GetSprints starting", ctx.log_prefix());

    let project_id = parse_uuid(&req.project_id, "project_id")?;

    // Authorization - View permission required
    db_read(&ctx, "check_permission", || async {
        check_permission(&ctx, project_id, Permission::View).await
    })
    .await?;

    let repo = SprintRepository::new(ctx.pool.clone());
    let sprints = db_read(&ctx, "find_sprints", || async {
        repo.find_by_project(project_id)
            .await
            .map_err(WsError::from)
    })
    .await?;

    info!(
        "{} Found {} sprints for project {}",
        ctx.log_prefix(),
        sprints.len(),
        project_id
    );

    Ok(build_sprints_list_response(&ctx.message_id, sprints))
}
