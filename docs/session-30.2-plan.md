# Session 30.2: Handler Framework & Protobuf

## Overview

Create the handler module structure and add protobuf messages for queries and optimistic locking.

**Estimated Files**: 11
**Dependencies**: Session 30.1 complete (schema changes, new repositories)

---

## Phase 1: Protobuf Message Additions

### 1.1 Add Query Messages

**File**: `proto/messages.proto` (modify)

```protobuf
// Add these message definitions:

// Tag 33: Get work items for a project
message GetWorkItemsRequest {
  string project_id = 1;
  optional int64 since_timestamp = 2;  // For incremental sync after reconnection
}

// Tag 43: Response with work items list
message WorkItemsList {
  repeated WorkItem work_items = 1;
  int64 as_of_timestamp = 2;  // Client stores this for next sync
}
```

### 1.2 Add Version to UpdateWorkItemRequest

```protobuf
message UpdateWorkItemRequest {
  string work_item_id = 1;
  int32 expected_version = 2;  // ADD: For optimistic locking
  // ... existing optional fields remain unchanged ...
}
```

### 1.3 Update WebSocketMessage oneof

```protobuf
message WebSocketMessage {
  string message_id = 1;
  int64 timestamp = 2;

  oneof payload {
    // ... existing payloads ...

    // Add new payload types:
    GetWorkItemsRequest get_work_items_request = 33;
    WorkItemsList work_items_list = 43;
  }
}
```

---

## Phase 2: Handler Module Structure

### 2.1 Module Exports

**File**: `pm-ws/src/handlers/mod.rs`

```rust
//! WebSocket message handlers for work item operations.
//!
//! Each handler follows the pattern:
//! 1. Check idempotency (return cached if replay)
//! 2. Validate input
//! 3. Check authorization
//! 4. Execute business logic in transaction
//! 5. Build response
//! 6. Store idempotency key
//! 7. Return response + broadcast info

mod context;
mod error_codes;
mod response_builder;
mod authorization;
mod hierarchy_validator;
mod change_tracker;
mod idempotency;
pub mod subscription;
pub mod work_item;
pub mod query;

pub use context::HandlerContext;
pub use error_codes::*;
pub use response_builder::*;
pub use authorization::{check_permission, Permission};
pub use hierarchy_validator::validate_hierarchy;
pub use change_tracker::track_changes;
pub use idempotency::{check_idempotency, store_idempotency};
```

---

### 2.2 Handler Context

**File**: `pm-ws/src/handlers/context.rs`

```rust
use sqlx::SqlitePool;
use uuid::Uuid;

/// Context passed to all handlers containing request metadata and resources.
#[derive(Debug, Clone)]
pub struct HandlerContext {
    /// Unique message ID for request/response correlation
    pub message_id: String,
    /// Tenant ID extracted from JWT
    pub tenant_id: String,
    /// User ID extracted from JWT
    pub user_id: Uuid,
    /// Database connection pool for this tenant
    pub pool: SqlitePool,
}

impl HandlerContext {
    pub fn new(
        message_id: String,
        tenant_id: String,
        user_id: Uuid,
        pool: SqlitePool,
    ) -> Self {
        Self {
            message_id,
            tenant_id,
            user_id,
            pool,
        }
    }
}
```

---

### 2.3 Error Codes

**File**: `pm-ws/src/handlers/error_codes.rs`

```rust
//! Standard error codes for WebSocket responses.

/// Input validation failed
pub const VALIDATION_ERROR: &str = "VALIDATION_ERROR";

/// Resource not found
pub const NOT_FOUND: &str = "NOT_FOUND";

/// User lacks required permission
pub const UNAUTHORIZED: &str = "UNAUTHORIZED";

/// Optimistic lock conflict - resource was modified
pub const CONFLICT: &str = "CONFLICT";

/// Delete blocked due to dependencies
pub const DELETE_BLOCKED: &str = "DELETE_BLOCKED";

/// Internal server error
pub const INTERNAL_ERROR: &str = "INTERNAL_ERROR";

/// Invalid message format
pub const INVALID_MESSAGE: &str = "INVALID_MESSAGE";

/// Rate limit exceeded
pub const RATE_LIMITED: &str = "RATE_LIMITED";
```

---

### 2.4 Response Builder

**File**: `pm-ws/src/handlers/response_builder.rs`

```rust
use chrono::Utc;
use uuid::Uuid;
use pm_proto::{WebSocketMessage, WorkItem as ProtoWorkItem, WorkItemCreated, WorkItemUpdated, WorkItemDeleted, WorkItemsList, FieldChange, Error as ProtoError};
use crate::models::WorkItem;

/// Build WorkItemCreated response
pub fn build_work_item_created_response(
    message_id: &str,
    work_item: &WorkItem,
    actor_id: Uuid,
) -> WebSocketMessage {
    WebSocketMessage {
        message_id: message_id.to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(pm_proto::web_socket_message::Payload::WorkItemCreated(
            WorkItemCreated {
                work_item: Some(work_item_to_proto(work_item)),
                actor_id: actor_id.to_string(),
            }
        )),
    }
}

/// Build WorkItemUpdated response with field changes
pub fn build_work_item_updated_response(
    message_id: &str,
    work_item: &WorkItem,
    changes: &[FieldChange],
    actor_id: Uuid,
) -> WebSocketMessage {
    WebSocketMessage {
        message_id: message_id.to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(pm_proto::web_socket_message::Payload::WorkItemUpdated(
            WorkItemUpdated {
                work_item: Some(work_item_to_proto(work_item)),
                changes: changes.to_vec(),
                actor_id: actor_id.to_string(),
            }
        )),
    }
}

/// Build WorkItemDeleted response
pub fn build_work_item_deleted_response(
    message_id: &str,
    work_item_id: Uuid,
    actor_id: Uuid,
) -> WebSocketMessage {
    WebSocketMessage {
        message_id: message_id.to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(pm_proto::web_socket_message::Payload::WorkItemDeleted(
            WorkItemDeleted {
                work_item_id: work_item_id.to_string(),
                actor_id: actor_id.to_string(),
            }
        )),
    }
}

/// Build WorkItemsList response
pub fn build_work_items_list_response(
    message_id: &str,
    work_items: Vec<WorkItem>,
    as_of_timestamp: i64,
) -> WebSocketMessage {
    WebSocketMessage {
        message_id: message_id.to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(pm_proto::web_socket_message::Payload::WorkItemsList(
            WorkItemsList {
                work_items: work_items.iter().map(work_item_to_proto).collect(),
                as_of_timestamp,
            }
        )),
    }
}

/// Build error response
pub fn build_error_response(message_id: &str, error: ProtoError) -> WebSocketMessage {
    WebSocketMessage {
        message_id: message_id.to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(pm_proto::web_socket_message::Payload::Error(error)),
    }
}

/// Convert domain WorkItem to proto WorkItem
fn work_item_to_proto(item: &WorkItem) -> ProtoWorkItem {
    ProtoWorkItem {
        id: item.id.to_string(),
        item_type: item.item_type as i32,
        title: item.title.clone(),
        description: item.description.clone(),
        status: item.status as i32,
        priority: item.priority as i32,
        parent_id: item.parent_id.map(|id| id.to_string()),
        project_id: item.project_id.to_string(),
        assignee_id: item.assignee_id.map(|id| id.to_string()),
        position: item.position,
        story_points: item.story_points,
        version: item.version,
        created_at: item.created_at.timestamp(),
        updated_at: item.updated_at.timestamp(),
        created_by: item.created_by.to_string(),
        updated_by: item.updated_by.to_string(),
    }
}
```

---

### 2.5 Authorization Module

**File**: `pm-ws/src/handlers/authorization.rs`

```rust
use uuid::Uuid;
use pm_db::repositories::ProjectMemberRepository;
use pm_db::models::Permission;
use error_location::ErrLoc;
use crate::error::WsError;
use super::context::HandlerContext;

pub use pm_db::models::Permission;

/// Check if user has required permission on a project.
///
/// Returns Ok(()) if authorized, Err(WsError::Unauthorized) otherwise.
pub async fn check_permission(
    ctx: &HandlerContext,
    project_id: Uuid,
    required: Permission,
) -> Result<(), WsError> {
    let repo = ProjectMemberRepository::new(&ctx.pool);
    let member = repo
        .find_by_user_and_project(ctx.user_id, project_id)
        .await
        .map_err(|e| WsError::Internal {
            message: format!("Failed to check permission: {}", e),
            location: ErrLoc::caller(),
        })?;

    match member {
        None => Err(WsError::Unauthorized {
            message: "Not a member of this project".to_string(),
            location: ErrLoc::caller(),
        }),
        Some(m) if !m.has_permission(required) => Err(WsError::Unauthorized {
            message: format!(
                "Insufficient permission. Required: {:?}, have: {}",
                required, m.role
            ),
            location: ErrLoc::caller(),
        }),
        Some(_) => Ok(()),
    }
}
```

---

### 2.6 Hierarchy Validator

**File**: `pm-ws/src/handlers/hierarchy_validator.rs`

```rust
use sqlx::SqlitePool;
use uuid::Uuid;
use pm_db::repositories::WorkItemRepository;
use error_location::ErrLoc;
use crate::error::WsError;

/// Valid parent-child relationships:
/// - Project: no parent
/// - Epic: parent must be Project
/// - Story: parent must be Epic
/// - Task: parent must be Story
///
/// Returns Ok(()) if hierarchy is valid.
pub async fn validate_hierarchy(
    pool: &SqlitePool,
    child_type: i32, // WorkItemType enum value
    parent_id: Uuid,
) -> Result<(), WsError> {
    let repo = WorkItemRepository::new(pool);
    let parent = repo
        .find_by_id(parent_id)
        .await
        .map_err(|e| WsError::Internal {
            message: format!("Failed to fetch parent: {}", e),
            location: ErrLoc::caller(),
        })?
        .ok_or_else(|| WsError::ValidationError {
            message: "Parent work item not found".to_string(),
            field: Some("parent_id".to_string()),
            location: ErrLoc::caller(),
        })?;

    let valid = match (parent.item_type, child_type) {
        // Project (0) can contain Epic (1)
        (0, 1) => true,
        // Epic (1) can contain Story (2)
        (1, 2) => true,
        // Story (2) can contain Task (3)
        (2, 3) => true,
        _ => false,
    };

    if !valid {
        return Err(WsError::ValidationError {
            message: format!(
                "Invalid hierarchy: {} cannot be a child of {}",
                item_type_name(child_type),
                item_type_name(parent.item_type)
            ),
            field: Some("parent_id".to_string()),
            location: ErrLoc::caller(),
        });
    }

    Ok(())
}

fn item_type_name(type_value: i32) -> &'static str {
    match type_value {
        0 => "Project",
        1 => "Epic",
        2 => "Story",
        3 => "Task",
        _ => "Unknown",
    }
}
```

---

### 2.7 Change Tracker

**File**: `pm-ws/src/handlers/change_tracker.rs`

```rust
use pm_proto::{FieldChange, UpdateWorkItemRequest};
use crate::models::WorkItem;

/// Track which fields changed between current state and update request.
/// Returns list of FieldChange for the WorkItemUpdated event.
pub fn track_changes(current: &WorkItem, request: &UpdateWorkItemRequest) -> Vec<FieldChange> {
    let mut changes = Vec::new();

    if let Some(ref new_title) = request.title {
        if &current.title != new_title {
            changes.push(FieldChange {
                field_name: "title".to_string(),
                old_value: current.title.clone(),
                new_value: new_title.clone(),
            });
        }
    }

    if let Some(ref new_desc) = request.description {
        let current_desc = current.description.as_deref().unwrap_or("");
        if current_desc != new_desc {
            changes.push(FieldChange {
                field_name: "description".to_string(),
                old_value: current_desc.to_string(),
                new_value: new_desc.clone(),
            });
        }
    }

    if let Some(new_status) = request.status {
        if current.status as i32 != new_status {
            changes.push(FieldChange {
                field_name: "status".to_string(),
                old_value: format!("{}", current.status as i32),
                new_value: format!("{}", new_status),
            });
        }
    }

    if let Some(new_priority) = request.priority {
        if current.priority as i32 != new_priority {
            changes.push(FieldChange {
                field_name: "priority".to_string(),
                old_value: format!("{}", current.priority as i32),
                new_value: format!("{}", new_priority),
            });
        }
    }

    if let Some(ref new_assignee) = request.assignee_id {
        let current_assignee = current.assignee_id.map(|id| id.to_string()).unwrap_or_default();
        if current_assignee != *new_assignee {
            changes.push(FieldChange {
                field_name: "assignee_id".to_string(),
                old_value: current_assignee,
                new_value: new_assignee.clone(),
            });
        }
    }

    if let Some(new_points) = request.story_points {
        let current_points = current.story_points.unwrap_or(0);
        if current_points != new_points {
            changes.push(FieldChange {
                field_name: "story_points".to_string(),
                old_value: format!("{}", current_points),
                new_value: format!("{}", new_points),
            });
        }
    }

    if let Some(new_position) = request.position {
        if current.position != new_position {
            changes.push(FieldChange {
                field_name: "position".to_string(),
                old_value: format!("{}", current.position),
                new_value: format!("{}", new_position),
            });
        }
    }

    changes
}
```

---

### 2.8 Idempotency Module

**File**: `pm-ws/src/handlers/idempotency.rs`

```rust
use sqlx::SqlitePool;
use pm_db::repositories::IdempotencyRepository;
use error_location::ErrLoc;
use crate::error::WsError;

/// Check if a message has already been processed.
/// Returns Some(cached_result) if replay, None if new request.
pub async fn check_idempotency(
    pool: &SqlitePool,
    message_id: &str,
) -> Result<Option<String>, WsError> {
    let repo = IdempotencyRepository::new(pool);
    repo.find_by_message_id(message_id)
        .await
        .map_err(|e| WsError::Internal {
            message: format!("Failed to check idempotency: {}", e),
            location: ErrLoc::caller(),
        })
}

/// Store the result of a successful operation for idempotency.
pub async fn store_idempotency(
    pool: &SqlitePool,
    message_id: &str,
    operation: &str,
    result_json: &str,
) -> Result<(), WsError> {
    let repo = IdempotencyRepository::new(pool);
    repo.create(message_id, operation, result_json)
        .await
        .map_err(|e| WsError::Internal {
            message: format!("Failed to store idempotency: {}", e),
            location: ErrLoc::caller(),
        })
}
```

---

## Phase 3: Error Extensions

### 3.1 Extend WsError

**File**: `pm-ws/src/error.rs` (modify)

Add new error variants:

```rust
#[derive(Debug, Error)]
pub enum WsError {
    // ... existing variants ...

    #[error("Validation failed: {message}")]
    ValidationError {
        message: String,
        field: Option<String>,
        location: ErrLoc,
    },

    #[error("Conflict: resource was modified (current version: {current_version})")]
    ConflictError {
        current_version: i32,
        location: ErrLoc,
    },

    #[error("Cannot delete: {message}")]
    DeleteBlocked {
        message: String,
        location: ErrLoc,
    },

    #[error("Unauthorized: {message}")]
    Unauthorized {
        message: String,
        location: ErrLoc,
    },
}

impl WsError {
    pub fn to_proto_error(&self) -> pm_proto::Error {
        pm_proto::Error {
            code: self.error_code().to_string(),
            message: self.to_string(),
            field: match self {
                WsError::ValidationError { field, .. } => field.clone(),
                _ => None,
            },
        }
    }

    fn error_code(&self) -> &'static str {
        match self {
            WsError::ValidationError { .. } => super::handlers::VALIDATION_ERROR,
            WsError::ConflictError { .. } => super::handlers::CONFLICT,
            WsError::DeleteBlocked { .. } => super::handlers::DELETE_BLOCKED,
            WsError::Unauthorized { .. } => super::handlers::UNAUTHORIZED,
            WsError::NotFound { .. } => super::handlers::NOT_FOUND,
            _ => super::handlers::INTERNAL_ERROR,
        }
    }
}
```

---

### 3.2 Add Structured Logging Helper

**File**: `pm-ws/src/lib.rs` (add function)

```rust
use tracing::info_span;

/// Create a tracing span for a WebSocket request.
/// All log entries within the handler will include these fields.
pub fn create_request_span(
    message_id: &str,
    tenant_id: &str,
    user_id: &str,
    operation: &str,
) -> tracing::Span {
    info_span!(
        "ws_request",
        message_id = %message_id,
        tenant_id = %tenant_id,
        user_id = %user_id,
        operation = %operation,
    )
}
```

---

## File Summary

| Action | Path |
|--------|------|
| Modify | `proto/messages.proto` |
| Create | `pm-ws/src/handlers/mod.rs` |
| Create | `pm-ws/src/handlers/context.rs` |
| Create | `pm-ws/src/handlers/error_codes.rs` |
| Create | `pm-ws/src/handlers/response_builder.rs` |
| Create | `pm-ws/src/handlers/authorization.rs` |
| Create | `pm-ws/src/handlers/hierarchy_validator.rs` |
| Create | `pm-ws/src/handlers/change_tracker.rs` |
| Create | `pm-ws/src/handlers/idempotency.rs` |
| Modify | `pm-ws/src/error.rs` |
| Modify | `pm-ws/src/lib.rs` |

---

## Verification

```bash
cd backend

# Regenerate proto
cargo build -p pm-proto

# Build workspace (handlers module should compile)
cargo build --workspace

# Run tests
cargo test --workspace
```

---

## Tests to Add

```rust
#[test]
fn test_track_changes_title() {
    let current = WorkItem { title: "Old".into(), ..default() };
    let request = UpdateWorkItemRequest { title: Some("New".into()), ..default() };
    let changes = track_changes(&current, &request);
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].field_name, "title");
}

#[test]
fn test_track_changes_no_change() {
    let current = WorkItem { title: "Same".into(), ..default() };
    let request = UpdateWorkItemRequest { title: Some("Same".into()), ..default() };
    let changes = track_changes(&current, &request);
    assert!(changes.is_empty());
}
```
