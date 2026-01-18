# Session 10 Implementation Plan: Wire Database & Message Dispatch

## What Already Exists (DO NOT RECREATE)

### pm-ws has:
- `HandlerContext` (context.rs) - message_id, user_id, pool
- `WsError` (error.rs) - **14 variants** with location tracking
- `response_builder.rs` - build_work_item_created_response(), build_error_response(), etc.
- `idempotency.rs` - check_idempotency(), store_idempotency()
- `authorization.rs` - check_permission(ctx, project_id, permission)
- `change_tracker.rs` - track_changes()
- `hierarchy_validator.rs` - validate_hierarchy()
- `MessageValidator` (message_validator.rs) - validation functions
- `ClientSubscriptions` - subscription tracking
- `Metrics` - performance metrics
- `ConnectionRegistry` - connection management
- `ShutdownCoordinator` - graceful shutdown

### pm-db has:
- 9 repositories with full CRUD
- `IdempotencyRepository` - find_by_message_id(), create(), cleanup_old_entries()
- `WorkItemRepository` and `ActivityLogRepository` - support transactions via executor pattern
- 13 complete migrations - schema is done

### pm-server has:
- `ServerError` enum in error.rs - extend this, don't create new

---

## What Actually Needs to Be Built

### 1. Add Database Error to WsError

**File**: `backend/crates/pm-ws/src/error.rs`

Add new variant and From impl:

```rust
// Add to existing WsError enum (after Unauthorized variant)
#[error("Database error: {0}")]
Database(#[from] pm_db::DbError),
```

Update error_code():
```rust
Self::Database { .. } => "DATABASE_ERROR",
```

### 2. Add Database Variants to ServerError

**File**: `backend/pm-server/src/error.rs`

Add new variants to existing enum:

```rust
// Add to existing ServerError enum
#[error("Failed to create database directory '{path}': {source}")]
CreateDirectory {
    path: std::path::PathBuf,
    #[source]
    source: std::io::Error,
},

#[error("Database pool error: {0}")]
DatabasePool(#[from] sqlx::Error),

#[error("Database migration failed: {0}")]
Migration(#[from] sqlx::migrate::MigrateError),
```

### 3. Add Pool to AppState

**File**: `backend/crates/pm-ws/src/app_state.rs`

Add SqlitePool field:

```rust
pub struct AppState {
    pub pool: SqlitePool,                              // ADD THIS
    pub jwt_validator: Option<Arc<JwtValidator>>,
    pub desktop_user_id: String,
    pub rate_limiter_factory: RateLimiterFactory,
    pub registry: ConnectionRegistry,
    pub metrics: Metrics,
    pub shutdown: ShutdownCoordinator,
    pub config: ConnectionConfig,
}
```

### 4. Database Initialization in main.rs

**File**: `backend/pm-server/src/main.rs`

```rust
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};

// After config loading
let db_path = config.database_path()?;
if let Some(parent) = db_path.parent() {
    std::fs::create_dir_all(parent).map_err(|e| ServerError::CreateDirectory {
        path: parent.to_path_buf(),
        source: e,
    })?;
}

let connect_options = SqliteConnectOptions::new()
    .filename(&db_path)
    .create_if_missing(true)
    .foreign_keys(true)
    .journal_mode(SqliteJournalMode::Wal)
    .busy_timeout(Duration::from_secs(30))
    .synchronous(SqliteSynchronous::Normal);

let pool = SqlitePoolOptions::new()
    .max_connections(5)
    .min_connections(1)
    .acquire_timeout(Duration::from_secs(5))
    .connect_with(connect_options)
    .await?;

sqlx::migrate!("../crates/pm-db/migrations")
    .run(&pool)
    .await?;

info!("Database initialized at {:?}", db_path);

// Pass pool to AppState
```

### 5. Wire handle_binary_message() to Dispatcher

**File**: `backend/crates/pm-ws/src/web_socket_connection.rs`

Replace the TODO in handle_binary_message():

```rust
async fn handle_binary_message(
    &mut self,
    data: bytes::Bytes,
    tx: &mpsc::Sender<Message>,
    pool: &SqlitePool,
    user_id: Uuid,
) -> WsErrorResult<()> {
    use prost::Message as _;
    use pm_proto::{WebSocketMessage, web_socket_message::Payload};

    // Decode protobuf
    let msg = WebSocketMessage::decode(&*data)?;
    let message_type = payload_type_name(&msg.payload);
    self.metrics.message_received(&message_type);

    // Create context
    let ctx = HandlerContext::new(
        msg.message_id.clone(),
        user_id,
        pool.clone(),
    );

    // Dispatch to handlers
    let response = dispatch(msg, ctx).await;

    // Encode and send response
    let encoded = response.encode_to_vec();
    tx.send(Message::Binary(encoded.into())).await
        .map_err(|_| WsError::SendBufferFull { location: ErrorLocation::caller() })?;

    Ok(())
}

fn payload_type_name(payload: &Option<web_socket_message::Payload>) -> String {
    match payload {
        Some(Payload::CreateWorkItemRequest(_)) => "CreateWorkItem".to_string(),
        Some(Payload::UpdateWorkItemRequest(_)) => "UpdateWorkItem".to_string(),
        Some(Payload::DeleteWorkItemRequest(_)) => "DeleteWorkItem".to_string(),
        Some(Payload::GetWorkItemsRequest(_)) => "GetWorkItems".to_string(),
        Some(Payload::Ping(_)) => "Ping".to_string(),
        _ => "Unknown".to_string(),
    }
}
```

### 6. Create Dispatcher Module

**Create**: `backend/crates/pm-ws/src/handlers/dispatcher.rs`

```rust
use crate::handlers::{context::HandlerContext, work_item, query, response_builder::build_error_response};
use crate::WsError;
use pm_proto::{WebSocketMessage, web_socket_message::Payload};
use error_location::ErrorLocation;
use std::panic::Location;

pub async fn dispatch(msg: WebSocketMessage, ctx: HandlerContext) -> WebSocketMessage {
    let message_id = msg.message_id.clone();

    let result = match msg.payload {
        Some(Payload::CreateWorkItemRequest(req)) => {
            work_item::handle_create(req, ctx).await
        }
        Some(Payload::UpdateWorkItemRequest(req)) => {
            work_item::handle_update(req, ctx).await
        }
        Some(Payload::DeleteWorkItemRequest(req)) => {
            work_item::handle_delete(req, ctx).await
        }
        Some(Payload::GetWorkItemsRequest(req)) => {
            query::handle_get_work_items(req, ctx).await
        }
        Some(Payload::Ping(ping)) => {
            // Handle ping directly
            return WebSocketMessage {
                message_id,
                timestamp: chrono::Utc::now().timestamp(),
                payload: Some(Payload::Pong(pm_proto::Pong { timestamp: ping.timestamp })),
            };
        }
        _ => Err(WsError::InvalidMessage {
            message: "Unsupported message type".into(),
            location: ErrorLocation::from(Location::caller()),
        }),
    };

    match result {
        Ok(response) => response,
        Err(e) => build_error_response(&message_id, e.to_proto_error()),
    }
}
```

### 7. Create Work Item Handler

**Create**: `backend/crates/pm-ws/src/handlers/work_item.rs`

```rust
use crate::handlers::{
    context::HandlerContext,
    idempotency::{check_idempotency, store_idempotency},
    authorization::check_permission,
    hierarchy_validator::validate_hierarchy,
    change_tracker::track_changes,
    response_builder::*,
};
use crate::{MessageValidator, Result as WsErrorResult, WsError};
use pm_core::{Permission, WorkItem, WorkItemType};
use pm_db::{WorkItemRepository, ActivityLogRepository};
use pm_proto::{CreateWorkItemRequest, UpdateWorkItemRequest, DeleteWorkItemRequest, WebSocketMessage};
use chrono::Utc;
use error_location::ErrorLocation;
use std::panic::Location;
use uuid::Uuid;

pub async fn handle_create(
    req: CreateWorkItemRequest,
    ctx: HandlerContext,
) -> WsErrorResult<WebSocketMessage> {
    // 1. Validate input
    MessageValidator::validate_work_item_create(&req)?;

    // 2. Check idempotency (before transaction)
    if let Some(cached) = check_idempotency(&ctx.pool, &ctx.message_id).await? {
        // Deserialize cached response
        let response: WebSocketMessage = serde_json::from_str(&cached)
            .map_err(|e| WsError::Internal {
                message: format!("Failed to deserialize cached response: {e}"),
                location: ErrorLocation::from(Location::caller()),
            })?;
        return Ok(response);
    }

    // 3. Parse and validate IDs
    let project_id = Uuid::parse_str(&req.project_id)
        .map_err(|_| WsError::ValidationError {
            message: "Invalid project_id format".to_string(),
            field: Some("project_id".to_string()),
            location: ErrorLocation::from(Location::caller()),
        })?;

    let parent_id = match &req.parent_id {
        Some(id) => Some(Uuid::parse_str(id).map_err(|_| WsError::ValidationError {
            message: "Invalid parent_id format".to_string(),
            field: Some("parent_id".to_string()),
            location: ErrorLocation::from(Location::caller()),
        })?),
        None => None,
    };

    // 4. Authorization
    check_permission(&ctx, project_id, Permission::Write).await?;

    // 5. Validate hierarchy if parent specified
    let item_type = WorkItemType::from(req.item_type);
    if let Some(pid) = parent_id {
        validate_hierarchy(&ctx.pool, item_type.clone(), pid).await?;
    } else if item_type != WorkItemType::Project {
        return Err(WsError::ValidationError {
            message: format!("{:?} must have a parent", item_type),
            field: Some("parent_id".to_string()),
            location: ErrorLocation::from(Location::caller()),
        });
    }

    // 6. Get next position
    let max_position = WorkItemRepository::find_max_position(&ctx.pool, project_id, parent_id).await?;

    // 7. Build work item
    let now = Utc::now();
    let work_item = WorkItem {
        id: Uuid::new_v4(),
        item_type,
        parent_id,
        project_id,
        position: max_position + 1,
        title: req.title,
        description: req.description,
        status: "todo".to_string(),
        priority: "medium".to_string(),
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

    // 8. Transaction: insert + activity log + idempotency
    let mut tx = ctx.pool.begin().await?;

    WorkItemRepository::create(&mut *tx, &work_item).await?;

    let activity = pm_core::ActivityLog {
        id: Uuid::new_v4(),
        entity_type: "work_item".to_string(),
        entity_id: work_item.id,
        action: "created".to_string(),
        changes_json: None,
        user_id: ctx.user_id,
        created_at: now,
    };
    ActivityLogRepository::create(&mut *tx, &activity).await?;

    // Build response
    let response = build_work_item_created_response(&ctx.message_id, &work_item, ctx.user_id);

    // Store idempotency inside transaction
    let response_json = serde_json::to_string(&response)
        .map_err(|e| WsError::Internal {
            message: format!("Failed to serialize response: {e}"),
            location: ErrorLocation::from(Location::caller()),
        })?;
    store_idempotency(&ctx.pool, &ctx.message_id, "create_work_item", &response_json).await?;

    tx.commit().await?;

    Ok(response)
}

pub async fn handle_update(
    req: UpdateWorkItemRequest,
    ctx: HandlerContext,
) -> WsErrorResult<WebSocketMessage> {
    // 1. Parse work item ID
    let work_item_id = Uuid::parse_str(&req.work_item_id)
        .map_err(|_| WsError::ValidationError {
            message: "Invalid work_item_id format".to_string(),
            field: Some("work_item_id".to_string()),
            location: ErrorLocation::from(Location::caller()),
        })?;

    // 2. Fetch existing work item
    let mut work_item = WorkItemRepository::find_by_id(&ctx.pool, work_item_id)
        .await?
        .ok_or_else(|| WsError::NotFound {
            message: format!("Work item {} not found", work_item_id),
            location: ErrorLocation::from(Location::caller()),
        })?;

    // 3. Authorization
    check_permission(&ctx, work_item.project_id, Permission::Write).await?;

    // 4. Optimistic locking - check version
    if work_item.version != req.expected_version {
        return Err(WsError::ConflictError {
            current_version: work_item.version,
            location: ErrorLocation::from(Location::caller()),
        });
    }

    // 5. Track changes before applying updates
    let changes = track_changes(&work_item, &req);

    // 6. Apply updates
    if let Some(ref title) = req.title {
        work_item.title = title.clone();
    }
    if let Some(ref desc) = req.description {
        work_item.description = Some(desc.clone());
    }
    if let Some(ref status) = req.status {
        work_item.status = status.clone();
    }
    if let Some(ref priority) = req.priority {
        work_item.priority = priority.clone();
    }
    if let Some(ref assignee_id) = req.assignee_id {
        work_item.assignee_id = if assignee_id.is_empty() {
            None
        } else {
            Some(Uuid::parse_str(assignee_id).map_err(|_| WsError::ValidationError {
                message: "Invalid assignee_id format".to_string(),
                field: Some("assignee_id".to_string()),
                location: ErrorLocation::from(Location::caller()),
            })?)
        };
    }
    if let Some(ref sprint_id) = req.sprint_id {
        work_item.sprint_id = if sprint_id.is_empty() {
            None
        } else {
            Some(Uuid::parse_str(sprint_id).map_err(|_| WsError::ValidationError {
                message: "Invalid sprint_id format".to_string(),
                field: Some("sprint_id".to_string()),
                location: ErrorLocation::from(Location::caller()),
            })?)
        };
    }
    if let Some(position) = req.position {
        work_item.position = position;
    }
    if let Some(story_points) = req.story_points {
        work_item.story_points = Some(story_points);
    }

    // 7. Update metadata
    let now = Utc::now();
    work_item.updated_at = now;
    work_item.updated_by = ctx.user_id;
    work_item.version += 1;

    // 8. Transaction: update + activity log
    let mut tx = ctx.pool.begin().await?;

    WorkItemRepository::update(&mut *tx, &work_item).await?;

    let changes_json = serde_json::to_string(&changes).ok();
    let activity = pm_core::ActivityLog {
        id: Uuid::new_v4(),
        entity_type: "work_item".to_string(),
        entity_id: work_item.id,
        action: "updated".to_string(),
        changes_json,
        user_id: ctx.user_id,
        created_at: now,
    };
    ActivityLogRepository::create(&mut *tx, &activity).await?;

    tx.commit().await?;

    Ok(build_work_item_updated_response(&ctx.message_id, &work_item, &changes, ctx.user_id))
}

pub async fn handle_delete(
    req: DeleteWorkItemRequest,
    ctx: HandlerContext,
) -> WsErrorResult<WebSocketMessage> {
    // 1. Parse work item ID
    let work_item_id = Uuid::parse_str(&req.work_item_id)
        .map_err(|_| WsError::ValidationError {
            message: "Invalid work_item_id format".to_string(),
            field: Some("work_item_id".to_string()),
            location: ErrorLocation::from(Location::caller()),
        })?;

    // 2. Fetch existing work item
    let work_item = WorkItemRepository::find_by_id(&ctx.pool, work_item_id)
        .await?
        .ok_or_else(|| WsError::NotFound {
            message: format!("Work item {} not found", work_item_id),
            location: ErrorLocation::from(Location::caller()),
        })?;

    // 3. Authorization
    check_permission(&ctx, work_item.project_id, Permission::Delete).await?;

    // 4. Check for children (prevent deleting items with children)
    let children = WorkItemRepository::find_children(&ctx.pool, work_item_id).await?;
    if !children.is_empty() {
        return Err(WsError::DeleteBlocked {
            message: format!("Cannot delete: {} child items exist", children.len()),
            location: ErrorLocation::from(Location::caller()),
        });
    }

    // 5. Transaction: soft delete + activity log
    let mut tx = ctx.pool.begin().await?;

    WorkItemRepository::soft_delete(&mut *tx, work_item_id, ctx.user_id).await?;

    let now = Utc::now();
    let activity = pm_core::ActivityLog {
        id: Uuid::new_v4(),
        entity_type: "work_item".to_string(),
        entity_id: work_item_id,
        action: "deleted".to_string(),
        changes_json: None,
        user_id: ctx.user_id,
        created_at: now,
    };
    ActivityLogRepository::create(&mut *tx, &activity).await?;

    tx.commit().await?;

    Ok(build_work_item_deleted_response(&ctx.message_id, work_item_id, ctx.user_id))
}
```

### 8. Create Query Handler

**Create**: `backend/crates/pm-ws/src/handlers/query.rs`

```rust
use crate::handlers::{
    context::HandlerContext,
    authorization::check_permission,
    response_builder::build_work_items_list_response,
};
use crate::{Result as WsErrorResult, WsError};
use pm_core::Permission;
use pm_db::WorkItemRepository;
use pm_proto::{GetWorkItemsRequest, WebSocketMessage};
use chrono::Utc;
use error_location::ErrorLocation;
use std::panic::Location;
use uuid::Uuid;

pub async fn handle_get_work_items(
    req: GetWorkItemsRequest,
    ctx: HandlerContext,
) -> WsErrorResult<WebSocketMessage> {
    // 1. Parse project ID
    let project_id = Uuid::parse_str(&req.project_id)
        .map_err(|_| WsError::ValidationError {
            message: "Invalid project_id format".to_string(),
            field: Some("project_id".to_string()),
            location: ErrorLocation::from(Location::caller()),
        })?;

    // 2. Authorization
    check_permission(&ctx, project_id, Permission::Read).await?;

    // 3. Fetch work items
    let as_of_timestamp = Utc::now().timestamp();
    let items = match req.since_timestamp {
        Some(since) => WorkItemRepository::find_by_project_since(&ctx.pool, project_id, since).await?,
        None => WorkItemRepository::find_by_project(&ctx.pool, project_id).await?,
    };

    Ok(build_work_items_list_response(&ctx.message_id, items, as_of_timestamp))
}
```

### 9. Update Module Exports

**File**: `backend/crates/pm-ws/src/handlers/mod.rs`

Add:
```rust
pub mod dispatcher;
pub mod work_item;
pub mod query;
```

---

## Files Summary

### Create (3 files)
| File | Purpose |
|------|---------|
| `pm-ws/src/handlers/dispatcher.rs` | Route messages to handlers |
| `pm-ws/src/handlers/work_item.rs` | CRUD handlers with full implementation |
| `pm-ws/src/handlers/query.rs` | Read handlers |

### Modify (6 files)
| File | Change |
|------|--------|
| `pm-ws/src/error.rs` | Add Database variant with From<pm_db::DbError> |
| `pm-server/src/error.rs` | Add 3 database variants to existing ServerError |
| `pm-server/src/main.rs` | Add database initialization |
| `pm-ws/src/app_state.rs` | Add SqlitePool field |
| `pm-ws/src/web_socket_connection.rs` | Wire handle_binary_message to dispatcher |
| `pm-ws/src/handlers/mod.rs` | Export new modules |

### DO NOT Create
- Error types (use existing WsError, ServerError)
- Response builders (use existing)
- Context (use existing HandlerContext)
- Idempotency functions (use existing)
- Authorization (use existing)
- Validation (use existing MessageValidator)
- Metrics (use existing)
- Migrations (schema is complete)

---

## Integration Test Example

**Create**: `backend/crates/pm-ws/tests/dispatcher_test.rs`

```rust
use pm_ws::handlers::{dispatcher::dispatch, context::HandlerContext};
use pm_proto::{WebSocketMessage, CreateWorkItemRequest, WorkItemType, web_socket_message::Payload};
use sqlx::SqlitePool;
use uuid::Uuid;

#[sqlx::test]
async fn test_create_work_item_dispatches_correctly(pool: SqlitePool) {
    // Create a project first (for FK constraint)
    let project_id = Uuid::new_v4();
    // ... setup project and project_member records ...

    let ctx = HandlerContext::new(
        "test-msg-1".to_string(),
        Uuid::new_v4(), // user_id
        pool.clone(),
    );

    let req = CreateWorkItemRequest {
        item_type: WorkItemType::Project as i32,
        title: "Test Project".to_string(),
        description: None,
        parent_id: None,
        project_id: project_id.to_string(),
    };

    let msg = WebSocketMessage {
        message_id: "test-msg-1".to_string(),
        timestamp: 0,
        payload: Some(Payload::CreateWorkItemRequest(req)),
    };

    let response = dispatch(msg, ctx).await;

    // Verify response is WorkItemCreated, not Error
    match response.payload {
        Some(Payload::WorkItemCreated(created)) => {
            assert!(created.work_item.is_some());
            let wi = created.work_item.unwrap();
            assert_eq!(wi.title, "Test Project");
        }
        Some(Payload::Error(e)) => panic!("Expected success, got error: {}", e.message),
        _ => panic!("Unexpected response type"),
    }
}
```

---

## Verification

```bash
# Build
cd backend && cargo build --workspace

# Run
cargo run --bin pm-server

# Verify database
sqlite3 .pm/data.db "PRAGMA journal_mode; PRAGMA foreign_keys;"

# Tests
cargo test --workspace
```

### Success Criteria
- [ ] Server starts and creates database
- [ ] Migrations run automatically
- [ ] WebSocket messages dispatch to handlers
- [ ] Create work item returns WorkItemCreated response
- [ ] Update work item handles optimistic locking
- [ ] Delete work item checks for children
- [ ] Idempotency returns cached responses on replay
- [ ] Authorization denies access for non-members
- [ ] Existing tests still pass
