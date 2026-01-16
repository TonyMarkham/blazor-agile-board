# Session 30.3: Work Item Handlers & Dispatcher

## Overview

Implement the CRUD handlers for work items, the query handler for initial load, and integrate with the message dispatcher. Add comprehensive handler tests.

**Estimated Files**: 6
**Dependencies**: Session 30.2 complete (handler framework exists)

---

## Phase 1: Subscription Handler

### 1.1 Subscribe/Unsubscribe Handlers

**File**: `pm-ws/src/handlers/subscription.rs`

```rust
use std::collections::HashSet;
use uuid::Uuid;
use pm_proto::{Subscribe, Unsubscribe, SubscribeAck};
use error_location::ErrLoc;
use crate::error::WsError;

/// Handle subscription request.
/// Adds project_id to the connection's subscription set.
pub async fn handle_subscribe(
    request: Subscribe,
    subscriptions: &mut HashSet<Uuid>,
) -> Result<SubscribeAck, WsError> {
    let project_id = Uuid::parse_str(&request.project_id)
        .map_err(|_| WsError::ValidationError {
            message: "Invalid project_id".to_string(),
            field: Some("project_id".to_string()),
            location: ErrLoc::caller(),
        })?;

    subscriptions.insert(project_id);

    Ok(SubscribeAck {
        project_id: request.project_id,
        success: true,
    })
}

/// Handle unsubscribe request.
/// Removes project_id from the connection's subscription set.
pub async fn handle_unsubscribe(
    request: Unsubscribe,
    subscriptions: &mut HashSet<Uuid>,
) -> Result<(), WsError> {
    let project_id = Uuid::parse_str(&request.project_id)
        .map_err(|_| WsError::ValidationError {
            message: "Invalid project_id".to_string(),
            field: Some("project_id".to_string()),
            location: ErrLoc::caller(),
        })?;

    subscriptions.remove(&project_id);

    Ok(())
}
```

---

## Phase 2: Work Item Handlers

### 2.1 Work Item CRUD Handlers

**File**: `pm-ws/src/handlers/work_item.rs`

```rust
use uuid::Uuid;
use chrono::Utc;
use tracing::Instrument;
use pm_proto::{CreateWorkItemRequest, UpdateWorkItemRequest, DeleteWorkItemRequest};
use pm_db::repositories::{WorkItemRepository, ActivityLogRepository};
use error_location::ErrLoc;

use crate::error::WsError;
use crate::create_request_span;
use crate::broadcast::BroadcastInfo;
use super::{
    HandlerContext, Permission,
    check_permission, validate_hierarchy, track_changes,
    check_idempotency, store_idempotency,
    build_work_item_created_response, build_work_item_updated_response, build_work_item_deleted_response,
};

/// Handle CreateWorkItemRequest.
///
/// Flow:
/// 1. Check idempotency
/// 2. Validate input
/// 3. Parse UUIDs
/// 4. Check authorization (Edit permission)
/// 5. Validate hierarchy
/// 6. Calculate position
/// 7. Create domain model
/// 8. Execute in transaction
/// 9. Build response
/// 10. Store idempotency key
/// 11. Return response + broadcast info
pub async fn handle_create(
    request: CreateWorkItemRequest,
    ctx: HandlerContext,
) -> Result<(pm_proto::WebSocketMessage, BroadcastInfo), WsError> {
    let span = create_request_span(
        &ctx.message_id,
        &ctx.tenant_id,
        &ctx.user_id.to_string(),
        "create_work_item",
    );

    async {
        // 1. Check idempotency
        if let Some(cached) = check_idempotency(&ctx.pool, &ctx.message_id).await? {
            return deserialize_cached_response(&cached);
        }

        // 2. Validate input
        validate_create_request(&request)?;

        // 3. Parse UUIDs
        let project_id = parse_uuid(&request.project_id, "project_id")?;
        let parent_id = request
            .parent_id
            .as_ref()
            .map(|s| parse_uuid(s, "parent_id"))
            .transpose()?;

        // 4. Check authorization
        check_permission(&ctx, project_id, Permission::Edit).await?;

        // 5. Validate hierarchy
        if let Some(pid) = parent_id {
            validate_hierarchy(&ctx.pool, request.item_type, pid).await?;
        }

        // 6. Calculate next position
        let position = calculate_next_position(&ctx.pool, project_id, parent_id).await?;

        // 7. Create domain model
        let work_item = WorkItem::new(
            request.item_type.into(),
            request.title.clone(),
            request.description.clone(),
            parent_id,
            project_id,
            ctx.user_id,
        );
        work_item.position = position;

        // 8. Transaction
        let mut tx = ctx.pool.begin().await.map_err(|e| WsError::Internal {
            message: format!("Failed to begin transaction: {}", e),
            location: ErrLoc::caller(),
        })?;

        let work_item_repo = WorkItemRepository::new_with_tx(&mut tx);
        work_item_repo.create(&work_item).await.map_err(|e| WsError::Internal {
            message: format!("Failed to create work item: {}", e),
            location: ErrLoc::caller(),
        })?;

        let activity_repo = ActivityLogRepository::new_with_tx(&mut tx);
        activity_repo
            .create(&ActivityLog::created("WorkItem", work_item.id, ctx.user_id))
            .await
            .map_err(|e| WsError::Internal {
                message: format!("Failed to log activity: {}", e),
                location: ErrLoc::caller(),
            })?;

        tx.commit().await.map_err(|e| WsError::Internal {
            message: format!("Failed to commit transaction: {}", e),
            location: ErrLoc::caller(),
        })?;

        // 9. Build response
        let response = build_work_item_created_response(&ctx.message_id, &work_item, ctx.user_id);

        // 10. Store idempotency
        let response_json = serde_json::to_string(&response).unwrap_or_default();
        store_idempotency(&ctx.pool, &ctx.message_id, "create_work_item", &response_json).await?;

        // 11. Return
        Ok((response, BroadcastInfo::new(project_id, "work_item_created")))
    }
    .instrument(span)
    .await
}

/// Handle UpdateWorkItemRequest with optimistic locking.
pub async fn handle_update(
    request: UpdateWorkItemRequest,
    ctx: HandlerContext,
) -> Result<(pm_proto::WebSocketMessage, BroadcastInfo), WsError> {
    let span = create_request_span(
        &ctx.message_id,
        &ctx.tenant_id,
        &ctx.user_id.to_string(),
        "update_work_item",
    );

    async {
        // 1. Parse work item ID
        let work_item_id = parse_uuid(&request.work_item_id, "work_item_id")?;

        // 2. Fetch existing item
        let repo = WorkItemRepository::new(&ctx.pool);
        let mut work_item = repo
            .find_by_id(work_item_id)
            .await
            .map_err(|e| WsError::Internal {
                message: format!("Failed to fetch work item: {}", e),
                location: ErrLoc::caller(),
            })?
            .ok_or_else(|| WsError::NotFound {
                resource: "WorkItem".to_string(),
                id: work_item_id.to_string(),
                location: ErrLoc::caller(),
            })?;

        // 3. Check authorization
        check_permission(&ctx, work_item.project_id, Permission::Edit).await?;

        // 4. Optimistic lock check
        if work_item.version != request.expected_version {
            return Err(WsError::ConflictError {
                current_version: work_item.version,
                location: ErrLoc::caller(),
            });
        }

        // 5. Track changes
        let changes = track_changes(&work_item, &request);

        // 6. Apply updates
        apply_updates(&mut work_item, &request);
        work_item.version += 1;
        work_item.updated_at = Utc::now();
        work_item.updated_by = ctx.user_id;

        // 7. Transaction
        let mut tx = ctx.pool.begin().await.map_err(|e| WsError::Internal {
            message: format!("Failed to begin transaction: {}", e),
            location: ErrLoc::caller(),
        })?;

        let work_item_repo = WorkItemRepository::new_with_tx(&mut tx);
        work_item_repo.update(&work_item).await.map_err(|e| WsError::Internal {
            message: format!("Failed to update work item: {}", e),
            location: ErrLoc::caller(),
        })?;

        let activity_repo = ActivityLogRepository::new_with_tx(&mut tx);
        activity_repo
            .create(&ActivityLog::updated("WorkItem", work_item.id, ctx.user_id, &changes))
            .await
            .map_err(|e| WsError::Internal {
                message: format!("Failed to log activity: {}", e),
                location: ErrLoc::caller(),
            })?;

        tx.commit().await.map_err(|e| WsError::Internal {
            message: format!("Failed to commit transaction: {}", e),
            location: ErrLoc::caller(),
        })?;

        // 8. Build response
        let response = build_work_item_updated_response(&ctx.message_id, &work_item, &changes, ctx.user_id);

        Ok((response, BroadcastInfo::new(work_item.project_id, "work_item_updated")))
    }
    .instrument(span)
    .await
}

/// Handle DeleteWorkItemRequest with cascade check.
pub async fn handle_delete(
    request: DeleteWorkItemRequest,
    ctx: HandlerContext,
) -> Result<(pm_proto::WebSocketMessage, BroadcastInfo), WsError> {
    let span = create_request_span(
        &ctx.message_id,
        &ctx.tenant_id,
        &ctx.user_id.to_string(),
        "delete_work_item",
    );

    async {
        // 1. Parse work item ID
        let work_item_id = parse_uuid(&request.work_item_id, "work_item_id")?;

        // 2. Fetch existing item
        let repo = WorkItemRepository::new(&ctx.pool);
        let work_item = repo
            .find_by_id(work_item_id)
            .await
            .map_err(|e| WsError::Internal {
                message: format!("Failed to fetch work item: {}", e),
                location: ErrLoc::caller(),
            })?
            .ok_or_else(|| WsError::NotFound {
                resource: "WorkItem".to_string(),
                id: work_item_id.to_string(),
                location: ErrLoc::caller(),
            })?;

        // 3. Check authorization
        check_permission(&ctx, work_item.project_id, Permission::Edit).await?;

        // 4. Check for children
        let children = repo
            .find_children(work_item_id)
            .await
            .map_err(|e| WsError::Internal {
                message: format!("Failed to check children: {}", e),
                location: ErrLoc::caller(),
            })?;

        if !children.is_empty() {
            return Err(WsError::DeleteBlocked {
                message: format!(
                    "Cannot delete: {} child items exist. Delete children first.",
                    children.len()
                ),
                location: ErrLoc::caller(),
            });
        }

        // 5. Soft delete in transaction
        let mut tx = ctx.pool.begin().await.map_err(|e| WsError::Internal {
            message: format!("Failed to begin transaction: {}", e),
            location: ErrLoc::caller(),
        })?;

        let work_item_repo = WorkItemRepository::new_with_tx(&mut tx);
        work_item_repo
            .soft_delete(work_item_id, ctx.user_id)
            .await
            .map_err(|e| WsError::Internal {
                message: format!("Failed to delete work item: {}", e),
                location: ErrLoc::caller(),
            })?;

        let activity_repo = ActivityLogRepository::new_with_tx(&mut tx);
        activity_repo
            .create(&ActivityLog::deleted("WorkItem", work_item_id, ctx.user_id))
            .await
            .map_err(|e| WsError::Internal {
                message: format!("Failed to log activity: {}", e),
                location: ErrLoc::caller(),
            })?;

        tx.commit().await.map_err(|e| WsError::Internal {
            message: format!("Failed to commit transaction: {}", e),
            location: ErrLoc::caller(),
        })?;

        // 6. Build response
        let response = build_work_item_deleted_response(&ctx.message_id, work_item_id, ctx.user_id);

        Ok((response, BroadcastInfo::new(work_item.project_id, "work_item_deleted")))
    }
    .instrument(span)
    .await
}

// Helper functions

fn parse_uuid(s: &str, field: &str) -> Result<Uuid, WsError> {
    Uuid::parse_str(s).map_err(|_| WsError::ValidationError {
        message: format!("Invalid UUID: {}", s),
        field: Some(field.to_string()),
        location: ErrLoc::caller(),
    })
}

fn validate_create_request(request: &CreateWorkItemRequest) -> Result<(), WsError> {
    if request.title.trim().is_empty() {
        return Err(WsError::ValidationError {
            message: "Title cannot be empty".to_string(),
            field: Some("title".to_string()),
            location: ErrLoc::caller(),
        });
    }
    if request.title.len() > 500 {
        return Err(WsError::ValidationError {
            message: "Title too long (max 500 chars)".to_string(),
            field: Some("title".to_string()),
            location: ErrLoc::caller(),
        });
    }
    Ok(())
}

fn apply_updates(work_item: &mut WorkItem, request: &UpdateWorkItemRequest) {
    if let Some(ref title) = request.title {
        work_item.title = title.clone();
    }
    if let Some(ref desc) = request.description {
        work_item.description = Some(desc.clone());
    }
    if let Some(status) = request.status {
        work_item.status = status.into();
    }
    if let Some(priority) = request.priority {
        work_item.priority = priority.into();
    }
    if let Some(ref assignee) = request.assignee_id {
        work_item.assignee_id = Some(Uuid::parse_str(assignee).ok()).flatten();
    }
    if let Some(points) = request.story_points {
        work_item.story_points = Some(points);
    }
    if let Some(position) = request.position {
        work_item.position = position;
    }
}

async fn calculate_next_position(
    pool: &SqlitePool,
    project_id: Uuid,
    parent_id: Option<Uuid>,
) -> Result<i32, WsError> {
    let repo = WorkItemRepository::new(pool);
    let max_position = repo
        .find_max_position(project_id, parent_id)
        .await
        .map_err(|e| WsError::Internal {
            message: format!("Failed to get max position: {}", e),
            location: ErrLoc::caller(),
        })?;
    Ok(max_position + 1)
}

fn deserialize_cached_response(json: &str) -> Result<(pm_proto::WebSocketMessage, BroadcastInfo), WsError> {
    // Deserialize cached response from idempotency store
    // This is simplified - actual impl needs proper deserialization
    Err(WsError::Internal {
        message: "Cached response replay not yet implemented".to_string(),
        location: ErrLoc::caller(),
    })
}
```

---

## Phase 3: Query Handler

### 3.1 GetWorkItems Handler

**File**: `pm-ws/src/handlers/query.rs`

```rust
use uuid::Uuid;
use chrono::Utc;
use tracing::Instrument;
use pm_proto::{GetWorkItemsRequest, WebSocketMessage};
use pm_db::repositories::WorkItemRepository;
use error_location::ErrLoc;

use crate::error::WsError;
use crate::create_request_span;
use super::{HandlerContext, Permission, check_permission, build_work_items_list_response};

/// Handle GetWorkItemsRequest for initial load and reconnection sync.
pub async fn handle_get_work_items(
    request: GetWorkItemsRequest,
    ctx: HandlerContext,
) -> Result<WebSocketMessage, WsError> {
    let span = create_request_span(
        &ctx.message_id,
        &ctx.tenant_id,
        &ctx.user_id.to_string(),
        "get_work_items",
    );

    async {
        // 1. Parse project ID
        let project_id = Uuid::parse_str(&request.project_id)
            .map_err(|_| WsError::ValidationError {
                message: "Invalid project_id".to_string(),
                field: Some("project_id".to_string()),
                location: ErrLoc::caller(),
            })?;

        // 2. Check View permission (not Edit)
        check_permission(&ctx, project_id, Permission::View).await?;

        // 3. Query work items
        let repo = WorkItemRepository::new(&ctx.pool);
        let work_items = match request.since_timestamp {
            Some(ts) => {
                // Incremental sync - items modified since timestamp
                repo.find_by_project_since(project_id, ts)
                    .await
                    .map_err(|e| WsError::Internal {
                        message: format!("Failed to query work items: {}", e),
                        location: ErrLoc::caller(),
                    })?
            }
            None => {
                // Full load - all items for project
                repo.find_by_project(project_id)
                    .await
                    .map_err(|e| WsError::Internal {
                        message: format!("Failed to query work items: {}", e),
                        location: ErrLoc::caller(),
                    })?
            }
        };

        // 4. Build response with current timestamp
        let response = build_work_items_list_response(
            &ctx.message_id,
            work_items,
            Utc::now().timestamp(),
        );

        Ok(response)
    }
    .instrument(span)
    .await
}
```

---

## Phase 4: Message Dispatcher Integration

### 4.1 Update WebSocketConnection

**File**: `pm-ws/src/web_socket_connection.rs` (modify)

Add the message dispatch logic to `handle_binary_message`:

```rust
use pm_proto::web_socket_message::Payload;
use crate::handlers::{self, HandlerContext};

impl WebSocketConnection {
    async fn handle_binary_message(&mut self, data: &[u8]) -> Result<(), WsError> {
        // Decode message with timeout
        let message = tokio::time::timeout(
            Duration::from_secs(1),
            async { WebSocketMessage::decode(data) }
        )
        .await
        .map_err(|_| WsError::Internal {
            message: "Decode timeout".to_string(),
            location: ErrLoc::caller(),
        })?
        .map_err(|e| WsError::ProtoDecode {
            source: e,
            location: ErrLoc::caller(),
        })?;

        // Create handler context
        let ctx = HandlerContext::new(
            message.message_id.clone(),
            self.tenant_context.tenant_id.clone(),
            self.tenant_context.user_id,
            self.connection_manager
                .get_pool(&self.tenant_context.tenant_id)
                .await?,
        );

        // Dispatch based on payload type
        let result = match message.payload {
            Some(Payload::CreateWorkItemRequest(req)) => {
                let (response, broadcast) = handlers::work_item::handle_create(req, ctx).await?;
                self.send_response_and_broadcast(response, broadcast).await
            }
            Some(Payload::UpdateWorkItemRequest(req)) => {
                let (response, broadcast) = handlers::work_item::handle_update(req, ctx).await?;
                self.send_response_and_broadcast(response, broadcast).await
            }
            Some(Payload::DeleteWorkItemRequest(req)) => {
                let (response, broadcast) = handlers::work_item::handle_delete(req, ctx).await?;
                self.send_response_and_broadcast(response, broadcast).await
            }
            Some(Payload::GetWorkItemsRequest(req)) => {
                let response = handlers::query::handle_get_work_items(req, ctx).await?;
                self.send_response(response).await
            }
            Some(Payload::Subscribe(req)) => {
                let ack = handlers::subscription::handle_subscribe(req, &mut self.subscriptions).await?;
                // Send ack back to client
                self.send_subscribe_ack(ack).await
            }
            Some(Payload::Unsubscribe(req)) => {
                handlers::subscription::handle_unsubscribe(req, &mut self.subscriptions).await?;
                Ok(())
            }
            _ => {
                Err(WsError::InvalidMessage {
                    message: "Unknown or unsupported payload type".to_string(),
                    location: ErrLoc::caller(),
                })
            }
        };

        // Handle errors by sending error response
        if let Err(ref e) = result {
            let error_response = handlers::build_error_response(&message.message_id, e.to_proto_error());
            self.send_response(error_response).await?;
        }

        result
    }

    async fn send_response_and_broadcast(
        &self,
        response: WebSocketMessage,
        broadcast_info: BroadcastInfo,
    ) -> Result<(), WsError> {
        // Send response to requester
        self.send_response(response.clone()).await?;

        // Broadcast to other subscribers
        self.broadcaster
            .broadcast(
                &self.tenant_context.tenant_id,
                BroadcastMessage {
                    payload: response.encode_to_vec().into(),
                    event_type: broadcast_info.event_type,
                    project_id: Some(broadcast_info.project_id.to_string()),
                },
            )
            .await?;

        Ok(())
    }
}
```

---

## Phase 5: Handler Tests

### 5.1 Comprehensive Handler Tests

**File**: `pm-ws/tests/work_item_handler_tests.rs`

```rust
use std::sync::Arc;
use uuid::Uuid;
use sqlx::SqlitePool;
use pm_ws::handlers::{self, HandlerContext};
use pm_proto::{CreateWorkItemRequest, UpdateWorkItemRequest, DeleteWorkItemRequest, WorkItemType};
use pm_db::repositories::{WorkItemRepository, ProjectMemberRepository};

async fn setup_test_context() -> (SqlitePool, HandlerContext) {
    let pool = create_test_pool().await;
    run_migrations(&pool).await;

    let ctx = HandlerContext::new(
        Uuid::new_v4().to_string(),
        "test-tenant".to_string(),
        Uuid::new_v4(),
        pool.clone(),
    );

    (pool, ctx)
}

async fn create_test_project(pool: &SqlitePool) -> WorkItem {
    let repo = WorkItemRepository::new(pool);
    let project = WorkItem::new_project("Test Project", Uuid::new_v4());
    repo.create(&project).await.unwrap();
    project
}

async fn add_project_member(pool: &SqlitePool, project_id: Uuid, user_id: Uuid, role: &str) {
    let repo = ProjectMemberRepository::new(pool);
    let member = ProjectMember::new(project_id, user_id, role);
    repo.create(&member).await.unwrap();
}

// ============ Create Tests ============

#[tokio::test]
async fn test_create_work_item_success() {
    let (pool, mut ctx) = setup_test_context().await;

    // Setup: Create project and add user as editor
    let project = create_test_project(&pool).await;
    add_project_member(&pool, project.id, ctx.user_id, "editor").await;

    let request = CreateWorkItemRequest {
        item_type: WorkItemType::Story as i32,
        title: "Test Story".to_string(),
        description: Some("Description".to_string()),
        project_id: project.id.to_string(),
        parent_id: None,
        ..Default::default()
    };

    let result = handlers::work_item::handle_create(request, ctx).await;

    assert!(result.is_ok());
    let (response, broadcast) = result.unwrap();
    assert_eq!(broadcast.event_type, "work_item_created");

    // Verify item was created in DB
    let repo = WorkItemRepository::new(&pool);
    let items = repo.find_by_project(project.id).await.unwrap();
    assert_eq!(items.len(), 2); // project + story
}

#[tokio::test]
async fn test_create_work_item_unauthorized() {
    let (pool, ctx) = setup_test_context().await;

    // Setup: Create project but DON'T add user as member
    let project = create_test_project(&pool).await;

    let request = CreateWorkItemRequest {
        item_type: WorkItemType::Story as i32,
        title: "Test Story".to_string(),
        project_id: project.id.to_string(),
        ..Default::default()
    };

    let result = handlers::work_item::handle_create(request, ctx).await;

    assert!(matches!(result, Err(WsError::Unauthorized { .. })));
}

#[tokio::test]
async fn test_create_work_item_invalid_hierarchy() {
    let (pool, mut ctx) = setup_test_context().await;

    let project = create_test_project(&pool).await;
    add_project_member(&pool, project.id, ctx.user_id, "editor").await;

    // Try to create a Task directly under a Project (invalid)
    let request = CreateWorkItemRequest {
        item_type: WorkItemType::Task as i32,
        title: "Test Task".to_string(),
        project_id: project.id.to_string(),
        parent_id: Some(project.id.to_string()), // Project -> Task is invalid
        ..Default::default()
    };

    let result = handlers::work_item::handle_create(request, ctx).await;

    assert!(matches!(result, Err(WsError::ValidationError { .. })));
}

// ============ Update Tests ============

#[tokio::test]
async fn test_update_work_item_success() {
    let (pool, mut ctx) = setup_test_context().await;

    let project = create_test_project(&pool).await;
    add_project_member(&pool, project.id, ctx.user_id, "editor").await;

    // Create a story
    let story = create_test_story(&pool, project.id, ctx.user_id).await;

    let request = UpdateWorkItemRequest {
        work_item_id: story.id.to_string(),
        expected_version: 0,
        title: Some("Updated Title".to_string()),
        ..Default::default()
    };

    let result = handlers::work_item::handle_update(request, ctx).await;

    assert!(result.is_ok());
    let (response, broadcast) = result.unwrap();
    assert_eq!(broadcast.event_type, "work_item_updated");

    // Verify update
    let repo = WorkItemRepository::new(&pool);
    let updated = repo.find_by_id(story.id).await.unwrap().unwrap();
    assert_eq!(updated.title, "Updated Title");
    assert_eq!(updated.version, 1);
}

#[tokio::test]
async fn test_update_work_item_conflict() {
    let (pool, mut ctx) = setup_test_context().await;

    let project = create_test_project(&pool).await;
    add_project_member(&pool, project.id, ctx.user_id, "editor").await;

    let story = create_test_story(&pool, project.id, ctx.user_id).await;

    // Simulate concurrent update by incrementing version directly
    update_item_version(&pool, story.id, 1).await;

    let request = UpdateWorkItemRequest {
        work_item_id: story.id.to_string(),
        expected_version: 0, // Stale version
        title: Some("Updated Title".to_string()),
        ..Default::default()
    };

    let result = handlers::work_item::handle_update(request, ctx).await;

    assert!(matches!(result, Err(WsError::ConflictError { current_version: 1, .. })));
}

// ============ Delete Tests ============

#[tokio::test]
async fn test_delete_work_item_success() {
    let (pool, mut ctx) = setup_test_context().await;

    let project = create_test_project(&pool).await;
    add_project_member(&pool, project.id, ctx.user_id, "editor").await;

    let story = create_test_story(&pool, project.id, ctx.user_id).await;

    let request = DeleteWorkItemRequest {
        work_item_id: story.id.to_string(),
    };

    let result = handlers::work_item::handle_delete(request, ctx).await;

    assert!(result.is_ok());

    // Verify soft delete
    let repo = WorkItemRepository::new(&pool);
    let deleted = repo.find_by_id(story.id).await.unwrap();
    assert!(deleted.is_none()); // find_by_id excludes soft-deleted
}

#[tokio::test]
async fn test_delete_blocked_by_children() {
    let (pool, mut ctx) = setup_test_context().await;

    let project = create_test_project(&pool).await;
    add_project_member(&pool, project.id, ctx.user_id, "editor").await;

    // Create epic with a story child
    let epic = create_test_epic(&pool, project.id, ctx.user_id).await;
    let _story = create_test_story_with_parent(&pool, project.id, epic.id, ctx.user_id).await;

    let request = DeleteWorkItemRequest {
        work_item_id: epic.id.to_string(),
    };

    let result = handlers::work_item::handle_delete(request, ctx).await;

    assert!(matches!(result, Err(WsError::DeleteBlocked { .. })));
}

// ============ Idempotency Tests ============

#[tokio::test]
async fn test_create_idempotency() {
    let (pool, ctx) = setup_test_context().await;

    let project = create_test_project(&pool).await;
    add_project_member(&pool, project.id, ctx.user_id, "editor").await;

    let message_id = Uuid::new_v4().to_string();

    let request = CreateWorkItemRequest {
        item_type: WorkItemType::Story as i32,
        title: "Test Story".to_string(),
        project_id: project.id.to_string(),
        ..Default::default()
    };

    // Create context with fixed message_id
    let ctx1 = HandlerContext::new(
        message_id.clone(),
        ctx.tenant_id.clone(),
        ctx.user_id,
        pool.clone(),
    );

    // First call - creates item
    let result1 = handlers::work_item::handle_create(request.clone(), ctx1).await;
    assert!(result1.is_ok());

    // Second call with same message_id - should return cached
    let ctx2 = HandlerContext::new(
        message_id,
        ctx.tenant_id,
        ctx.user_id,
        pool.clone(),
    );
    let result2 = handlers::work_item::handle_create(request, ctx2).await;
    assert!(result2.is_ok());

    // Verify only one item created
    let repo = WorkItemRepository::new(&pool);
    let items = repo.find_by_project(project.id).await.unwrap();
    // Should be 2: project + 1 story (not 3)
    assert_eq!(items.len(), 2);
}

// ============ Query Tests ============

#[tokio::test]
async fn test_get_work_items_full_load() {
    let (pool, mut ctx) = setup_test_context().await;

    let project = create_test_project(&pool).await;
    add_project_member(&pool, project.id, ctx.user_id, "viewer").await; // Viewer is enough

    // Create some items
    create_test_story(&pool, project.id, ctx.user_id).await;
    create_test_story(&pool, project.id, ctx.user_id).await;

    let request = GetWorkItemsRequest {
        project_id: project.id.to_string(),
        since_timestamp: None,
    };

    let result = handlers::query::handle_get_work_items(request, ctx).await;

    assert!(result.is_ok());
    // Verify response contains all items
}

#[tokio::test]
async fn test_get_work_items_incremental_sync() {
    let (pool, mut ctx) = setup_test_context().await;

    let project = create_test_project(&pool).await;
    add_project_member(&pool, project.id, ctx.user_id, "viewer").await;

    // Create story with old timestamp
    let old_story = create_test_story_with_timestamp(&pool, project.id, ctx.user_id, old_timestamp).await;

    // Create story with new timestamp
    let new_story = create_test_story(&pool, project.id, ctx.user_id).await;

    let request = GetWorkItemsRequest {
        project_id: project.id.to_string(),
        since_timestamp: Some(cutoff_timestamp),
    };

    let result = handlers::query::handle_get_work_items(request, ctx).await;

    assert!(result.is_ok());
    // Verify response contains only new_story
}

// Helper functions for tests
async fn create_test_pool() -> SqlitePool { /* ... */ }
async fn run_migrations(pool: &SqlitePool) { /* ... */ }
async fn create_test_story(pool: &SqlitePool, project_id: Uuid, user_id: Uuid) -> WorkItem { /* ... */ }
async fn create_test_epic(pool: &SqlitePool, project_id: Uuid, user_id: Uuid) -> WorkItem { /* ... */ }
async fn create_test_story_with_parent(pool: &SqlitePool, project_id: Uuid, parent_id: Uuid, user_id: Uuid) -> WorkItem { /* ... */ }
async fn update_item_version(pool: &SqlitePool, item_id: Uuid, version: i32) { /* ... */ }
```

---

## File Summary

| Action | Path |
|--------|------|
| Create | `pm-ws/src/handlers/subscription.rs` |
| Create | `pm-ws/src/handlers/work_item.rs` |
| Create | `pm-ws/src/handlers/query.rs` |
| Modify | `pm-ws/src/web_socket_connection.rs` |
| Create | `pm-ws/tests/work_item_handler_tests.rs` |

---

## Verification

```bash
cd backend

# Build workspace
cargo build --workspace

# Run all tests
cargo test --workspace

# Run handler tests specifically
cargo test -p pm-ws work_item_handler

# Run with logging to see handler execution
RUST_LOG=debug cargo test -p pm-ws work_item_handler -- --nocapture
```

---

## End-to-End Test Checklist

After completing Session 30.3, you should be able to:

1. [ ] Connect to WebSocket with valid JWT
2. [ ] Subscribe to a project
3. [ ] Create a work item (as editor)
4. [ ] Receive WorkItemCreated broadcast
5. [ ] Update a work item with correct version
6. [ ] Receive WorkItemUpdated broadcast with FieldChange list
7. [ ] Fail update with stale version (ConflictError)
8. [ ] Delete a work item without children
9. [ ] Fail delete when children exist (DeleteBlocked)
10. [ ] Query work items (full load)
11. [ ] Query work items (incremental sync)
12. [ ] Fail operations when unauthorized
13. [ ] Idempotent retry returns cached result
