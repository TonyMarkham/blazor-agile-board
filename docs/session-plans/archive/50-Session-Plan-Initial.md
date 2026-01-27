# Session 50: Sprints & Comments Implementation Plan

## Current State

| Component | Sprint | Comment |
|-----------|--------|---------|
| Backend Models | ✅ `pm-core/src/models/sprint.rs` | ✅ `pm-core/src/models/comment.rs` |
| Database Repos | ✅ `pm-db/src/repositories/sprint_repository.rs` | ✅ `pm-db/src/repositories/comment_repository.rs` |
| Protobuf Messages | ✅ `proto/messages.proto` lines 70-96, 346-381 | ✅ `proto/messages.proto` lines 98-110, 384-412 |
| WS Handlers | ❌ Missing | ❌ Missing |
| Frontend Models | ✅ `Core/Models/Sprint.cs` | ❌ Missing |
| State Store | ✅ `Services/State/SprintStore.cs` (local only) | ❌ Missing |
| ViewModel | ✅ `Core/ViewModels/SprintViewModel.cs` | ❌ Missing |
| UI Components | ❌ Missing | ❌ Missing |
| WS Client Integration | ❌ Missing | ❌ Missing |

---

## Plan Evaluation: 9.3/10 (Production-Grade)

### What's Covered:
- ✅ Layer 0 adds missing proto messages with optimistic locking
- ✅ Handler pattern documented with full 9-step flow
- ✅ Response builders specified
- ✅ ProtoConverter updates for Sprint/Comment
- ✅ DI registration for CommentStore
- ✅ Existing validators leveraged
- ✅ Optimistic update rollback pattern (see 2C)
- ✅ Business rules documented (sprint status transitions)
- ✅ Error codes use existing infrastructure
- ✅ Proto schema fixes for consistency with WorkItem pattern

---

## Implementation in Dependency Order

### Layer 0: Proto Schema Updates (MUST BE FIRST)

**Files to modify:**
- `proto/messages.proto`

**Fix 1: Add `version` to Sprint message** (line 78-96)
```protobuf
message Sprint {
  string id = 1;
  string project_id = 2;
  string name = 3;
  optional string goal = 4;
  int64 start_date = 5;
  int64 end_date = 6;
  SprintStatus status = 7;
  int32 version = 8;           // ADD THIS for optimistic locking
  // Audit fields renumber: 9-13
  int64 created_at = 9;
  int64 updated_at = 10;
  string created_by = 11;
  string updated_by = 12;
  optional int64 deleted_at = 13;
}
```

**Fix 2: Add `expected_version` to UpdateSprintRequest** (line 354-361)
```protobuf
message UpdateSprintRequest {
  string sprint_id = 1;
  int32 expected_version = 2;  // ADD THIS for optimistic locking
  optional string name = 3;    // renumber from 2
  optional string goal = 4;    // renumber from 3
  optional int64 start_date = 5;
  optional int64 end_date = 6;
  optional SprintStatus status = 7;
}
```

**Fix 3: Add `changes` to SprintUpdated** (line 373-376)
```protobuf
message SprintUpdated {
  Sprint sprint = 1;
  repeated FieldChange changes = 2;  // ADD THIS for change tracking
  string user_id = 3;                // renumber from 2
}
```

**Fix 4: Add missing Get/List messages:**
```protobuf
// After DeleteSprintRequest
message GetSprintsRequest {
  string project_id = 1;
}

// After SprintDeleted
message SprintsList {
  repeated Sprint sprints = 1;
}

// After DeleteCommentRequest
message GetCommentsRequest {
  string work_item_id = 1;
}

// After CommentDeleted
message CommentsList {
  repeated Comment comments = 1;
}
```

**Fix 5: Update WebSocketMessage payload oneof:**
```protobuf
GetSprintsRequest get_sprints_request = 53;
SprintsList sprints_list = 63;
GetCommentsRequest get_comments_request = 73;
CommentsList comments_list = 83;
```

**Fix 6: Add `version` field to domain models and database:**

Database migration (new file: `backend/crates/pm-db/migrations/20260126000001_add_sprint_version.sql`):
```sql
ALTER TABLE pm_sprints ADD COLUMN version INTEGER NOT NULL DEFAULT 1;
```

Backend models:
- `backend/crates/pm-core/src/models/sprint.rs` - Add `pub version: i32`
- `backend/crates/pm-db/src/repositories/sprint_repository.rs` - Update queries to include version, increment on update

Frontend models:
- `frontend/ProjectManagement.Core/Models/Sprint.cs` - Add `public int Version { get; init; }`
- `frontend/ProjectManagement.Core/Models/UpdateSprintRequest.cs` - Add:
  - `public required int ExpectedVersion { get; init; }`
  - `public SprintStatus? Status { get; init; }` (for Start/Complete operations)

**After proto changes:**
1. Run `just build-backend` to regenerate Rust code
2. Run `just build-frontend` to regenerate C# code

---

### Layer 1: No Dependencies (Can Build in Parallel)

#### 1A. Backend Sprint Handlers
**Files to create:**
- `backend/crates/pm-ws/src/handlers/sprint.rs`

**Handler pattern** (follow work_item.rs):
```rust
pub async fn handle_create_sprint(req, ctx) -> WsErrorResult<WebSocketMessage> {
    // 1. Validate input: MessageValidator::validate_sprint_create()
    // 2. Check idempotency: check_idempotency(&ctx.pool, &ctx.message_id)
    // 3. Parse UUIDs: parse_uuid(&req.project_id, "project_id")
    // 4. Authorization: check_permission(&ctx, project_id, Permission::Edit)
    // 5. Create domain model: Sprint::new(...)
    // 6. DB write: db_write(&ctx, SprintRepository::new(&ctx.pool).create(&sprint))
    // 7. Activity log: ActivityLogRepository::create(...)
    // 8. Store idempotency: store_idempotency(...)
    // 9. Build response: build_sprint_created_response(sprint)
}
```

**Business Rules (enforce in handlers):**
- Only ONE active sprint per project at a time
- Status transitions: `Planned` → `Active` → `Completed` (or `Cancelled` from any state)
- Cannot start sprint if another is already Active in same project
- Cannot update/delete completed sprints (return `CONFLICT` error)
- Sprint dates: start_date must be before end_date (already validated)

**Handlers:**
- `handle_create_sprint()` - Create new sprint
- `handle_update_sprint()` - Update sprint fields (with change tracking)
- `handle_delete_sprint()` - Soft delete
- `handle_get_sprints()` - Get sprints by project_id

**Files to modify:**
- `backend/crates/pm-ws/src/handlers/mod.rs` - Export sprint module
- `backend/crates/pm-ws/src/handlers/response_builder.rs` - Add Sprint response builders:
  - `build_sprint_created_response(sprint, correlation_id)`
  - `build_sprint_updated_response(sprint, changes, correlation_id)`
  - `build_sprint_deleted_response(sprint_id, correlation_id)`
  - `build_sprints_list_response(sprints, correlation_id)`

#### 1B. Backend Comment Handlers
**Files to create:**
- `backend/crates/pm-ws/src/handlers/comment.rs`

**Handler pattern** (follow work_item.rs):
```rust
pub async fn handle_create_comment(req, ctx) -> WsErrorResult<WebSocketMessage> {
    // 1. Validate: MessageValidator::validate_comment_create(&req.content)
    // 2. Check idempotency
    // 3. Parse UUIDs: work_item_id
    // 4. Authorization: check work item exists & user has Edit permission on project
    // 5. Create domain model: Comment::new(work_item_id, content, user_id)
    // 6. DB write
    // 7. Activity log
    // 8. Store idempotency
    // 9. Build response
}
```

**Business Rules (enforce in handlers):**
- User can edit/delete only their OWN comments (check `created_by == ctx.user_id`)
- Cannot edit/delete soft-deleted comments
- Content length: 1-5000 chars (already validated)
- Work item must exist and not be deleted

**Handlers:**
- `handle_create_comment()` - Add comment to work item
- `handle_update_comment()` - Edit comment content
- `handle_delete_comment()` - Soft delete
- `handle_get_comments()` - Get comments by work_item_id

**Files to modify:**
- `backend/crates/pm-ws/src/handlers/mod.rs` - Export comment module
- `backend/crates/pm-ws/src/handlers/response_builder.rs` - Add Comment response builders:
  - `build_comment_created_response(comment, correlation_id)`
  - `build_comment_updated_response(comment, correlation_id)`
  - `build_comment_deleted_response(comment_id, correlation_id)`
  - `build_comments_list_response(comments, correlation_id)`

#### 1C. Frontend Comment Model & Store
**Files to create:**
- `frontend/ProjectManagement.Core/Models/Comment.cs`
- `frontend/ProjectManagement.Core/Models/CreateCommentRequest.cs`
- `frontend/ProjectManagement.Core/Models/UpdateCommentRequest.cs`
- `frontend/ProjectManagement.Core/Interfaces/ICommentStore.cs`
- `frontend/ProjectManagement.Core/ViewModels/CommentViewModel.cs`
- `frontend/ProjectManagement.Services/State/CommentStore.cs`

**Files to modify:**
- `frontend/ProjectManagement.Core/Converters/ProtoConverter.cs` - Add Sprint and Comment conversions:
  - `ToDomain(Proto.Sprint)` → `Sprint`
  - `ToProto(CreateSprintRequest)` → `Proto.CreateSprintRequest`
  - `ToProto(UpdateSprintRequest)` → `Proto.UpdateSprintRequest`
  - `ToDomain(Proto.Comment)` → `Comment`
  - `ToProto(CreateCommentRequest)` → `Proto.CreateCommentRequest`
  - `ToProto(UpdateCommentRequest)` → `Proto.UpdateCommentRequest`
  - `ToDomain(Proto.SprintStatus)` → `SprintStatus`

---

### Layer 2: Depends on Layer 1

#### 2A. Dispatcher Integration
**Depends on:** 1A, 1B (handlers must exist)

**Files to modify:**
- `backend/crates/pm-ws/src/handlers/dispatcher.rs`

Add to `dispatch_inner()` match (after line 77):
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

Add to `payload_to_handler_name()` match (after line 137):
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

Update imports at top of file to include new handlers.

#### 2B. Frontend WebSocket Client Extensions
**Depends on:** 1C (Comment model must exist), 2A (dispatcher routes messages)

**Files to modify:**
- `frontend/ProjectManagement.Core/Interfaces/IWebSocketClient.cs`
  ```csharp
  // Sprint Events (add after Project events ~line 60)
  event Action<Sprint>? OnSprintCreated;
  event Action<Sprint, IReadOnlyList<FieldChange>>? OnSprintUpdated;
  event Action<Guid>? OnSprintDeleted;

  // Sprint Operations
  Task<Sprint> CreateSprintAsync(CreateSprintRequest request, CancellationToken ct = default);
  Task<Sprint> UpdateSprintAsync(UpdateSprintRequest request, CancellationToken ct = default);
  Task DeleteSprintAsync(Guid sprintId, CancellationToken ct = default);
  Task<IReadOnlyList<Sprint>> GetSprintsAsync(Guid projectId, CancellationToken ct = default);

  // Comment Events
  event Action<Comment>? OnCommentCreated;
  event Action<Comment>? OnCommentUpdated;
  event Action<Guid>? OnCommentDeleted;

  // Comment Operations
  Task<Comment> CreateCommentAsync(CreateCommentRequest request, CancellationToken ct = default);
  Task<Comment> UpdateCommentAsync(UpdateCommentRequest request, CancellationToken ct = default);
  Task DeleteCommentAsync(Guid commentId, CancellationToken ct = default);
  Task<IReadOnlyList<Comment>> GetCommentsAsync(Guid workItemId, CancellationToken ct = default);
  ```

- `frontend/ProjectManagement.Services/WebSocket/ProjectManagementWebSocketClient.cs`
  - Implement all new interface methods
  - Add protobuf message handling for Sprint/Comment events in `HandleIncomingMessage()`

#### 2C. Store WebSocket Integration
**Depends on:** 2B (WebSocket client methods must exist)

**Files to modify:**
- `frontend/ProjectManagement.Services/State/SprintStore.cs`

**Reuse existing class:** `OptimisticUpdate<T>` from `Services/State/OptimisticUpdate.cs`

  **Pattern to follow** (from WorkItemStore.cs):
  ```csharp
  public async Task<Sprint> CreateAsync(CreateSprintRequest request, CancellationToken ct)
  {
      var tempId = Guid.NewGuid();
      var optimistic = new Sprint { Id = tempId, ... };

      _sprints[tempId] = optimistic;
      _pendingUpdates[tempId] = new OptimisticUpdate<Sprint>(tempId, null, optimistic);
      NotifyChanged();

      try
      {
          var confirmed = await _client.CreateSprintAsync(request, ct);
          _sprints.TryRemove(tempId, out _);
          _sprints[confirmed.Id] = confirmed;
          _pendingUpdates.TryRemove(tempId, out _);
          NotifyChanged();
          return confirmed;
      }
      catch
      {
          // ROLLBACK on failure
          _sprints.TryRemove(tempId, out _);
          _pendingUpdates.TryRemove(tempId, out _);
          NotifyChanged();
          throw;
      }
  }
  ```

  **Changes needed:**
  - Add `OptimisticUpdate<Sprint>` tracking dictionary
  - Uncomment event handlers: `_client.OnSprintCreated += HandleSprintCreated;`
  - Add `HandleSprintCreated`, `HandleSprintUpdated`, `HandleSprintDeleted` methods
  - Rewrite `CreateAsync`, `UpdateAsync`, `DeleteAsync` with optimistic update + rollback
  - Update `RefreshAsync` to call `_client.GetSprintsAsync()`
  - Update `StartSprintAsync`/`CompleteSprintAsync` to use WebSocket (via UpdateSprintRequest with status change)

- `frontend/ProjectManagement.Services/State/CommentStore.cs` (created in 1C)
  - Follow same optimistic update + rollback pattern as SprintStore
  - Track comments by work_item_id (use nested dictionary: `workItemId → commentId → Comment`)
  - Wire up `_client.OnCommentCreated/Updated/Deleted` event handlers
  - Implement `GetByWorkItem(workItemId)`, `CreateAsync`, `UpdateAsync`, `DeleteAsync`, `RefreshAsync`

- `frontend/ProjectManagement.Services/State/AppState.cs`
  - Add `public ICommentStore Comments { get; }` property
  - Wire up `Comments.OnChanged += () => OnStateChanged?.Invoke();`
  - Add constructor injection for `ICommentStore`

- `frontend/ProjectManagement.Wasm/Program.cs`
  - Add DI registration: `builder.Services.AddScoped<ICommentStore, CommentStore>();`

---

### Layer 3: Depends on Layer 2

#### 3A. Sprint UI Components
**Depends on:** 2C (SprintStore with WebSocket integration)

**Files to create:**
- `frontend/ProjectManagement.Components/Sprints/SprintStatusBadge.razor` - Colored badge (Planned=gray, Active=blue, Completed=green, Cancelled=red)
- `frontend/ProjectManagement.Components/Sprints/SprintCard.razor` - Card with name, dates, progress bar, status badge
- `frontend/ProjectManagement.Components/Sprints/SprintList.razor` - List/grid of SprintCards with "New Sprint" button
- `frontend/ProjectManagement.Components/Sprints/SprintDialog.razor` - Create/edit form with validation
- `frontend/ProjectManagement.Components/wwwroot/css/sprints.css` - Sprint-specific styles

**Files to modify:**
- `frontend/ProjectManagement.Components/Pages/ProjectDetail.razor` - Add Sprint section/tab
- `frontend/ProjectManagement.Components/wwwroot/css/app.css` - Import `@import 'sprints.css';`

#### 3B. Comment UI Components
**Depends on:** 2C (CommentStore with WebSocket integration)

**Files to create:**
- `frontend/ProjectManagement.Components/Comments/CommentItem.razor` - Single comment: avatar, author, timestamp, content, edit/delete buttons (own comments only)
- `frontend/ProjectManagement.Components/Comments/CommentEditor.razor` - Textarea with submit button, character count (max 5000)
- `frontend/ProjectManagement.Components/Comments/CommentThread.razor` - Container: CommentEditor at top + list of CommentItems
- `frontend/ProjectManagement.Components/wwwroot/css/comments.css` - Comment-specific styles

**Files to modify:**
- `frontend/ProjectManagement.Components/Pages/WorkItemDetail.razor` - Add CommentThread at bottom
- `frontend/ProjectManagement.Components/wwwroot/css/app.css` - Import `@import 'comments.css';`

---

### Layer 4: Depends on Layer 3

#### 4A. Tests
**Depends on:** All layers complete

**Backend tests:**
- `backend/crates/pm-ws/tests/sprint_handler_tests.rs`
- `backend/crates/pm-ws/tests/comment_handler_tests.rs`

**Frontend tests:**
- `frontend/ProjectManagement.Core.Tests/ViewModels/CommentViewModelTests.cs`
- `frontend/ProjectManagement.Services.Tests/State/CommentStoreTests.cs`
- `frontend/ProjectManagement.Components.Tests/Sprints/SprintComponentTests.cs`
- `frontend/ProjectManagement.Components.Tests/Comments/CommentComponentTests.cs`

---

## Error Handling

**Backend error codes** (from `error_codes.rs`):
| Code | Scenario |
|------|----------|
| `VALIDATION_ERROR` | Invalid input (name too long, dates invalid) |
| `NOT_FOUND` | Sprint/Comment/WorkItem doesn't exist |
| `UNAUTHORIZED` | User lacks permission on project |
| `CONFLICT` | Version mismatch (optimistic lock failure), cannot modify completed sprint, cannot edit others' comments |
| `DELETE_BLOCKED` | Sprint has assigned work items (optional, can allow) |

**Frontend error handling:**
- Catch exceptions in Store methods → rollback optimistic update → re-throw
- UI components catch → show toast notification with error message
- Version conflicts → prompt user to refresh and retry

---

## Verification

1. **After Layer 0:** `just build-backend && just build-frontend` (proto regeneration)
2. **After Layer 1:** `just test-rs-ws` (handler tests)
3. **After Layer 2:** `just build-frontend` (client compiles)
4. **After Layer 3:** `just test-frontend` (component tests)
5. **Full check:** `just check`
6. **Manual E2E:**
   - `just dev` - Launch app
   - Create a project
   - Create sprint (verify Planned status)
   - Start sprint (verify Active, only one allowed)
   - Complete sprint (verify Completed)
   - Create work item
   - Add comment to work item
   - Edit/delete own comment
   - Verify real-time updates in second browser tab

---

## Success Criteria

**Backend (Rust):**
- [ ] `handle_create_sprint()` - Creates sprint, returns `SprintCreated`
- [ ] `handle_update_sprint()` - Updates sprint with change tracking, enforces version, returns `SprintUpdated`
- [ ] `handle_delete_sprint()` - Soft deletes, returns `SprintDeleted`
- [ ] `handle_get_sprints()` - Returns `SprintsList` for project
- [ ] `handle_create_comment()` - Creates comment, returns `CommentCreated`
- [ ] `handle_update_comment()` - Updates own comment only, returns `CommentUpdated`
- [ ] `handle_delete_comment()` - Deletes own comment only, returns `CommentDeleted`
- [ ] `handle_get_comments()` - Returns `CommentsList` for work item
- [ ] Business rules enforced: one active sprint, status transitions, own comments only

**Frontend (Blazor):**
- [ ] Proto regeneration successful for both Rust and C#
- [ ] `SprintStore` uses WebSocket with optimistic updates + rollback
- [ ] `CommentStore` uses WebSocket with optimistic updates + rollback
- [ ] `IWebSocketClient` has Sprint/Comment events and operations
- [ ] Sprint UI: SprintList, SprintCard, SprintDialog, SprintStatusBadge
- [ ] Comment UI: CommentThread, CommentItem, CommentEditor
- [ ] Real-time updates work across browser tabs

**Tests:**
- [ ] `just test-rs-ws` passes (handler tests)
- [ ] `just test-frontend` passes (component + store tests)
- [ ] `just check` passes (full workspace)
