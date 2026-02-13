//! WebSocket handlers for Project CRUD operations.

use crate::{
    HandlerContext, MessageValidator, Result as WsResult, WsError, build_project_created_response,
    build_project_deleted_response, build_project_list_response, build_project_updated_response,
    check_idempotency, db_read, db_write, log_handler_entry, sanitize_string, store_idempotency,
};

use pm_core::{ActivityLog, Project, ProjectMember, ProjectStatus};
use pm_db::{
    ActivityLogRepository, ProjectMemberRepository, ProjectRepository, WorkItemRepository,
};
use pm_proto::{
    CreateProjectRequest, DeleteProjectRequest, FieldChange, ListProjectsRequest,
    UpdateProjectRequest, WebSocketMessage,
};

use std::panic::Location;

use chrono::Utc;
use error_location::ErrorLocation;
use uuid::Uuid;

/// Handle CreateProjectRequest
pub async fn handle_create(
    req: CreateProjectRequest,
    ctx: HandlerContext,
) -> WsResult<WebSocketMessage> {
    log_handler_entry!(ctx.request_ctx, "CreateProject");

    // 1. Validate input
    MessageValidator::validate_project_create(
        &req.title,
        req.description.as_deref(),
        &req.key,
        &ctx.validation,
    )?;

    // 2. Check idempotency
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

    // 3. Check key uniqueness (uppercase for comparison)
    let key_upper = req.key.to_uppercase();
    let repo = ProjectRepository::new(ctx.pool.clone());
    let existing = db_read(&ctx, "check_key_unique", || async {
        repo.find_by_key(&key_upper).await.map_err(WsError::from)
    })
    .await?;

    if existing.is_some() {
        return Err(WsError::ValidationError {
            message: format!("Project key '{}' already exists", key_upper),
            field: Some("key".to_string()),
            location: ErrorLocation::from(Location::caller()),
        });
    }

    // 4. Build project
    let project = Project {
        id: Uuid::new_v4(),
        title: sanitize_string(&req.title),
        description: req.description.as_ref().map(|d| sanitize_string(d)),
        key: key_upper,
        status: ProjectStatus::Active,
        version: 1,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        created_by: ctx.user_id,
        updated_by: ctx.user_id,
        deleted_at: None,
        next_work_item_number: 1,
    };

    // 5. Create in database
    let project_clone = project.clone();
    let repo_clone = ProjectRepository::new(ctx.pool.clone());
    db_write(&ctx, "create_project", || async {
        repo_clone
            .create(&project_clone)
            .await
            .map_err(WsError::from)
    })
    .await?;

    // 6. Add creator as project admin
    let member = ProjectMember {
        id: Uuid::new_v4(),
        project_id: project.id,
        user_id: ctx.user_id,
        role: "admin".to_string(),
        created_at: Utc::now(),
    };
    let member_repo = ProjectMemberRepository::new(ctx.pool.clone());
    db_write(&ctx, "add_project_member", || async {
        member_repo.create(&member).await.map_err(WsError::from)
    })
    .await?;

    // 7. Log activity
    let activity = ActivityLog::created("project", project.id, ctx.user_id);
    let pool_clone = ctx.pool.clone();
    db_write(&ctx, "log_create_activity", || async {
        ActivityLogRepository::create(&pool_clone, &activity)
            .await
            .map_err(WsError::from)
    })
    .await?;

    // 8. Build response
    let response = build_project_created_response(&ctx.message_id, &project, ctx.user_id);

    // 9. Store idempotency (after commit, failure here is non-fatal)
    use base64::Engine;
    use prost::Message as ProstMessage;
    let response_bytes = response.encode_to_vec();
    let response_b64 = base64::engine::general_purpose::STANDARD.encode(&response_bytes);

    if let Err(e) =
        store_idempotency(&ctx.pool, &ctx.message_id, "create_project", &response_b64).await
    {
        log::warn!("{} Failed to store idempotency: {}", ctx.log_prefix(), e);
    }

    log::info!(
        "{} Created project {} (key={})",
        ctx.log_prefix(),
        project.id,
        project.key
    );

    Ok(response)
}

/// Handle UpdateProjectRequest
pub async fn handle_update(
    req: UpdateProjectRequest,
    ctx: HandlerContext,
) -> WsResult<WebSocketMessage> {
    log_handler_entry!(ctx.request_ctx, "UpdateProject");

    // 1. Parse + fetch existing
    let project_id = req
        .project_id
        .parse::<Uuid>()
        .map_err(|_| WsError::ValidationError {
            message: "Invalid project_id format".to_string(),
            field: Some("project_id".to_string()),
            location: ErrorLocation::from(Location::caller()),
        })?;

    let repo = ProjectRepository::new(ctx.pool.clone());
    let mut project = db_read(&ctx, "get_project", || async {
        repo.find_by_id(project_id).await.map_err(WsError::from)
    })
    .await?
    .ok_or_else(|| WsError::NotFound {
        message: format!("Project {} not found", project_id),
        location: ErrorLocation::from(Location::caller()),
    })?;

    // 2. Optimistic locking
    if project.version != req.expected_version {
        return Err(WsError::ConflictError {
            current_version: project.version,
            location: ErrorLocation::from(Location::caller()),
        });
    }

    // 3. Validate updates
    if let Some(ref title) = req.title {
        MessageValidator::validate_string(title, "title", 1, ctx.validation.max_title_length)?;
    }
    if let Some(ref desc) = req.description
        && desc.chars().count() > ctx.validation.max_description_length
    {
        return Err(WsError::ValidationError {
            message: format!(
                "description exceeds maximum length ({} characters)",
                ctx.validation.max_description_length
            ),
            field: Some("description".to_string()),
            location: ErrorLocation::from(Location::caller()),
        });
    }

    // 4. Track changes
    let mut changes = Vec::new();

    if let Some(ref new_title) = req.title
        && &project.title != new_title
    {
        changes.push(FieldChange {
            field_name: "title".to_string(),
            old_value: Some(project.title.clone()),
            new_value: Some(new_title.clone()),
        });
        project.title = sanitize_string(new_title);
    }

    if let Some(ref new_desc) = req.description {
        let old_desc = project.description.clone().unwrap_or_default();
        if &old_desc != new_desc {
            changes.push(FieldChange {
                field_name: "description".to_string(),
                old_value: project.description.clone(),
                new_value: Some(new_desc.clone()),
            });
            project.description = Some(sanitize_string(new_desc));
        }
    }

    if let Some(new_status) = req.status {
        let domain_status = match new_status {
            1 => ProjectStatus::Active,
            2 => ProjectStatus::Archived,
            _ => {
                return Err(WsError::ValidationError {
                    message: format!("Invalid status value: {}", new_status),
                    field: Some("status".to_string()),
                    location: ErrorLocation::from(Location::caller()),
                });
            }
        };
        if project.status != domain_status {
            changes.push(FieldChange {
                field_name: "status".to_string(),
                old_value: Some(project.status.as_str().to_string()),
                new_value: Some(domain_status.as_str().to_string()),
            });
            project.status = domain_status;
        }
    }

    // 5. No changes? Return current state
    if changes.is_empty() {
        return Ok(build_project_updated_response(
            &ctx.message_id,
            &project,
            &[],
            ctx.user_id,
        ));
    }

    // 6. Apply metadata updates
    project.updated_at = Utc::now();
    project.updated_by = ctx.user_id;
    project.version += 1;

    // 7. Update database
    let project_clone = project.clone();
    let repo_clone = ProjectRepository::new(ctx.pool.clone());
    db_write(&ctx, "update_project", || async {
        repo_clone
            .update(&project_clone)
            .await
            .map_err(WsError::from)
    })
    .await?;

    // 8. Log activity
    let activity = ActivityLog::updated("project", project.id, ctx.user_id, &changes);
    let pool_clone = ctx.pool.clone();
    db_write(&ctx, "log_update_activity", || async {
        ActivityLogRepository::create(&pool_clone, &activity)
            .await
            .map_err(WsError::from)
    })
    .await?;

    log::info!(
        "{} Updated project {} ({} changes)",
        ctx.log_prefix(),
        project.id,
        changes.len()
    );

    Ok(build_project_updated_response(
        &ctx.message_id,
        &project,
        &changes,
        ctx.user_id,
    ))
}

/// Handle DeleteProjectRequest
pub async fn handle_delete(
    req: DeleteProjectRequest,
    ctx: HandlerContext,
) -> WsResult<WebSocketMessage> {
    log_handler_entry!(ctx.request_ctx, "DeleteProject");

    // 1. Parse project ID
    let project_id = req
        .project_id
        .parse::<Uuid>()
        .map_err(|_| WsError::ValidationError {
            message: "Invalid project_id format".to_string(),
            field: Some("project_id".to_string()),
            location: ErrorLocation::from(Location::caller()),
        })?;

    // 2. Fetch existing
    let project_repo = ProjectRepository::new(ctx.pool.clone());
    let project = db_read(&ctx, "get_project", || async {
        project_repo
            .find_by_id(project_id)
            .await
            .map_err(WsError::from)
    })
    .await?
    .ok_or_else(|| WsError::NotFound {
        message: format!("Project {} not found", project_id),
        location: ErrorLocation::from(Location::caller()),
    })?;

    // 3. Optimistic locking
    if project.version != req.expected_version {
        return Err(WsError::ConflictError {
            current_version: project.version,
            location: ErrorLocation::from(Location::caller()),
        });
    }

    // 4. Check no work items exist
    let pool_clone = ctx.pool.clone();
    let items = db_read(&ctx, "check_work_items", || async {
        WorkItemRepository::find_by_project(&pool_clone, project_id, true)
            .await
            .map_err(WsError::from)
    })
    .await?;

    if !items.is_empty() {
        return Err(WsError::DeleteBlocked {
            message: format!(
                "Cannot delete project: has {} work item(s). Delete or move them first.",
                items.len()
            ),
            location: ErrorLocation::from(Location::caller()),
        });
    }

    // 5. Soft delete
    let deleted_at = Utc::now().timestamp();
    let repo_clone = ProjectRepository::new(ctx.pool.clone());
    db_write(&ctx, "delete_project", || async {
        repo_clone
            .delete(project_id, deleted_at)
            .await
            .map_err(WsError::from)
    })
    .await?;

    // 6. Log activity
    let activity = ActivityLog::deleted("project", project_id, ctx.user_id);
    let pool_clone = ctx.pool.clone();
    db_write(&ctx, "log_delete_activity", || async {
        ActivityLogRepository::create(&pool_clone, &activity)
            .await
            .map_err(WsError::from)
    })
    .await?;

    log::info!(
        "{} Deleted project {} (key={})",
        ctx.log_prefix(),
        project_id,
        project.key
    );

    Ok(build_project_deleted_response(
        &ctx.message_id,
        project_id,
        ctx.user_id,
    ))
}

/// Handle ListProjectsRequest
pub async fn handle_list(
    _req: ListProjectsRequest,
    ctx: HandlerContext,
) -> WsResult<WebSocketMessage> {
    log_handler_entry!(ctx.request_ctx, "ListProjects");

    let repo = ProjectRepository::new(ctx.pool.clone());
    let projects = db_read(&ctx, "list_projects", || async {
        repo.find_all().await.map_err(WsError::from)
    })
    .await?;

    log::debug!("{} Listed {} projects", ctx.log_prefix(), projects.len());

    Ok(build_project_list_response(&ctx.message_id, &projects))
}
