# Session 30 Production-Grade Plan: Work Items via WebSocket

## Target: 9+/10 Production Grade

This plan addresses ALL production concerns identified through codebase analysis.

---

## Production Checklist

| Concern | Status | Solution |
|---------|--------|----------|
| Tenant isolation | ✅ EXISTS | Per-tenant DBs, JWT extraction |
| Rate limiting | ✅ EXISTS | Governor-based, per-connection |
| Metrics | ✅ EXISTS | Comprehensive in metrics.rs |
| Connection limits | ✅ EXISTS | Global + per-tenant |
| Graceful shutdown | ✅ EXISTS | ShutdownCoordinator |
| Input validation | ✅ EXISTS | message_validator.rs |
| Soft deletes | ✅ EXISTS | deleted_at pattern |
| Error location | ✅ EXISTS | error_location crate |
| Message ID | ✅ EXISTS | In WebSocketMessage proto |
| Field-level errors | ✅ EXISTS | Error.field in proto |
| **Authorization** | 🔧 ADD | Project membership check |
| **Optimistic locking** | 🔧 ADD | version column + conflict detection |
| **Transaction atomicity** | 🔧 ADD | SQLx transactions |
| **DB timeouts** | 🔧 ADD | Pool acquire + query timeouts |
| **Initial data load** | 🔧 ADD | GetWorkItemsRequest message |
| **Cascade delete safety** | 🔧 ADD | Reject if children exist |
| **Idempotency** | 🔧 ADD | Client-generated UUIDs + ON CONFLICT |
| **Request/response correlation** | 🔧 ADD | Echo message_id in responses |
| **Structured logging** | 🔧 ADD | Migrate to tracing with spans |
| **Handler tests** | 🔧 ADD | Integration tests for all handlers |
| **Reconnection sync** | 🔧 ADD | GetWorkItemsSince query |

---

## Phase 1: Schema Additions (Migration)

### 1.1 Add version column for optimistic locking

```sql
-- Migration: add_version_column
ALTER TABLE pm_work_items ADD COLUMN version INTEGER NOT NULL DEFAULT 0;
```

**Why**: Prevents silent data loss from concurrent updates. Client sends `expected_version`, server rejects if stale.

### 1.2 Add project_members table for authorization

```sql
-- Migration: add_project_members
CREATE TABLE pm_project_members (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    role TEXT NOT NULL CHECK(role IN ('viewer', 'editor', 'admin')),
    created_at INTEGER NOT NULL,
    FOREIGN KEY (project_id) REFERENCES pm_work_items(id),
    UNIQUE(project_id, user_id)
);

CREATE INDEX idx_pm_project_members_project ON pm_project_members(project_id);
CREATE INDEX idx_pm_project_members_user ON pm_project_members(user_id);
```

**Why**: Enforces who can view/edit which projects. Viewer can read, editor can CRUD, admin can manage members.

### 1.3 Add idempotency tracking table

```sql
-- Migration: add_idempotency_keys
CREATE TABLE pm_idempotency_keys (
    message_id TEXT PRIMARY KEY,
    operation TEXT NOT NULL,
    result_json TEXT NOT NULL,
    created_at INTEGER NOT NULL
);

-- Cleanup old entries (run periodically)
-- DELETE FROM pm_idempotency_keys WHERE created_at < (unixepoch() - 3600);
```

**Why**: Prevents duplicate creates on network retries. Store result, return cached on replay.

---

## Phase 2: Backend Infrastructure Updates

### 2.1 Database Timeout Configuration

**File**: `pm-db/src/connection/tenant_connection_manager.rs`

```rust
let pool = SqlitePoolOptions::new()
    .max_connections(5)
    .acquire_timeout(Duration::from_secs(5))  // ADD: Fail fast if pool exhausted
    .connect_with(options)
    .await?;
```

### 2.2 Structured Logging Setup

**File**: `pm-ws/src/lib.rs` (new function)

```rust
use tracing::{info_span, Instrument};

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

**Usage in handlers**:
```rust
async fn handle_create(request, ctx) {
    let span = create_request_span(&request.message_id, &ctx.tenant_id, &ctx.user_id, "create_work_item");
    async {
        // handler logic
    }.instrument(span).await
}
```

### 2.3 Extend WsError for field names

**File**: `pm-ws/src/error.rs`

```rust
#[derive(Debug, Error)]
pub enum WsError {
    // ... existing variants ...

    #[error("Validation failed: {message}")]
    ValidationError {
        message: String,
        field: Option<String>,  // ADD field name
        location: ErrorLocation,
    },

    #[error("Conflict: resource was modified (current version: {current_version})")]
    ConflictError {
        current_version: i32,
        location: ErrorLocation,
    },

    #[error("Cannot delete: {message}")]
    DeleteBlocked {
        message: String,
        location: ErrorLocation,
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
}
```

---

## Phase 3: Protobuf Message Additions

**File**: `proto/messages.proto`

### 3.1 Query messages for initial load

```protobuf
// Tag 33: Get work items for a project
message GetWorkItemsRequest {
  string project_id = 1;
  optional int64 since_timestamp = 2;  // For incremental sync
}

// Tag 43: Response with work items list
message WorkItemsList {
  repeated WorkItem work_items = 1;
  int64 as_of_timestamp = 2;  // For client to track sync point
}
```

### 3.2 Update WebSocketMessage oneof

```protobuf
oneof payload {
  // ... existing ...
  GetWorkItemsRequest get_work_items_request = 33;
  WorkItemsList work_items_list = 43;
}
```

### 3.3 Add version to UpdateWorkItemRequest

```protobuf
message UpdateWorkItemRequest {
  string work_item_id = 1;
  int32 expected_version = 2;  // ADD: For optimistic locking
  // ... existing optional fields ...
}
```

---

## Phase 4: Handler Module Structure

**Directory**: `backend/crates/pm-ws/src/handlers/`

```
handlers/
├── mod.rs                    # Module exports + dispatcher
├── context.rs                # Request context (tenant_id, user_id, pool)
├── error_codes.rs            # VALIDATION_ERROR, NOT_FOUND, CONFLICT, etc.
├── response_builder.rs       # Build protobuf responses with message_id
├── authorization.rs          # Project membership checks
├── hierarchy_validator.rs    # project→epic→story→task rules
├── change_tracker.rs         # Track field changes for FieldChange list
├── idempotency.rs            # Check/store idempotency keys
├── subscription.rs           # Subscribe/Unsubscribe handlers
├── work_item.rs              # CRUD + query handlers
└── query.rs                  # GetWorkItemsRequest handler
```

### 4.1 Handler Context

```rust
pub struct HandlerContext {
    pub message_id: String,
    pub tenant_id: String,
    pub user_id: Uuid,
    pub pool: SqlitePool,
}
```

### 4.2 Authorization Module

```rust
pub enum Permission {
    View,
    Edit,
    Admin,
}

pub async fn check_permission(
    ctx: &HandlerContext,
    project_id: Uuid,
    required: Permission,
) -> Result<(), WsError> {
    let repo = ProjectMemberRepository::new(&ctx.pool);
    let member = repo.find_by_user_and_project(ctx.user_id, project_id).await?;

    match member {
        None => Err(WsError::Unauthorized { ... }),
        Some(m) if !m.role.has_permission(required) => Err(WsError::Unauthorized { ... }),
        Some(_) => Ok(()),
    }
}
```

### 4.3 Idempotency Module

```rust
pub async fn check_idempotency(
    pool: &SqlitePool,
    message_id: &str,
) -> Result<Option<String>, WsError> {
    // Return cached result if exists
    let repo = IdempotencyRepository::new(pool);
    repo.find_by_message_id(message_id).await
}

pub async fn store_idempotency(
    pool: &SqlitePool,
    message_id: &str,
    operation: &str,
    result_json: &str,
) -> Result<(), WsError> {
    let repo = IdempotencyRepository::new(pool);
    repo.create(message_id, operation, result_json).await
}
```

---

## Phase 5: Work Item Handler Implementation

### 5.1 Create Handler (Full Production Pattern)

```rust
pub async fn handle_create(
    request: CreateWorkItemRequest,
    ctx: HandlerContext,
) -> Result<(WebSocketMessage, BroadcastInfo), WsError> {
    let span = create_request_span(&ctx.message_id, &ctx.tenant_id, &ctx.user_id.to_string(), "create_work_item");

    async {
        // 1. Check idempotency (return cached if replay)
        if let Some(cached) = check_idempotency(&ctx.pool, &ctx.message_id).await? {
            return Ok(deserialize_cached_response(cached));
        }

        // 2. Validate input
        validate_work_item_create(&request)?;

        // 3. Parse UUIDs
        let project_id = Uuid::parse_str(&request.project_id)
            .map_err(|_| WsError::ValidationError {
                message: "Invalid project_id".into(),
                field: Some("project_id".into()),
                location: ErrorLocation::caller(),
            })?;

        // 4. Check authorization (must be editor or admin)
        check_permission(&ctx, project_id, Permission::Edit).await?;

        // 5. Validate hierarchy (parent must allow this child type)
        if let Some(parent_id) = &request.parent_id {
            let parent_uuid = Uuid::parse_str(parent_id)?;
            validate_hierarchy(&ctx.pool, request.item_type, parent_uuid).await?;
        }

        // 6. Calculate next position
        let position = calculate_next_position(&ctx.pool, project_id, request.parent_id.as_deref()).await?;

        // 7. Create domain model
        let work_item = WorkItem::new(
            request.item_type.into(),
            request.title,
            request.description,
            request.parent_id.map(|s| Uuid::parse_str(&s).unwrap()),
            project_id,
            ctx.user_id,
        );
        work_item.position = position;

        // 8. Execute in transaction
        let mut tx = ctx.pool.begin().await?;

        let work_item_repo = WorkItemRepository::new_with_tx(&mut tx);
        work_item_repo.create(&work_item).await?;

        let activity_repo = ActivityLogRepository::new_with_tx(&mut tx);
        activity_repo.create(&ActivityLog::created("WorkItem", work_item.id, ctx.user_id)).await?;

        tx.commit().await?;

        // 9. Build response
        let response = build_work_item_created_response(&ctx.message_id, &work_item, ctx.user_id);

        // 10. Store idempotency key
        store_idempotency(&ctx.pool, &ctx.message_id, "create_work_item", &serialize_response(&response)).await?;

        // 11. Return response + broadcast info
        Ok((response, BroadcastInfo::new(project_id, "work_item_created")))
    }.instrument(span).await
}
```

### 5.2 Update Handler (with Optimistic Locking)

```rust
pub async fn handle_update(
    request: UpdateWorkItemRequest,
    ctx: HandlerContext,
) -> Result<(WebSocketMessage, BroadcastInfo), WsError> {
    // ... span setup ...

    // 1. Fetch existing item
    let work_item_id = Uuid::parse_str(&request.work_item_id)?;
    let repo = WorkItemRepository::new(&ctx.pool);
    let mut work_item = repo.find_by_id(work_item_id).await?
        .ok_or(WsError::NotFound { ... })?;

    // 2. Check authorization
    check_permission(&ctx, work_item.project_id, Permission::Edit).await?;

    // 3. Optimistic lock check
    if work_item.version != request.expected_version {
        return Err(WsError::ConflictError {
            current_version: work_item.version,
            location: ErrorLocation::caller(),
        });
    }

    // 4. Track changes for FieldChange list
    let changes = track_changes(&work_item, &request);

    // 5. Apply changes
    apply_updates(&mut work_item, &request);
    work_item.version += 1;
    work_item.updated_at = Utc::now();
    work_item.updated_by = ctx.user_id;

    // 6. Transaction: update + activity log
    let mut tx = ctx.pool.begin().await?;
    // ... same pattern as create ...

    // 7. Build response with FieldChange list
    let response = build_work_item_updated_response(&ctx.message_id, &work_item, &changes, ctx.user_id);

    Ok((response, BroadcastInfo::new(work_item.project_id, "work_item_updated")))
}
```

### 5.3 Delete Handler (with Cascade Check)

```rust
pub async fn handle_delete(
    request: DeleteWorkItemRequest,
    ctx: HandlerContext,
) -> Result<(WebSocketMessage, BroadcastInfo), WsError> {
    // ... setup ...

    // 1. Fetch item
    let work_item = repo.find_by_id(work_item_id).await?
        .ok_or(WsError::NotFound { ... })?;

    // 2. Check authorization
    check_permission(&ctx, work_item.project_id, Permission::Edit).await?;

    // 3. Check for children (reject if any exist)
    let children = repo.find_children(work_item_id).await?;
    if !children.is_empty() {
        return Err(WsError::DeleteBlocked {
            message: format!("Cannot delete: {} child items exist. Delete children first.", children.len()),
            location: ErrorLocation::caller(),
        });
    }

    // 4. Soft delete in transaction
    // ... transaction pattern ...

    Ok((response, BroadcastInfo::new(work_item.project_id, "work_item_deleted")))
}
```

### 5.4 Query Handler (Initial Load + Sync)

```rust
pub async fn handle_get_work_items(
    request: GetWorkItemsRequest,
    ctx: HandlerContext,
) -> Result<WebSocketMessage, WsError> {
    let project_id = Uuid::parse_str(&request.project_id)?;

    // Check view permission (not edit)
    check_permission(&ctx, project_id, Permission::View).await?;

    let repo = WorkItemRepository::new(&ctx.pool);
    let work_items = match request.since_timestamp {
        Some(ts) => repo.find_by_project_since(project_id, ts).await?,
        None => repo.find_by_project(project_id).await?,
    };

    let response = build_work_items_list_response(
        &ctx.message_id,
        work_items,
        Utc::now().timestamp(),
    );

    Ok(response)
}
```

---

## Phase 6: Message Dispatcher Integration

**File**: `pm-ws/src/web_socket_connection.rs` (lines 260-280)

```rust
async fn handle_binary_message(&mut self, data: &[u8]) -> Result<(), WsError> {
    // Decode with timeout
    let message = tokio::time::timeout(
        Duration::from_secs(1),
        async { WebSocketMessage::decode(data) }
    ).await
        .map_err(|_| WsError::Internal { message: "Decode timeout".into(), ... })?
        .map_err(|e| WsError::ProtoDecode { source: e, ... })?;

    // Create handler context
    let ctx = HandlerContext {
        message_id: message.message_id.clone(),
        tenant_id: self.tenant_context.tenant_id.clone(),
        user_id: self.tenant_context.user_id,
        pool: self.connection_manager.get_pool(&self.tenant_context.tenant_id).await?,
    };

    // Dispatch based on payload
    let result = match message.payload {
        Some(Payload::CreateWorkItemRequest(req)) => {
            handlers::work_item::handle_create(req, ctx).await
        }
        Some(Payload::UpdateWorkItemRequest(req)) => {
            handlers::work_item::handle_update(req, ctx).await
        }
        Some(Payload::DeleteWorkItemRequest(req)) => {
            handlers::work_item::handle_delete(req, ctx).await
        }
        Some(Payload::GetWorkItemsRequest(req)) => {
            let response = handlers::query::handle_get_work_items(req, ctx).await?;
            self.send_response(response).await?;
            return Ok(());
        }
        Some(Payload::Subscribe(req)) => {
            handlers::subscription::handle_subscribe(req, &mut self.subscriptions).await
        }
        Some(Payload::Unsubscribe(req)) => {
            handlers::subscription::handle_unsubscribe(req, &mut self.subscriptions).await
        }
        _ => {
            return Err(WsError::InvalidMessage { message: "Unknown payload type".into(), ... });
        }
    };

    match result {
        Ok((response, broadcast_info)) => {
            // Send response to requester
            self.send_response(response.clone()).await?;

            // Broadcast event to other subscribers
            self.broadcaster.broadcast(&self.tenant_context.tenant_id, BroadcastMessage {
                payload: response.encode_to_vec().into(),
                event_type: broadcast_info.event_type,
                project_id: Some(broadcast_info.project_id.to_string()),
            }).await?;
        }
        Err(e) => {
            // Send error response
            let error_response = build_error_response(&message.message_id, e);
            self.send_response(error_response).await?;
        }
    }

    Ok(())
}
```

---

## Phase 7: Frontend Implementation

### 7.1 WebSocket Client with Correlation

```csharp
public class ProjectManagementWebSocketClient : IAsyncDisposable
{
    private readonly ConcurrentDictionary<string, TaskCompletionSource<WebSocketMessage>> _pendingRequests = new();

    public async Task<TResponse> SendRequestAsync<TResponse>(
        WebSocketMessage request,
        CancellationToken ct = default)
    {
        var tcs = new TaskCompletionSource<WebSocketMessage>();
        _pendingRequests[request.MessageId] = tcs;

        try
        {
            await _outgoing.Writer.WriteAsync(request, ct);

            using var cts = CancellationTokenSource.CreateLinkedTokenSource(ct);
            cts.CancelAfter(TimeSpan.FromSeconds(30)); // Request timeout

            var response = await tcs.Task.WaitAsync(cts.Token);
            return ExtractResponse<TResponse>(response);
        }
        finally
        {
            _pendingRequests.TryRemove(request.MessageId, out _);
        }
    }

    private async Task ReceiveLoop(CancellationToken ct)
    {
        while (!ct.IsCancellationRequested)
        {
            var message = await ReceiveMessage(ct);

            // Check if this is a response to a pending request
            if (_pendingRequests.TryGetValue(message.MessageId, out var tcs))
            {
                tcs.TrySetResult(message);
            }
            else
            {
                // Broadcast event - route to state manager
                await _incoming.Writer.WriteAsync(message, ct);
            }
        }
    }
}
```

### 7.2 State with Version Tracking

```csharp
public class WorkItemState
{
    private readonly ConcurrentDictionary<Guid, WorkItem> _items = new();
    private long _lastSyncTimestamp;

    public event Action? OnChange;

    public void Upsert(WorkItem item)
    {
        _items.AddOrUpdate(item.Id, item, (_, existing) =>
        {
            // Only update if newer version
            return item.Version >= existing.Version ? item : existing;
        });
        OnChange?.Invoke();
    }

    public long GetSyncTimestamp() => _lastSyncTimestamp;

    public void SetSyncTimestamp(long timestamp) => _lastSyncTimestamp = timestamp;
}
```

### 7.3 Reconnection with Sync

```csharp
public class ProjectStateManager
{
    private async Task ReconnectAsync()
    {
        while (true)
        {
            try
            {
                await _client.ConnectAsync(_serverUri, _jwtToken);

                // Resubscribe to project
                await _client.SubscribeAsync(_currentProjectId);

                // Fetch items since last sync
                var syncRequest = new GetWorkItemsRequest
                {
                    ProjectId = _currentProjectId.ToString(),
                    SinceTimestamp = _state.GetSyncTimestamp(),
                };

                var response = await _client.SendRequestAsync<WorkItemsList>(
                    WrapInMessage(syncRequest));

                foreach (var item in response.WorkItems)
                {
                    _state.Upsert(ProtoMapper.FromProto(item));
                }
                _state.SetSyncTimestamp(response.AsOfTimestamp);

                break; // Reconnected successfully
            }
            catch
            {
                await Task.Delay(_backoff.GetNextDelay());
            }
        }
    }
}
```

---

## Phase 8: Handler Tests

### 8.1 Work Item Handler Tests

**File**: `pm-ws/tests/work_item_handler_tests.rs`

```rust
#[tokio::test]
async fn test_create_work_item_success() {
    let (pool, ctx) = setup_test_context().await;

    // Setup: Create project and add user as editor
    let project = create_test_project(&pool).await;
    add_project_member(&pool, project.id, ctx.user_id, "editor").await;

    let request = CreateWorkItemRequest {
        item_type: WorkItemType::Story as i32,
        title: "Test Story".into(),
        project_id: project.id.to_string(),
        ..Default::default()
    };

    let result = handle_create(request, ctx).await;

    assert!(result.is_ok());
    let (response, broadcast) = result.unwrap();
    assert_eq!(broadcast.event_type, "work_item_created");
}

#[tokio::test]
async fn test_create_work_item_unauthorized() {
    let (pool, ctx) = setup_test_context().await;
    let project = create_test_project(&pool).await;
    // Note: user NOT added as member

    let request = CreateWorkItemRequest { ... };
    let result = handle_create(request, ctx).await;

    assert!(matches!(result, Err(WsError::Unauthorized { .. })));
}

#[tokio::test]
async fn test_update_work_item_conflict() {
    let (pool, ctx) = setup_test_context().await;
    // ... setup ...

    let request = UpdateWorkItemRequest {
        work_item_id: item.id.to_string(),
        expected_version: 0, // Stale version
        title: Some("Updated".into()),
        ..Default::default()
    };

    // Update item directly to version 1
    update_item_version(&pool, item.id, 1).await;

    let result = handle_update(request, ctx).await;

    assert!(matches!(result, Err(WsError::ConflictError { current_version: 1, .. })));
}

#[tokio::test]
async fn test_delete_blocked_by_children() {
    let (pool, ctx) = setup_test_context().await;
    let parent = create_test_work_item(&pool, "parent").await;
    let _child = create_test_work_item_with_parent(&pool, "child", parent.id).await;

    let request = DeleteWorkItemRequest { work_item_id: parent.id.to_string() };
    let result = handle_delete(request, ctx).await;

    assert!(matches!(result, Err(WsError::DeleteBlocked { .. })));
}

#[tokio::test]
async fn test_create_idempotency() {
    let (pool, ctx) = setup_test_context().await;
    // ... setup ...

    let message_id = "unique-msg-123";
    let request = CreateWorkItemRequest { ... };

    // First call - creates item
    let result1 = handle_create(request.clone(), ctx_with_msg_id(message_id)).await;
    assert!(result1.is_ok());

    // Second call with same message_id - returns cached
    let result2 = handle_create(request, ctx_with_msg_id(message_id)).await;
    assert!(result2.is_ok());

    // Verify only one item created
    let count = count_work_items(&pool).await;
    assert_eq!(count, 1);
}
```

---

## File Summary

### Backend Files (17 total)

| Action | Path | Purpose |
|--------|------|---------|
| Create | `pm-db/migrations/003_add_version_column.sql` | Optimistic locking |
| Create | `pm-db/migrations/004_add_project_members.sql` | Authorization |
| Create | `pm-db/migrations/005_add_idempotency_keys.sql` | Deduplication |
| Create | `pm-db/src/repositories/project_member_repository.rs` | Membership queries |
| Create | `pm-db/src/repositories/idempotency_repository.rs` | Idempotency storage |
| Create | `pm-ws/src/handlers/mod.rs` | Module exports |
| Create | `pm-ws/src/handlers/context.rs` | Handler context |
| Create | `pm-ws/src/handlers/error_codes.rs` | Error constants |
| Create | `pm-ws/src/handlers/response_builder.rs` | Response construction |
| Create | `pm-ws/src/handlers/authorization.rs` | Permission checks |
| Create | `pm-ws/src/handlers/hierarchy_validator.rs` | Hierarchy rules |
| Create | `pm-ws/src/handlers/change_tracker.rs` | Field change tracking |
| Create | `pm-ws/src/handlers/idempotency.rs` | Dedup logic |
| Create | `pm-ws/src/handlers/subscription.rs` | Subscribe handlers |
| Create | `pm-ws/src/handlers/work_item.rs` | CRUD handlers |
| Create | `pm-ws/src/handlers/query.rs` | Query handlers |
| Create | `pm-ws/tests/work_item_handler_tests.rs` | Handler tests |
| Modify | `pm-ws/src/lib.rs` | Add handlers module |
| Modify | `pm-ws/src/web_socket_connection.rs` | Message dispatch |
| Modify | `pm-ws/src/error.rs` | Add error variants |
| Modify | `pm-db/src/connection/tenant_connection_manager.rs` | Add timeout |
| Modify | `proto/messages.proto` | Add query messages |

### Frontend Files (20 total)

| Action | Path |
|--------|------|
| Create | `frontend/ProjectManagement.sln` |
| Create | `frontend/ProjectManagement.Core/ProjectManagement.Core.csproj` |
| Create | `frontend/ProjectManagement.Core/Models/WorkItem.cs` |
| Create | `frontend/ProjectManagement.Core/Enums/WorkItemType.cs` |
| Create | `frontend/ProjectManagement.Core/Proto/` (generated) |
| Create | `frontend/ProjectManagement.Core/Mapping/ProtoMapper.cs` |
| Create | `frontend/ProjectManagement.Services/ProjectManagement.Services.csproj` |
| Create | `frontend/ProjectManagement.Services/WebSocket/ProjectManagementWebSocketClient.cs` |
| Create | `frontend/ProjectManagement.Services/State/WorkItemState.cs` |
| Create | `frontend/ProjectManagement.Services/State/ProjectStateManager.cs` |
| Create | `frontend/ProjectManagement.Services/Commands/WorkItemCommands.cs` |
| Create | `frontend/ProjectManagement.Components/ProjectManagement.Components.csproj` |
| Create | `frontend/ProjectManagement.Components/_Imports.razor` |
| Create | `frontend/ProjectManagement.Components/Pages/ProjectDashboard.razor` |
| Create | `frontend/ProjectManagement.Components/Components/KanbanBoard.razor` |
| Create | `frontend/ProjectManagement.Components/Components/WorkItemCard.razor` |
| Create | `frontend/ProjectManagement.Components/Components/WorkItemList.razor` |
| Create | `frontend/ProjectManagement.Components/Dialogs/CreateWorkItemDialog.razor` |
| Create | `frontend/ProjectManagement.Wasm/ProjectManagement.Wasm.csproj` |
| Create | `frontend/ProjectManagement.Wasm/wwwroot/index.html` |
| Create | `frontend/ProjectManagement.Wasm/Program.cs` |

---

## Verification

### Backend
```bash
cd backend
cargo build --workspace
cargo test --workspace
```

### Frontend
```bash
cd frontend
dotnet build ProjectManagement.sln
dotnet run --project ProjectManagement.Wasm
```

### End-to-End Tests
1. Start backend with test database
2. Create project via SQL
3. Add user as project member
4. Connect WebSocket with JWT
5. Subscribe to project
6. Receive initial WorkItemsList
7. Create work item via CreateWorkItemRequest
8. Verify WorkItemCreated broadcast received
9. Open second client, verify sync
10. Test concurrent update conflict detection
11. Test unauthorized access rejection
12. Test delete with children rejection
13. Test idempotent retry behavior

---

## Production Grade Checklist: 9/10

| Requirement | Status |
|-------------|--------|
| Tenant isolation | ✅ |
| Authorization (project-level) | ✅ |
| Input validation | ✅ |
| Optimistic locking | ✅ |
| Transaction atomicity | ✅ |
| Idempotency | ✅ |
| Request/response correlation | ✅ |
| Structured logging | ✅ |
| DB timeouts | ✅ |
| Rate limiting | ✅ |
| Connection limits | ✅ |
| Graceful shutdown | ✅ |
| Error handling (field-level) | ✅ |
| Soft deletes | ✅ |
| Audit trail | ✅ |
| Cascade safety | ✅ |
| Initial data load | ✅ |
| Reconnection sync | ✅ |
| Handler tests | ✅ |
| Metrics | ✅ |

**Missing for 10/10** (future sessions):
- Circuit breaker for DB overload
- Distributed tracing correlation
- Load testing validation
- Chaos engineering verification
