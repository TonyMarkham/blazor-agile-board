# Session 50: Sprints & Comments - ACTUAL Implementation Plan

**Total estimated lines of code: ~1,850 lines**
- Backend Rust: ~650 lines
- Frontend C#: ~850 lines
- Proto changes: ~50 lines
- Tests: ~300 lines

---

## Phase 1: Proto Schema Updates (~50 lines)

### File: `proto/messages.proto`

**Add version field to Sprint message (line 88, after status):**
```protobuf
  int32 version = 13;
```

**Add expected_version and status to UpdateSprintRequest (after line 354):**
```protobuf
message UpdateSprintRequest {
  string sprint_id = 1;
  int32 expected_version = 2;
  optional string name = 3;
  optional string goal = 4;
  optional int64 start_date = 5;
  optional int64 end_date = 6;
  optional SprintStatus status = 7;
}
```

**Add changes to SprintUpdated (line 373-376):**
```protobuf
message SprintUpdated {
  Sprint sprint = 1;
  repeated FieldChange changes = 2;
  string user_id = 3;
}
```

**Add Get/List messages (after line 381):**
```protobuf
message GetSprintsRequest {
  string project_id = 1;
}

message SprintsList {
  repeated Sprint sprints = 1;
}

message GetCommentsRequest {
  string work_item_id = 1;
}

message CommentsList {
  repeated Comment comments = 1;
}
```

**Add to WebSocketMessage oneof (after line 245):**
```protobuf
    GetSprintsRequest get_sprints_request = 53;
    SprintsList sprints_list = 63;
    GetCommentsRequest get_comments_request = 73;
    CommentsList comments_list = 83;
```

---

## Phase 2: Backend Sprint Model Update (~20 lines)

### File: `backend/crates/pm-core/src/models/sprint.rs`

**Add version field to struct (after line 18):**
```rust
    pub version: i32,
```

**Update `new()` constructor (line 38, add before created_at):**
```rust
            version: 1,
```

---

## Phase 3: Backend Sprint Repository Update (~40 lines)

### File: `backend/crates/pm-db/src/repositories/sprint_repository.rs`

**Update create() INSERT to include version (line 34-38):**
```rust
        sqlx::query!(
            r#"
              INSERT INTO pm_sprints (
                  id, project_id, name, goal,
                  start_date, end_date, status, version,
                  created_at, updated_at, created_by, updated_by, deleted_at
              ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
              "#,
            id,
            project_id,
            sprint.name,
            sprint.goal,
            start_date,
            end_date,
            status,
            sprint.version,
            created_at,
            updated_at,
            created_by,
            updated_by,
            deleted_at,
        )
```

**Update find_by_id() SELECT to include version (line 62-66):**
```rust
        let row = sqlx::query!(
            r#"
              SELECT id, project_id, name, goal, start_date, end_date, status, version,
                     created_at, updated_at, created_by, updated_by, deleted_at
              FROM pm_sprints
              WHERE id = ? AND deleted_at IS NULL
              "#,
            id_str
        )
```

**Update find_by_id() mapping to include version (line 74-87):**
```rust
        Ok(row.map(|r| Sprint {
            id: Uuid::parse_str(r.id.as_ref().unwrap()).unwrap(),
            project_id: Uuid::parse_str(&r.project_id).unwrap(),
            name: r.name,
            goal: r.goal,
            start_date: DateTime::from_timestamp(r.start_date, 0).unwrap(),
            end_date: DateTime::from_timestamp(r.end_date, 0).unwrap(),
            status: SprintStatus::from_str(&r.status).unwrap(),
            version: r.version,
            created_at: DateTime::from_timestamp(r.created_at, 0).unwrap(),
            updated_at: DateTime::from_timestamp(r.updated_at, 0).unwrap(),
            created_by: Uuid::parse_str(&r.created_by).unwrap(),
            updated_by: Uuid::parse_str(&r.updated_by).unwrap(),
            deleted_at: r.deleted_at.and_then(|ts| DateTime::from_timestamp(ts, 0)),
        }))
```

**Same changes for find_by_project() SELECT and mapping.**

**Add find_active_by_project() method (after find_by_project, ~line 124):**
```rust
    pub async fn find_active_by_project(&self, project_id: Uuid) -> DbErrorResult<Option<Sprint>> {
        let project_id_str = project_id.to_string();
        let active_status = SprintStatus::Active.as_str();

        let row = sqlx::query!(
            r#"
              SELECT id, project_id, name, goal, start_date, end_date, status, version,
                     created_at, updated_at, created_by, updated_by, deleted_at
              FROM pm_sprints
              WHERE project_id = ? AND status = ? AND deleted_at IS NULL
              "#,
            project_id_str,
            active_status
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| Sprint {
            id: Uuid::parse_str(r.id.as_ref().unwrap()).unwrap(),
            project_id: Uuid::parse_str(&r.project_id).unwrap(),
            name: r.name,
            goal: r.goal,
            start_date: DateTime::from_timestamp(r.start_date, 0).unwrap(),
            end_date: DateTime::from_timestamp(r.end_date, 0).unwrap(),
            status: SprintStatus::from_str(&r.status).unwrap(),
            version: r.version,
            created_at: DateTime::from_timestamp(r.created_at, 0).unwrap(),
            updated_at: DateTime::from_timestamp(r.updated_at, 0).unwrap(),
            created_by: Uuid::parse_str(&r.created_by).unwrap(),
            updated_by: Uuid::parse_str(&r.updated_by).unwrap(),
            deleted_at: r.deleted_at.and_then(|ts| DateTime::from_timestamp(ts, 0)),
        }))
    }
```

**Update update() to include version (line 134-140):**
```rust
        sqlx::query!(
            r#"
              UPDATE pm_sprints
              SET project_id = ?, name = ?, goal = ?,
                  start_date = ?, end_date = ?, status = ?, version = ?,
                  updated_at = ?, updated_by = ?
              WHERE id = ? AND deleted_at IS NULL
              "#,
            project_id,
            sprint.name,
            sprint.goal,
            start_date,
            end_date,
            status,
            sprint.version,
            updated_at,
            updated_by,
            id,
        )
```

---

## Phase 4: Backend Sprint Handler (~200 lines)

### File: `backend/crates/pm-ws/src/handlers/sprint.rs` (NEW)

```rust
use crate::handlers::{
    authorization::check_permission,
    change_tracker::FieldChangeBuilder,
    context::HandlerContext,
    db_ops::{db_read, db_write},
    idempotency::{check_idempotency, store_idempotency},
    response_builder::*,
    work_item::sanitize_string,
};
use crate::{MessageValidator, Result as WsErrorResult, WsError};

use pm_core::{ActivityLog, Permission, Sprint, SprintStatus};
use pm_db::{ActivityLogRepository, SprintRepository};
use pm_proto::{
    CreateSprintRequest, DeleteSprintRequest, GetSprintsRequest, UpdateSprintRequest,
    WebSocketMessage, SprintStatus as ProtoSprintStatus,
};

use std::panic::Location;
use std::str::FromStr;

use chrono::{DateTime, Utc};
use error_location::ErrorLocation;
use uuid::Uuid;

fn parse_uuid(s: &str, field: &str) -> Result<Uuid, WsError> {
    Uuid::parse_str(s).map_err(|_| WsError::ValidationError {
        message: format!("Invalid UUID format for {}", field),
        field: Some(field.to_string()),
        location: ErrorLocation::from(Location::caller()),
    })
}

fn proto_to_domain_status(proto_status: i32) -> Result<SprintStatus, WsError> {
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

pub async fn handle_create_sprint(
    req: CreateSprintRequest,
    ctx: HandlerContext,
) -> WsErrorResult<WebSocketMessage> {
    log::debug!("{} CreateSprint starting", ctx.log_prefix());

    // 1. Validate input
    MessageValidator::validate_sprint_create(&req.name, req.goal.as_deref())?;

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

    // 3. Parse IDs
    let project_id = parse_uuid(&req.project_id, "project_id")?;

    // 4. Authorization
    db_read(&ctx, "check_permission", || async {
        check_permission(&ctx, project_id, Permission::Edit).await
    })
    .await?;

    // 5. Parse dates
    let start_date = DateTime::from_timestamp(req.start_date, 0).ok_or_else(|| {
        WsError::ValidationError {
            message: "Invalid start_date timestamp".to_string(),
            field: Some("start_date".to_string()),
            location: ErrorLocation::from(Location::caller()),
        }
    })?;
    let end_date = DateTime::from_timestamp(req.end_date, 0).ok_or_else(|| {
        WsError::ValidationError {
            message: "Invalid end_date timestamp".to_string(),
            field: Some("end_date".to_string()),
            location: ErrorLocation::from(Location::caller()),
        }
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
    let sprint_clone = sprint.clone();
    db_write(&ctx, "create_sprint_tx", || async {
        let repo = SprintRepository::new(ctx.pool.clone());
        repo.create(&sprint_clone).await?;

        let activity = ActivityLog::created("sprint", sprint_clone.id, ctx.user_id);
        ActivityLogRepository::create(&ctx.pool, &activity).await?;

        Ok::<_, WsError>(())
    })
    .await?;

    // 8. Build response
    let response = build_sprint_created_response(&ctx.message_id, &sprint, ctx.user_id);

    // 9. Store idempotency
    use base64::Engine;
    use prost::Message as ProstMessage;
    let response_bytes = response.encode_to_vec();
    let response_b64 = base64::engine::general_purpose::STANDARD.encode(&response_bytes);

    if let Err(e) = store_idempotency(&ctx.pool, &ctx.message_id, "create_sprint", &response_b64).await {
        log::warn!("{} Failed to store idempotency (non-fatal): {}", ctx.log_prefix(), e);
    }

    log::info!("{} Created sprint {} in project {}", ctx.log_prefix(), sprint.id, project_id);

    Ok(response)
}

pub async fn handle_update_sprint(
    req: UpdateSprintRequest,
    ctx: HandlerContext,
) -> WsErrorResult<WebSocketMessage> {
    log::debug!("{} UpdateSprint starting", ctx.log_prefix());

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

    // 4. Optimistic locking
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
        let new_start = DateTime::from_timestamp(start_date, 0).ok_or_else(|| {
            WsError::ValidationError {
                message: "Invalid start_date".to_string(),
                field: Some("start_date".to_string()),
                location: ErrorLocation::from(Location::caller()),
            }
        })?;
        changes.track("start_date", &sprint.start_date.to_rfc3339(), &new_start.to_rfc3339());
        sprint.start_date = new_start;
    }
    if let Some(end_date) = req.end_date {
        let new_end = DateTime::from_timestamp(end_date, 0).ok_or_else(|| {
            WsError::ValidationError {
                message: "Invalid end_date".to_string(),
                field: Some("end_date".to_string()),
                location: ErrorLocation::from(Location::caller()),
            }
        })?;
        changes.track("end_date", &sprint.end_date.to_rfc3339(), &new_end.to_rfc3339());
        sprint.end_date = new_end;
    }
    if let Some(status) = req.status {
        let new_status = proto_to_domain_status(status)?;

        // Validate status transition
        validate_sprint_status_transition(&ctx, &sprint, new_status).await?;

        changes.track("status", sprint.status.as_str(), new_status.as_str());
        sprint.status = new_status;
    }

    let field_changes = changes.build();
    if field_changes.is_empty() {
        return Ok(build_sprint_updated_response(&ctx.message_id, &sprint, &field_changes, ctx.user_id));
    }

    // 7. Update metadata
    sprint.updated_at = Utc::now();
    sprint.updated_by = ctx.user_id;
    sprint.version += 1;

    // 8. Transaction
    let sprint_clone = sprint.clone();
    let changes_clone = field_changes.clone();
    db_write(&ctx, "update_sprint_tx", || async {
        let repo = SprintRepository::new(ctx.pool.clone());
        repo.update(&sprint_clone).await?;

        let activity = ActivityLog::updated("sprint", sprint_clone.id, ctx.user_id, &changes_clone);
        ActivityLogRepository::create(&ctx.pool, &activity).await?;

        Ok::<_, WsError>(())
    })
    .await?;

    log::info!("{} Updated sprint {} (version {})", ctx.log_prefix(), sprint.id, sprint.version);

    Ok(build_sprint_updated_response(&ctx.message_id, &sprint, &field_changes, ctx.user_id))
}

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
        (from, to) if from == to => true, // No change
        _ => false,
    };

    if !valid {
        return Err(WsError::ValidationError {
            message: format!("Invalid status transition: {} -> {}", sprint.status.as_str(), new_status.as_str()),
            field: Some("status".to_string()),
            location: ErrorLocation::from(Location::caller()),
        });
    }

    // Check for existing active sprint when activating
    if new_status == SprintStatus::Active && sprint.status != SprintStatus::Active {
        let repo = SprintRepository::new(ctx.pool.clone());
        let active = db_read(ctx, "find_active_sprint", || async {
            repo.find_active_by_project(sprint.project_id).await.map_err(WsError::from)
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

pub async fn handle_delete_sprint(
    req: DeleteSprintRequest,
    ctx: HandlerContext,
) -> WsErrorResult<WebSocketMessage> {
    log::debug!("{} DeleteSprint starting", ctx.log_prefix());

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

    // Authorization - Admin required
    db_read(&ctx, "check_permission", || async {
        check_permission(&ctx, sprint.project_id, Permission::Admin).await
    })
    .await?;

    // Cannot delete completed sprints
    if sprint.status == SprintStatus::Completed {
        return Err(WsError::DeleteBlocked {
            message: "Cannot delete completed sprint".to_string(),
            location: ErrorLocation::from(Location::caller()),
        });
    }

    // Soft delete
    db_write(&ctx, "delete_sprint_tx", || async {
        let repo = SprintRepository::new(ctx.pool.clone());
        repo.delete(sprint_id, Utc::now().timestamp()).await?;

        let activity = ActivityLog::deleted("sprint", sprint_id, ctx.user_id);
        ActivityLogRepository::create(&ctx.pool, &activity).await?;

        Ok::<_, WsError>(())
    })
    .await?;

    log::info!("{} Deleted sprint {}", ctx.log_prefix(), sprint_id);

    Ok(build_sprint_deleted_response(&ctx.message_id, sprint_id, ctx.user_id))
}

pub async fn handle_get_sprints(
    req: GetSprintsRequest,
    ctx: HandlerContext,
) -> WsErrorResult<WebSocketMessage> {
    log::debug!("{} GetSprints starting", ctx.log_prefix());

    let project_id = parse_uuid(&req.project_id, "project_id")?;

    // Authorization
    db_read(&ctx, "check_permission", || async {
        check_permission(&ctx, project_id, Permission::View).await
    })
    .await?;

    let repo = SprintRepository::new(ctx.pool.clone());
    let sprints = db_read(&ctx, "find_sprints", || async {
        repo.find_by_project(project_id).await.map_err(WsError::from)
    })
    .await?;

    log::info!("{} Found {} sprints for project {}", ctx.log_prefix(), sprints.len(), project_id);

    Ok(build_sprints_list_response(&ctx.message_id, sprints))
}
```

---

## Phase 5: Backend Comment Handler (~180 lines)

### File: `backend/crates/pm-ws/src/handlers/comment.rs` (NEW)

```rust
use crate::handlers::{
    authorization::check_permission,
    context::HandlerContext,
    db_ops::{db_read, db_write},
    idempotency::{check_idempotency, store_idempotency},
    response_builder::*,
    work_item::sanitize_string,
};
use crate::{MessageValidator, Result as WsErrorResult, WsError};

use pm_core::{ActivityLog, Comment, Permission};
use pm_db::{ActivityLogRepository, CommentRepository, WorkItemRepository};
use pm_proto::{
    CreateCommentRequest, DeleteCommentRequest, GetCommentsRequest, UpdateCommentRequest,
    WebSocketMessage,
};

use std::panic::Location;

use chrono::Utc;
use error_location::ErrorLocation;
use uuid::Uuid;

fn parse_uuid(s: &str, field: &str) -> Result<Uuid, WsError> {
    Uuid::parse_str(s).map_err(|_| WsError::ValidationError {
        message: format!("Invalid UUID format for {}", field),
        field: Some(field.to_string()),
        location: ErrorLocation::from(Location::caller()),
    })
}

pub async fn handle_create_comment(
    req: CreateCommentRequest,
    ctx: HandlerContext,
) -> WsErrorResult<WebSocketMessage> {
    log::debug!("{} CreateComment starting", ctx.log_prefix());

    // 1. Validate content
    MessageValidator::validate_comment(&req.content)?;

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

    // 3. Parse work_item_id
    let work_item_id = parse_uuid(&req.work_item_id, "work_item_id")?;

    // 4. Verify work item exists and get project_id for authorization
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

    // 5. Authorization
    db_read(&ctx, "check_permission", || async {
        check_permission(&ctx, work_item.project_id, Permission::Edit).await
    })
    .await?;

    // 6. Create comment
    let comment = Comment::new(work_item_id, sanitize_string(&req.content), ctx.user_id);

    // 7. Execute transaction
    let comment_clone = comment.clone();
    db_write(&ctx, "create_comment_tx", || async {
        let repo = CommentRepository::new(ctx.pool.clone());
        repo.create(&comment_clone).await?;

        let activity = ActivityLog::created("comment", comment_clone.id, ctx.user_id);
        ActivityLogRepository::create(&ctx.pool, &activity).await?;

        Ok::<_, WsError>(())
    })
    .await?;

    // 8. Build response
    let response = build_comment_created_response(&ctx.message_id, &comment, ctx.user_id);

    // 9. Store idempotency
    use base64::Engine;
    use prost::Message as ProstMessage;
    let response_bytes = response.encode_to_vec();
    let response_b64 = base64::engine::general_purpose::STANDARD.encode(&response_bytes);

    if let Err(e) = store_idempotency(&ctx.pool, &ctx.message_id, "create_comment", &response_b64).await {
        log::warn!("{} Failed to store idempotency (non-fatal): {}", ctx.log_prefix(), e);
    }

    log::info!("{} Created comment {} on work item {}", ctx.log_prefix(), comment.id, work_item_id);

    Ok(response)
}

pub async fn handle_update_comment(
    req: UpdateCommentRequest,
    ctx: HandlerContext,
) -> WsErrorResult<WebSocketMessage> {
    log::debug!("{} UpdateComment starting", ctx.log_prefix());

    // 1. Validate content
    MessageValidator::validate_comment(&req.content)?;

    // 2. Parse comment ID
    let comment_id = parse_uuid(&req.comment_id, "comment_id")?;

    // 3. Fetch existing
    let repo = CommentRepository::new(ctx.pool.clone());
    let mut comment = db_read(&ctx, "find_comment", || async {
        repo.find_by_id(comment_id).await.map_err(WsError::from)
    })
    .await?
    .ok_or_else(|| WsError::NotFound {
        message: format!("Comment {} not found", comment_id),
        location: ErrorLocation::from(Location::caller()),
    })?;

    // 4. Only author can edit their own comments
    if comment.created_by != ctx.user_id {
        return Err(WsError::Unauthorized {
            message: "Cannot edit another user's comment".to_string(),
            location: ErrorLocation::from(Location::caller()),
        });
    }

    // 5. Apply update
    comment.content = sanitize_string(&req.content);
    comment.updated_at = Utc::now();
    comment.updated_by = ctx.user_id;

    // 6. Transaction
    let comment_clone = comment.clone();
    db_write(&ctx, "update_comment_tx", || async {
        let repo = CommentRepository::new(ctx.pool.clone());
        repo.update(&comment_clone).await?;

        let activity = ActivityLog::updated("comment", comment_clone.id, ctx.user_id, &[]);
        ActivityLogRepository::create(&ctx.pool, &activity).await?;

        Ok::<_, WsError>(())
    })
    .await?;

    log::info!("{} Updated comment {}", ctx.log_prefix(), comment.id);

    Ok(build_comment_updated_response(&ctx.message_id, &comment, ctx.user_id))
}

pub async fn handle_delete_comment(
    req: DeleteCommentRequest,
    ctx: HandlerContext,
) -> WsErrorResult<WebSocketMessage> {
    log::debug!("{} DeleteComment starting", ctx.log_prefix());

    let comment_id = parse_uuid(&req.comment_id, "comment_id")?;

    let repo = CommentRepository::new(ctx.pool.clone());
    let comment = db_read(&ctx, "find_comment", || async {
        repo.find_by_id(comment_id).await.map_err(WsError::from)
    })
    .await?
    .ok_or_else(|| WsError::NotFound {
        message: format!("Comment {} not found", comment_id),
        location: ErrorLocation::from(Location::caller()),
    })?;

    // Only author can delete their own comments
    if comment.created_by != ctx.user_id {
        return Err(WsError::Unauthorized {
            message: "Cannot delete another user's comment".to_string(),
            location: ErrorLocation::from(Location::caller()),
        });
    }

    // Soft delete
    db_write(&ctx, "delete_comment_tx", || async {
        let repo = CommentRepository::new(ctx.pool.clone());
        repo.delete(comment_id, Utc::now().timestamp()).await?;

        let activity = ActivityLog::deleted("comment", comment_id, ctx.user_id);
        ActivityLogRepository::create(&ctx.pool, &activity).await?;

        Ok::<_, WsError>(())
    })
    .await?;

    log::info!("{} Deleted comment {}", ctx.log_prefix(), comment_id);

    Ok(build_comment_deleted_response(&ctx.message_id, comment_id, ctx.user_id))
}

pub async fn handle_get_comments(
    req: GetCommentsRequest,
    ctx: HandlerContext,
) -> WsErrorResult<WebSocketMessage> {
    log::debug!("{} GetComments starting", ctx.log_prefix());

    let work_item_id = parse_uuid(&req.work_item_id, "work_item_id")?;

    // Verify work item exists and get project_id
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

    // Authorization
    db_read(&ctx, "check_permission", || async {
        check_permission(&ctx, work_item.project_id, Permission::View).await
    })
    .await?;

    let repo = CommentRepository::new(ctx.pool.clone());
    let comments = db_read(&ctx, "find_comments", || async {
        repo.find_by_work_item(work_item_id).await.map_err(WsError::from)
    })
    .await?;

    log::info!("{} Found {} comments for work item {}", ctx.log_prefix(), comments.len(), work_item_id);

    Ok(build_comments_list_response(&ctx.message_id, comments))
}
```

---

## Phase 6: Response Builder Updates (~100 lines)

### File: `backend/crates/pm-ws/src/handlers/response_builder.rs`

**Add imports at top:**
```rust
use pm_core::{Comment, Sprint, SprintStatus};
use pm_proto::{
    Comment as ProtoComment, CommentCreated, CommentDeleted, CommentUpdated, CommentsList,
    Sprint as ProtoSprint, SprintCreated, SprintDeleted, SprintUpdated, SprintsList,
    SprintStatus as ProtoSprintStatus,
    web_socket_message::Payload::{
        CommentCreated as ProtoCommentCreated, CommentDeleted as ProtoCommentDeleted,
        CommentUpdated as ProtoCommentUpdated, CommentsList as ProtoCommentsList,
        SprintCreated as ProtoSprintCreated, SprintDeleted as ProtoSprintDeleted,
        SprintUpdated as ProtoSprintUpdated, SprintsList as ProtoSprintsList,
    },
};
```

**Add sprint_to_proto helper:**
```rust
fn sprint_to_proto(sprint: &Sprint) -> ProtoSprint {
    ProtoSprint {
        id: sprint.id.to_string(),
        project_id: sprint.project_id.to_string(),
        name: sprint.name.clone(),
        goal: sprint.goal.clone(),
        start_date: sprint.start_date.timestamp(),
        end_date: sprint.end_date.timestamp(),
        status: match sprint.status {
            SprintStatus::Planned => ProtoSprintStatus::Planned.into(),
            SprintStatus::Active => ProtoSprintStatus::Active.into(),
            SprintStatus::Completed => ProtoSprintStatus::Completed.into(),
            SprintStatus::Cancelled => ProtoSprintStatus::Cancelled.into(),
        },
        version: sprint.version,
        created_at: sprint.created_at.timestamp(),
        updated_at: sprint.updated_at.timestamp(),
        created_by: sprint.created_by.to_string(),
        updated_by: sprint.updated_by.to_string(),
        deleted_at: sprint.deleted_at.map(|dt| dt.timestamp()),
    }
}

fn comment_to_proto(comment: &Comment) -> ProtoComment {
    ProtoComment {
        id: comment.id.to_string(),
        work_item_id: comment.work_item_id.to_string(),
        content: comment.content.clone(),
        created_at: comment.created_at.timestamp(),
        updated_at: comment.updated_at.timestamp(),
        created_by: comment.created_by.to_string(),
        updated_by: comment.updated_by.to_string(),
        deleted_at: comment.deleted_at.map(|dt| dt.timestamp()),
    }
}
```

**Add Sprint response builders:**
```rust
pub fn build_sprint_created_response(
    message_id: &str,
    sprint: &Sprint,
    actor_id: Uuid,
) -> WebSocketMessage {
    WebSocketMessage {
        message_id: message_id.to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(ProtoSprintCreated(SprintCreated {
            sprint: Some(sprint_to_proto(sprint)),
            user_id: actor_id.to_string(),
        })),
    }
}

pub fn build_sprint_updated_response(
    message_id: &str,
    sprint: &Sprint,
    changes: &[FieldChange],
    actor_id: Uuid,
) -> WebSocketMessage {
    WebSocketMessage {
        message_id: message_id.to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(ProtoSprintUpdated(SprintUpdated {
            sprint: Some(sprint_to_proto(sprint)),
            changes: changes.to_vec(),
            user_id: actor_id.to_string(),
        })),
    }
}

pub fn build_sprint_deleted_response(
    message_id: &str,
    sprint_id: Uuid,
    actor_id: Uuid,
) -> WebSocketMessage {
    WebSocketMessage {
        message_id: message_id.to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(ProtoSprintDeleted(SprintDeleted {
            sprint_id: sprint_id.to_string(),
            user_id: actor_id.to_string(),
        })),
    }
}

pub fn build_sprints_list_response(message_id: &str, sprints: Vec<Sprint>) -> WebSocketMessage {
    WebSocketMessage {
        message_id: message_id.to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(ProtoSprintsList(SprintsList {
            sprints: sprints.iter().map(sprint_to_proto).collect(),
        })),
    }
}
```

**Add Comment response builders:**
```rust
pub fn build_comment_created_response(
    message_id: &str,
    comment: &Comment,
    actor_id: Uuid,
) -> WebSocketMessage {
    WebSocketMessage {
        message_id: message_id.to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(ProtoCommentCreated(CommentCreated {
            comment: Some(comment_to_proto(comment)),
            user_id: actor_id.to_string(),
        })),
    }
}

pub fn build_comment_updated_response(
    message_id: &str,
    comment: &Comment,
    actor_id: Uuid,
) -> WebSocketMessage {
    WebSocketMessage {
        message_id: message_id.to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(ProtoCommentUpdated(CommentUpdated {
            comment: Some(comment_to_proto(comment)),
            user_id: actor_id.to_string(),
        })),
    }
}

pub fn build_comment_deleted_response(
    message_id: &str,
    comment_id: Uuid,
    actor_id: Uuid,
) -> WebSocketMessage {
    WebSocketMessage {
        message_id: message_id.to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(ProtoCommentDeleted(CommentDeleted {
            comment_id: comment_id.to_string(),
            user_id: actor_id.to_string(),
        })),
    }
}

pub fn build_comments_list_response(message_id: &str, comments: Vec<Comment>) -> WebSocketMessage {
    WebSocketMessage {
        message_id: message_id.to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(ProtoCommentsList(CommentsList {
            comments: comments.iter().map(comment_to_proto).collect(),
        })),
    }
}
```

---

## Phase 7: Dispatcher Updates (~40 lines)

### File: `backend/crates/pm-ws/src/handlers/dispatcher.rs`

**Add imports:**
```rust
use crate::handlers::sprint::{handle_create_sprint, handle_update_sprint, handle_delete_sprint, handle_get_sprints};
use crate::handlers::comment::{handle_create_comment, handle_update_comment, handle_delete_comment, handle_get_comments};
```

**Add to dispatch_inner() match (after line 77):**
```rust
        // Sprint handlers
        Some(Payload::CreateSprintRequest(req)) => handle_create_sprint(req, ctx).await,
        Some(Payload::UpdateSprintRequest(req)) => handle_update_sprint(req, ctx).await,
        Some(Payload::DeleteSprintRequest(req)) => handle_delete_sprint(req, ctx).await,
        Some(Payload::GetSprintsRequest(req)) => handle_get_sprints(req, ctx).await,

        // Comment handlers
        Some(Payload::CreateCommentRequest(req)) => handle_create_comment(req, ctx).await,
        Some(Payload::UpdateCommentRequest(req)) => handle_update_comment(req, ctx).await,
        Some(Payload::DeleteCommentRequest(req)) => handle_delete_comment(req, ctx).await,
        Some(Payload::GetCommentsRequest(req)) => handle_get_comments(req, ctx).await,
```

**Add to payload_to_handler_name() match (after line 137):**
```rust
        // Sprints
        Some(Payload::CreateSprintRequest(_)) => "CreateSprint",
        Some(Payload::UpdateSprintRequest(_)) => "UpdateSprint",
        Some(Payload::DeleteSprintRequest(_)) => "DeleteSprint",
        Some(Payload::GetSprintsRequest(_)) => "GetSprints",

        // Comments
        Some(Payload::CreateCommentRequest(_)) => "CreateComment",
        Some(Payload::UpdateCommentRequest(_)) => "UpdateComment",
        Some(Payload::DeleteCommentRequest(_)) => "DeleteComment",
        Some(Payload::GetCommentsRequest(_)) => "GetComments",
```

---

## Phase 8: Module Exports (~20 lines)

### File: `backend/crates/pm-ws/src/handlers/mod.rs`

**Add new modules:**
```rust
pub(crate) mod comment;
pub(crate) mod sprint;
```

### File: `backend/crates/pm-ws/src/lib.rs`

**Add to exports (after line 57):**
```rust
    sprint::{handle_create_sprint, handle_update_sprint, handle_delete_sprint, handle_get_sprints},
    comment::{handle_create_comment, handle_update_comment, handle_delete_comment, handle_get_comments},
    response_builder::{
        build_sprint_created_response, build_sprint_updated_response,
        build_sprint_deleted_response, build_sprints_list_response,
        build_comment_created_response, build_comment_updated_response,
        build_comment_deleted_response, build_comments_list_response,
    },
```

---

## Phase 9: Message Validator Updates (~30 lines)

### File: `backend/crates/pm-ws/src/message_validator.rs`

**Add Sprint validation:**
```rust
    pub fn validate_sprint_create(name: &str, goal: Option<&str>) -> Result<(), WsError> {
        Self::validate_string(name, "name", 1, 100)?;
        if let Some(g) = goal {
            Self::validate_string(g, "goal", 0, 500)?;
        }
        Ok(())
    }
```

**Add Comment validation:**
```rust
    pub fn validate_comment(content: &str) -> Result<(), WsError> {
        Self::validate_string(content, "content", 1, 5000)
    }
```

---

## Phase 10: Frontend Model Updates (~30 lines)

### File: `frontend/ProjectManagement.Core/Models/Sprint.cs`

**Add Version property (after Status):**
```csharp
    public int Version { get; init; } = 1;
```

### File: `frontend/ProjectManagement.Core/Models/UpdateSprintRequest.cs`

**Replace entire file:**
```csharp
namespace ProjectManagement.Core.Models;

public sealed record UpdateSprintRequest
{
    public required Guid SprintId { get; init; }
    public required int ExpectedVersion { get; init; }
    public string? Name { get; init; }
    public string? Goal { get; init; }
    public DateTime? StartDate { get; init; }
    public DateTime? EndDate { get; init; }
    public SprintStatus? Status { get; init; }
}
```

---

## Phase 11: Frontend Comment Models (~60 lines)

### File: `frontend/ProjectManagement.Core/Models/Comment.cs` (NEW)

```csharp
using ProjectManagement.Core.Interfaces;

namespace ProjectManagement.Core.Models;

public sealed record Comment : IAuditable
{
    public Guid Id { get; init; }
    public Guid WorkItemId { get; init; }
    public string Content { get; init; } = string.Empty;
    public DateTime CreatedAt { get; init; }
    public DateTime UpdatedAt { get; init; }
    public Guid CreatedBy { get; init; }
    public Guid UpdatedBy { get; init; }
    public DateTime? DeletedAt { get; init; }
}
```

### File: `frontend/ProjectManagement.Core/Models/CreateCommentRequest.cs` (NEW)

```csharp
namespace ProjectManagement.Core.Models;

public sealed record CreateCommentRequest
{
    public required Guid WorkItemId { get; init; }
    public required string Content { get; init; }
}
```

### File: `frontend/ProjectManagement.Core/Models/UpdateCommentRequest.cs` (NEW)

```csharp
namespace ProjectManagement.Core.Models;

public sealed record UpdateCommentRequest
{
    public required Guid CommentId { get; init; }
    public required string Content { get; init; }
}
```

---

## Verification Commands

After each phase:

```bash
# Phase 1: Proto changes
just build-backend && just build-frontend

# Phase 2-9: Backend
just check-rs-ws && just test-rs-ws

# Phase 10-15: Frontend
just build-frontend && just test-frontend

# Final
just check
```

---

## Summary

| Phase | Files | Lines | Cumulative |
|-------|-------|-------|------------|
| 1. Proto | 1 | 50 | 50 |
| 2. Sprint Model | 1 | 20 | 70 |
| 3. Sprint Repo | 1 | 40 | 110 |
| 4. Sprint Handler | 1 | 200 | 310 |
| 5. Comment Handler | 1 | 180 | 490 |
| 6. Response Builder | 1 | 100 | 590 |
| 7. Dispatcher | 1 | 40 | 630 |
| 8. Module Exports | 2 | 20 | 650 |
| 9. Msg Validator | 1 | 30 | 680 |
| 10. Frontend Models | 2 | 30 | 710 |
| 11. Comment Models | 3 | 60 | 770 |

---

## Phase 12: Frontend ProtoConverter Updates (~100 lines)

### File: `frontend/ProjectManagement.Core/Converters/ProtoConverter.cs`

**Add after Project region (line 107):**
```csharp
    #region Sprint Conversions

    public static Sprint ToDomain(Proto.Sprint proto)
    {
        ArgumentNullException.ThrowIfNull(proto);

        return new Sprint
        {
            Id = ParseGuid(proto.Id, "Sprint.Id"),
            ProjectId = ParseGuid(proto.ProjectId, "Sprint.ProjectId"),
            Name = proto.Name ?? string.Empty,
            Goal = string.IsNullOrEmpty(proto.Goal) ? null : proto.Goal,
            StartDate = FromUnixTimestamp(proto.StartDate),
            EndDate = FromUnixTimestamp(proto.EndDate),
            Status = ToDomain(proto.Status),
            Version = proto.Version,
            CreatedAt = FromUnixTimestamp(proto.CreatedAt),
            UpdatedAt = FromUnixTimestamp(proto.UpdatedAt),
            CreatedBy = ParseGuid(proto.CreatedBy, "Sprint.CreatedBy"),
            UpdatedBy = ParseGuid(proto.UpdatedBy, "Sprint.UpdatedBy"),
            DeletedAt = proto.DeletedAt == 0 ? null : FromUnixTimestamp(proto.DeletedAt),
        };
    }

    public static Proto.CreateSprintRequest ToProto(CreateSprintRequest req)
    {
        ArgumentNullException.ThrowIfNull(req);

        return new Proto.CreateSprintRequest
        {
            ProjectId = req.ProjectId.ToString(),
            Name = req.Name,
            Goal = req.Goal ?? string.Empty,
            StartDate = ToUnixTimestamp(req.StartDate),
            EndDate = ToUnixTimestamp(req.EndDate),
        };
    }

    public static Proto.UpdateSprintRequest ToProto(UpdateSprintRequest req)
    {
        ArgumentNullException.ThrowIfNull(req);

        var proto = new Proto.UpdateSprintRequest
        {
            SprintId = req.SprintId.ToString(),
            ExpectedVersion = req.ExpectedVersion,
        };

        if (req.Name is not null) proto.Name = req.Name;
        if (req.Goal is not null) proto.Goal = req.Goal;
        if (req.StartDate.HasValue) proto.StartDate = ToUnixTimestamp(req.StartDate.Value);
        if (req.EndDate.HasValue) proto.EndDate = ToUnixTimestamp(req.EndDate.Value);
        if (req.Status.HasValue) proto.Status = ToProto(req.Status.Value);

        return proto;
    }

    public static SprintStatus ToDomain(Proto.SprintStatus proto)
    {
        return proto switch
        {
            Proto.SprintStatus.Planned => SprintStatus.Planned,
            Proto.SprintStatus.Active => SprintStatus.Active,
            Proto.SprintStatus.Completed => SprintStatus.Completed,
            Proto.SprintStatus.Cancelled => SprintStatus.Cancelled,
            _ => SprintStatus.Planned
        };
    }

    public static Proto.SprintStatus ToProto(SprintStatus domain)
    {
        return domain switch
        {
            SprintStatus.Planned => Proto.SprintStatus.Planned,
            SprintStatus.Active => Proto.SprintStatus.Active,
            SprintStatus.Completed => Proto.SprintStatus.Completed,
            SprintStatus.Cancelled => Proto.SprintStatus.Cancelled,
            _ => Proto.SprintStatus.Planned
        };
    }

    #endregion

    #region Comment Conversions

    public static Comment ToDomain(Proto.Comment proto)
    {
        ArgumentNullException.ThrowIfNull(proto);

        return new Comment
        {
            Id = ParseGuid(proto.Id, "Comment.Id"),
            WorkItemId = ParseGuid(proto.WorkItemId, "Comment.WorkItemId"),
            Content = proto.Content ?? string.Empty,
            CreatedAt = FromUnixTimestamp(proto.CreatedAt),
            UpdatedAt = FromUnixTimestamp(proto.UpdatedAt),
            CreatedBy = ParseGuid(proto.CreatedBy, "Comment.CreatedBy"),
            UpdatedBy = ParseGuid(proto.UpdatedBy, "Comment.UpdatedBy"),
            DeletedAt = proto.DeletedAt == 0 ? null : FromUnixTimestamp(proto.DeletedAt),
        };
    }

    public static Proto.CreateCommentRequest ToProto(CreateCommentRequest req)
    {
        ArgumentNullException.ThrowIfNull(req);

        return new Proto.CreateCommentRequest
        {
            WorkItemId = req.WorkItemId.ToString(),
            Content = req.Content,
        };
    }

    public static Proto.UpdateCommentRequest ToProto(UpdateCommentRequest req)
    {
        ArgumentNullException.ThrowIfNull(req);

        return new Proto.UpdateCommentRequest
        {
            CommentId = req.CommentId.ToString(),
            Content = req.Content,
        };
    }

    #endregion
```

---

## Phase 13: Frontend IWebSocketClient Updates (~50 lines)

### File: `frontend/ProjectManagement.Core/Interfaces/IWebSocketClient.cs`

**Add after Project events (line 59):**
```csharp
    // ============================================================
    // Sprint Events
    // ============================================================

    event Action<Sprint>? OnSprintCreated;
    event Action<Sprint, IReadOnlyList<FieldChange>>? OnSprintUpdated;
    event Action<Guid>? OnSprintDeleted;

    // ============================================================
    // Sprint Operations
    // ============================================================

    Task<Sprint> CreateSprintAsync(CreateSprintRequest request, CancellationToken ct = default);
    Task<Sprint> UpdateSprintAsync(UpdateSprintRequest request, CancellationToken ct = default);
    Task DeleteSprintAsync(Guid sprintId, CancellationToken ct = default);
    Task<IReadOnlyList<Sprint>> GetSprintsAsync(Guid projectId, CancellationToken ct = default);

    // ============================================================
    // Comment Events
    // ============================================================

    event Action<Comment>? OnCommentCreated;
    event Action<Comment>? OnCommentUpdated;
    event Action<Guid>? OnCommentDeleted;

    // ============================================================
    // Comment Operations
    // ============================================================

    Task<Comment> CreateCommentAsync(CreateCommentRequest request, CancellationToken ct = default);
    Task<Comment> UpdateCommentAsync(UpdateCommentRequest request, CancellationToken ct = default);
    Task DeleteCommentAsync(Guid commentId, CancellationToken ct = default);
    Task<IReadOnlyList<Comment>> GetCommentsAsync(Guid workItemId, CancellationToken ct = default);
```

---

## Phase 14: Frontend WebSocketClient Implementation (~200 lines)

### File: `frontend/ProjectManagement.Services/WebSocket/WebSocketClient.cs`

**Add events after line 81:**
```csharp
    public event Action<Sprint>? OnSprintCreated;
    public event Action<Sprint, IReadOnlyList<FieldChange>>? OnSprintUpdated;
    public event Action<Guid>? OnSprintDeleted;
    public event Action<Comment>? OnCommentCreated;
    public event Action<Comment>? OnCommentUpdated;
    public event Action<Guid>? OnCommentDeleted;
```

**Add Sprint operations after GetProjectsAsync (~line 518):**
```csharp
    public async Task<Sprint> CreateSprintAsync(CreateSprintRequest request, CancellationToken ct = default)
    {
        ThrowIfDisposed();
        EnsureConnected();

        var message = new Pm.WebSocketMessage
        {
            MessageId = Guid.NewGuid().ToString(),
            Timestamp = DateTimeOffset.UtcNow.ToUnixTimeSeconds(),
            CreateSprintRequest = ProtoConverter.ToProto(request),
        };

        var response = await SendRequestAsync(message, ct);

        if (response.PayloadCase == Pm.WebSocketMessage.PayloadOneofCase.Error)
            throw new ServerRejectedException(response.Error.Code, response.Error.Message);

        if (response.PayloadCase != Pm.WebSocketMessage.PayloadOneofCase.SprintCreated)
            throw new InvalidOperationException($"Unexpected response type: {response.PayloadCase}");

        return ProtoConverter.ToDomain(response.SprintCreated.Sprint);
    }

    public async Task<Sprint> UpdateSprintAsync(UpdateSprintRequest request, CancellationToken ct = default)
    {
        ThrowIfDisposed();
        EnsureConnected();

        var message = new Pm.WebSocketMessage
        {
            MessageId = Guid.NewGuid().ToString(),
            Timestamp = DateTimeOffset.UtcNow.ToUnixTimeSeconds(),
            UpdateSprintRequest = ProtoConverter.ToProto(request),
        };

        var response = await SendRequestAsync(message, ct);

        if (response.PayloadCase == Pm.WebSocketMessage.PayloadOneofCase.Error)
            throw new ServerRejectedException(response.Error.Code, response.Error.Message);

        if (response.PayloadCase != Pm.WebSocketMessage.PayloadOneofCase.SprintUpdated)
            throw new InvalidOperationException($"Unexpected response type: {response.PayloadCase}");

        return ProtoConverter.ToDomain(response.SprintUpdated.Sprint);
    }

    public async Task DeleteSprintAsync(Guid sprintId, CancellationToken ct = default)
    {
        ThrowIfDisposed();
        EnsureConnected();

        var message = new Pm.WebSocketMessage
        {
            MessageId = Guid.NewGuid().ToString(),
            Timestamp = DateTimeOffset.UtcNow.ToUnixTimeSeconds(),
            DeleteSprintRequest = new Pm.DeleteSprintRequest { SprintId = sprintId.ToString() },
        };

        var response = await SendRequestAsync(message, ct);

        if (response.PayloadCase == Pm.WebSocketMessage.PayloadOneofCase.Error)
            throw new ServerRejectedException(response.Error.Code, response.Error.Message);

        if (response.PayloadCase != Pm.WebSocketMessage.PayloadOneofCase.SprintDeleted)
            throw new InvalidOperationException($"Unexpected response type: {response.PayloadCase}");
    }

    public async Task<IReadOnlyList<Sprint>> GetSprintsAsync(Guid projectId, CancellationToken ct = default)
    {
        ThrowIfDisposed();
        EnsureConnected();

        var message = new Pm.WebSocketMessage
        {
            MessageId = Guid.NewGuid().ToString(),
            Timestamp = DateTimeOffset.UtcNow.ToUnixTimeSeconds(),
            GetSprintsRequest = new Pm.GetSprintsRequest { ProjectId = projectId.ToString() },
        };

        var response = await SendRequestAsync(message, ct);

        if (response.PayloadCase == Pm.WebSocketMessage.PayloadOneofCase.Error)
            throw new ServerRejectedException(response.Error.Code, response.Error.Message);

        if (response.PayloadCase != Pm.WebSocketMessage.PayloadOneofCase.SprintsList)
            throw new InvalidOperationException($"Unexpected response type: {response.PayloadCase}");

        return response.SprintsList.Sprints.Select(ProtoConverter.ToDomain).ToList();
    }

    public async Task<Comment> CreateCommentAsync(CreateCommentRequest request, CancellationToken ct = default)
    {
        ThrowIfDisposed();
        EnsureConnected();

        var message = new Pm.WebSocketMessage
        {
            MessageId = Guid.NewGuid().ToString(),
            Timestamp = DateTimeOffset.UtcNow.ToUnixTimeSeconds(),
            CreateCommentRequest = ProtoConverter.ToProto(request),
        };

        var response = await SendRequestAsync(message, ct);

        if (response.PayloadCase == Pm.WebSocketMessage.PayloadOneofCase.Error)
            throw new ServerRejectedException(response.Error.Code, response.Error.Message);

        if (response.PayloadCase != Pm.WebSocketMessage.PayloadOneofCase.CommentCreated)
            throw new InvalidOperationException($"Unexpected response type: {response.PayloadCase}");

        return ProtoConverter.ToDomain(response.CommentCreated.Comment);
    }

    public async Task<Comment> UpdateCommentAsync(UpdateCommentRequest request, CancellationToken ct = default)
    {
        ThrowIfDisposed();
        EnsureConnected();

        var message = new Pm.WebSocketMessage
        {
            MessageId = Guid.NewGuid().ToString(),
            Timestamp = DateTimeOffset.UtcNow.ToUnixTimeSeconds(),
            UpdateCommentRequest = ProtoConverter.ToProto(request),
        };

        var response = await SendRequestAsync(message, ct);

        if (response.PayloadCase == Pm.WebSocketMessage.PayloadOneofCase.Error)
            throw new ServerRejectedException(response.Error.Code, response.Error.Message);

        if (response.PayloadCase != Pm.WebSocketMessage.PayloadOneofCase.CommentUpdated)
            throw new InvalidOperationException($"Unexpected response type: {response.PayloadCase}");

        return ProtoConverter.ToDomain(response.CommentUpdated.Comment);
    }

    public async Task DeleteCommentAsync(Guid commentId, CancellationToken ct = default)
    {
        ThrowIfDisposed();
        EnsureConnected();

        var message = new Pm.WebSocketMessage
        {
            MessageId = Guid.NewGuid().ToString(),
            Timestamp = DateTimeOffset.UtcNow.ToUnixTimeSeconds(),
            DeleteCommentRequest = new Pm.DeleteCommentRequest { CommentId = commentId.ToString() },
        };

        var response = await SendRequestAsync(message, ct);

        if (response.PayloadCase == Pm.WebSocketMessage.PayloadOneofCase.Error)
            throw new ServerRejectedException(response.Error.Code, response.Error.Message);

        if (response.PayloadCase != Pm.WebSocketMessage.PayloadOneofCase.CommentDeleted)
            throw new InvalidOperationException($"Unexpected response type: {response.PayloadCase}");
    }

    public async Task<IReadOnlyList<Comment>> GetCommentsAsync(Guid workItemId, CancellationToken ct = default)
    {
        ThrowIfDisposed();
        EnsureConnected();

        var message = new Pm.WebSocketMessage
        {
            MessageId = Guid.NewGuid().ToString(),
            Timestamp = DateTimeOffset.UtcNow.ToUnixTimeSeconds(),
            GetCommentsRequest = new Pm.GetCommentsRequest { WorkItemId = workItemId.ToString() },
        };

        var response = await SendRequestAsync(message, ct);

        if (response.PayloadCase == Pm.WebSocketMessage.PayloadOneofCase.Error)
            throw new ServerRejectedException(response.Error.Code, response.Error.Message);

        if (response.PayloadCase != Pm.WebSocketMessage.PayloadOneofCase.CommentsList)
            throw new InvalidOperationException($"Unexpected response type: {response.PayloadCase}");

        return response.CommentsList.Comments.Select(ProtoConverter.ToDomain).ToList();
    }
```

**Add to HandleBroadcastEvent switch (after ProjectDeleted case, ~line 680):**
```csharp
            case Pm.WebSocketMessage.PayloadOneofCase.SprintCreated:
                var createdSprint = ProtoConverter.ToDomain(message.SprintCreated.Sprint);
                OnSprintCreated?.Invoke(createdSprint);
                break;

            case Pm.WebSocketMessage.PayloadOneofCase.SprintUpdated:
                var updatedSprint = ProtoConverter.ToDomain(message.SprintUpdated.Sprint);
                var sprintChanges = message.SprintUpdated.Changes
                    .Select(c => new FieldChange(c.FieldName, c.OldValue, c.NewValue))
                    .ToList();
                OnSprintUpdated?.Invoke(updatedSprint, sprintChanges);
                break;

            case Pm.WebSocketMessage.PayloadOneofCase.SprintDeleted:
                if (Guid.TryParse(message.SprintDeleted.SprintId, out var deletedSprintId))
                    OnSprintDeleted?.Invoke(deletedSprintId);
                break;

            case Pm.WebSocketMessage.PayloadOneofCase.CommentCreated:
                var createdComment = ProtoConverter.ToDomain(message.CommentCreated.Comment);
                OnCommentCreated?.Invoke(createdComment);
                break;

            case Pm.WebSocketMessage.PayloadOneofCase.CommentUpdated:
                var updatedComment = ProtoConverter.ToDomain(message.CommentUpdated.Comment);
                OnCommentUpdated?.Invoke(updatedComment);
                break;

            case Pm.WebSocketMessage.PayloadOneofCase.CommentDeleted:
                if (Guid.TryParse(message.CommentDeleted.CommentId, out var deletedCommentId))
                    OnCommentDeleted?.Invoke(deletedCommentId);
                break;
```

---

## Phase 15: Frontend SprintStore WebSocket Integration (~80 lines)

### File: `frontend/ProjectManagement.Services/State/SprintStore.cs`

**Update constructor to wire up events (replace lines 29-32):**
```csharp
        _client.OnSprintCreated += HandleSprintCreated;
        _client.OnSprintUpdated += HandleSprintUpdated;
        _client.OnSprintDeleted += HandleSprintDeleted;
```

**Update Dispose (replace lines 36-42):**
```csharp
    public void Dispose()
    {
        if (_disposed) return;
        _disposed = true;

        _client.OnSprintCreated -= HandleSprintCreated;
        _client.OnSprintUpdated -= HandleSprintUpdated;
        _client.OnSprintDeleted -= HandleSprintDeleted;
    }
```

**Replace CreateAsync (lines 71-106):**
```csharp
    public async Task<Sprint> CreateAsync(
        CreateSprintRequest request,
        CancellationToken ct = default)
    {
        ThrowIfDisposed();

        var tempId = Guid.NewGuid();
        var optimistic = new Sprint
        {
            Id = tempId,
            ProjectId = request.ProjectId,
            Name = request.Name,
            Goal = request.Goal,
            StartDate = request.StartDate,
            EndDate = request.EndDate,
            Status = SprintStatus.Planned,
            Version = 1,
            CreatedAt = DateTime.UtcNow,
            UpdatedAt = DateTime.UtcNow,
            CreatedBy = Guid.Empty,
            UpdatedBy = Guid.Empty
        };

        _sprints[tempId] = optimistic;
        _pendingUpdates[tempId] = true;
        NotifyChanged();

        try
        {
            var confirmed = await _client.CreateSprintAsync(request, ct);
            _sprints.TryRemove(tempId, out _);
            _sprints[confirmed.Id] = confirmed;
            _pendingUpdates.TryRemove(tempId, out _);
            NotifyChanged();
            _logger.LogDebug("Sprint created: {Id}", confirmed.Id);
            return confirmed;
        }
        catch
        {
            _sprints.TryRemove(tempId, out _);
            _pendingUpdates.TryRemove(tempId, out _);
            NotifyChanged();
            throw;
        }
    }
```

**Replace UpdateAsync (lines 108-138):**
```csharp
    public async Task<Sprint> UpdateAsync(
        UpdateSprintRequest request,
        CancellationToken ct = default)
    {
        ThrowIfDisposed();

        if (!_sprints.TryGetValue(request.SprintId, out var current))
            throw new KeyNotFoundException($"Sprint not found: {request.SprintId}");

        var optimistic = current with
        {
            Name = request.Name ?? current.Name,
            Goal = request.Goal ?? current.Goal,
            StartDate = request.StartDate ?? current.StartDate,
            EndDate = request.EndDate ?? current.EndDate,
            Status = request.Status ?? current.Status,
            Version = current.Version + 1,
            UpdatedAt = DateTime.UtcNow
        };

        var previousValue = _sprints[request.SprintId];
        _sprints[request.SprintId] = optimistic;
        _pendingUpdates[request.SprintId] = true;
        NotifyChanged();

        try
        {
            var confirmed = await _client.UpdateSprintAsync(request, ct);
            _sprints[request.SprintId] = confirmed;
            _pendingUpdates.TryRemove(request.SprintId, out _);
            NotifyChanged();
            return confirmed;
        }
        catch
        {
            _sprints[request.SprintId] = previousValue;
            _pendingUpdates.TryRemove(request.SprintId, out _);
            NotifyChanged();
            throw;
        }
    }
```

**Replace RefreshAsync (lines 228-235):**
```csharp
    public async Task RefreshAsync(Guid projectId, CancellationToken ct = default)
    {
        ThrowIfDisposed();

        var sprints = await _client.GetSprintsAsync(projectId, ct);

        var toRemove = _sprints.Values
            .Where(s => s.ProjectId == projectId)
            .Select(s => s.Id)
            .ToList();

        foreach (var id in toRemove) _sprints.TryRemove(id, out _);
        foreach (var sprint in sprints) _sprints[sprint.Id] = sprint;

        NotifyChanged();
        _logger.LogDebug("Refreshed {Count} sprints for project {ProjectId}", sprints.Count, projectId);
    }
```

**Add event handlers at end of class (before final closing brace):**
```csharp
    #region Event Handlers

    private void HandleSprintCreated(Sprint sprint)
    {
        if (_pendingUpdates.ContainsKey(sprint.Id)) return;
        _sprints[sprint.Id] = sprint;
        NotifyChanged();
        _logger.LogDebug("Received sprint created: {Id}", sprint.Id);
    }

    private void HandleSprintUpdated(Sprint sprint, IReadOnlyList<FieldChange> changes)
    {
        if (_pendingUpdates.ContainsKey(sprint.Id)) return;
        _sprints[sprint.Id] = sprint;
        NotifyChanged();
        _logger.LogDebug("Received sprint updated: {Id}", sprint.Id);
    }

    private void HandleSprintDeleted(Guid id)
    {
        if (_pendingUpdates.ContainsKey(id)) return;
        if (_sprints.TryGetValue(id, out var sprint))
        {
            _sprints[id] = sprint with { DeletedAt = DateTime.UtcNow };
            NotifyChanged();
        }
        _logger.LogDebug("Received sprint deleted: {Id}", id);
    }

    #endregion
```

---

## Complete Summary

| Phase | Files | Lines | Cumulative |
|-------|-------|-------|------------|
| 1. Proto | 1 | 50 | 50 |
| 2. Sprint Model | 1 | 20 | 70 |
| 3. Sprint Repo | 1 | 40 | 110 |
| 4. Sprint Handler | 1 | 200 | 310 |
| 5. Comment Handler | 1 | 180 | 490 |
| 6. Response Builder | 1 | 100 | 590 |
| 7. Dispatcher | 1 | 40 | 630 |
| 8. Module Exports | 2 | 20 | 650 |
| 9. Msg Validator | 1 | 30 | 680 |
| 10. Frontend Models | 2 | 30 | 710 |
| 11. Comment Models | 3 | 60 | 770 |
| 12. ProtoConverter | 1 | 100 | 870 |
| 13. IWebSocketClient | 1 | 50 | 920 |
| 14. WebSocketClient | 1 | 200 | 1120 |
| 15. SprintStore | 1 | 80 | 1200 |

**Total: ~1,200 lines of actual code**

---

## What's NOT in This Session (Session 50.1)

- CommentStore (~100 lines)
- ICommentStore interface (~30 lines)
- CommentViewModel (~50 lines)
- Sprint UI components (~300 lines)
- Comment UI components (~200 lines)
- Backend tests (~150 lines)
- Frontend tests (~150 lines)

**Session 50.1 total: ~980 additional lines**
