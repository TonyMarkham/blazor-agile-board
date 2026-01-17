# Session 30.3 Pre-Plan: Deficiency Analysis

**Purpose**: Document all issues found in the original session-30.3-plan.md before creating corrected implementation.

**Status**: Analysis complete, corrections pending

---

## Critical Issues Requiring Code Changes

### Issue #1: Subscribe/Unsubscribe Handler Signature Mismatch

**Location**: Phase 1, lines 22-69

**Problem**:
- Plan assumes single project_id subscription
- Actual protobuf uses `Vec<String>` for both project_ids and sprint_ids
- Plan returns `SubscribeAck` which doesn't exist in protobuf

**Evidence**:
```rust
// Plan says:
pub async fn handle_subscribe(
    request: Subscribe,
    subscriptions: &mut HashSet<Uuid>,
) -> Result<SubscribeAck, WsError> {
    let project_id = Uuid::parse_str(&request.project_id)  // ❌ No such field

// Actual protobuf (pm.rs:249-254):
pub struct Subscribe {
    pub project_ids: Vec<String>,   // ✅ Multiple projects
    pub sprint_ids: Vec<String>,    // ✅ Multiple sprints
}
```

**Fix Required**:
- Loop over `request.project_ids` vector
- Handle multiple subscriptions atomically
- Remove SubscribeAck return type (use `Result<(), WsError>`)
- Add TODO comment for sprint_ids support

**User Decision**: Keep multi-project subscription design (approved)

---

### Issue #2: Plan Outdated - Repository Executor Pattern Already Implemented ✅ CODE IS CORRECT

**Location**: Phase 2 (work_item.rs), lines 168-182, 218-231, 258-272, 348-362

**Situation**:
Implementation uses executor pattern (commit `8e73572`), but plan still shows instance pattern. The executor pattern is **production-grade and correct** - the PLAN needs updating, not the code.

**Why Executor Pattern is Better**:
- One API works with both `SqlitePool` AND `&mut Transaction`
- Zero-sized type (no memory overhead per instance)
- SQLx idiomatic pattern for transaction support
- All 118 tests passing with this pattern

**Evidence**:
```rust
// Plan shows (OUTDATED):
let work_item_repo = WorkItemRepository::new_with_tx(&mut tx);
work_item_repo.create(&work_item).await

// Actual implementation (CORRECT):
pub struct WorkItemRepository;  // Zero-sized type

impl WorkItemRepository {
    pub async fn create<'e, E>(executor: E, work_item: &WorkItem) -> DbErrorResult<()>
    where E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    //    ^^^^^^^^^^^^^^^^^ Works with Pool OR Transaction!
}

// Usage:
let mut tx = ctx.pool.begin().await?;
WorkItemRepository::create(&mut tx, &work_item).await?;  // ✅
WorkItemRepository::create(&ctx.pool, &work_item).await?;  // ✅ Also works
```

**Affected Methods** (all need plan updates):
- `WorkItemRepository::create(executor, &work_item)`
- `WorkItemRepository::find_by_id(executor, id)`
- `WorkItemRepository::update(executor, &work_item)`
- `WorkItemRepository::soft_delete(executor, id, user_id)`
- `WorkItemRepository::find_children(executor, parent_id)`
- `WorkItemRepository::find_max_position(executor, project_id, parent_id)`
- `WorkItemRepository::find_by_project(executor, project_id)`
- `WorkItemRepository::find_by_project_since(executor, project_id, since)`
- `ActivityLogRepository::create(executor, &log)`

**Fix Required**:
Update plan Phase 2 code to use executor pattern throughout. No code changes needed.

**User Decision**: Executor pattern approved as production-grade (2026-01-16).

---

### Issue #3: Plan Writing Error - Missing `mut` Keyword

**Location**: Phase 2, lines 152-160

**Problem**:
Plan has typo - forgot `mut` keyword when creating mutable WorkItem.

**Evidence**:
```rust
let work_item = WorkItem::new(...);  // ❌ Plan forgot mut
work_item.position = position;       // ❌ Won't compile
```

**Fix Required**:
```rust
let mut work_item = WorkItem::new(...);  // ✅ Add mut
work_item.position = position;
```

**Assessment**: Simple documentation error in plan writing, not an architectural issue.

---

### Issue #4: Missing Type Conversion for WorkItemType

**Location**: Phase 2, line 154

**Problem**:
i32 from protobuf doesn't auto-convert to WorkItemType enum.

**Evidence**:
```rust
WorkItem::new(
    request.item_type.into(),  // ❌ i32 -> WorkItemType not implemented
```

**Fix Required**:
```rust
use pm_core::WorkItemType;

let item_type = WorkItemType::try_from(request.item_type)
    .map_err(|_| WsError::ValidationError {
        message: format!("Invalid item_type: {}", request.item_type),
        field: Some("item_type".to_string()),
        location: ErrorLocation::from(Location::caller()),
    })?;

WorkItem::new(item_type, ...)
```

**Note**: Need to verify if `TryFrom<i32>` is implemented for WorkItemType or if manual matching is needed.

---

### Issue #5: Unnecessary Type Conversions in apply_updates

**Location**: Phase 2, lines 414-419

**Problem**:
Plan calls `.into()` on String fields that are already the correct type.

**Evidence**:
```rust
// Plan says:
if let Some(status) = request.status {
    work_item.status = status.into();  // ❌ String -> String conversion
}

// Actual types:
// - WorkItem.status: String (work_item.rs:22)
// - UpdateWorkItemRequest.status: Option<String> (pm.rs:287)
```

**Fix Required**:
```rust
if let Some(ref status) = request.status {
    work_item.status = status.clone();  // ✅ Direct assignment
}
// Or simply:
if let Some(status) = request.status {
    work_item.status = status;  // ✅ Move the string
}
```

**Affected Fields**:
- status (line 414-417)
- priority (line 417-420)
- assignee_id (needs different fix - see below)

---

### Issue #6: WsError::NotFound Signature Mismatch

**Location**: Phase 2, lines 227-231, 313-317

**Problem**:
Plan uses NotFound variant with resource/id fields, but actual error only has message field.

**Evidence**:
```rust
// Plan assumes:
WsError::NotFound {
    resource: "WorkItem".to_string(),
    id: work_item_id.to_string(),
    location: ErrLoc::caller(),
}

// Actual definition (error.rs:73-76):
NotFound {
    message: String,  // ✅ Only message field exists
    location: ErrorLocation,
}
```

**Fix Required**:
```rust
WsError::NotFound {
    message: format!("WorkItem {} not found", work_item_id),
    location: ErrorLocation::from(Location::caller()),
}
```

---

### Issue #7: Phase 4 WebSocketConnection Integration Issues

**Location**: Phase 4, lines 544-645

**Multiple Problems**:

#### 7a. Missing connection_manager Field
```rust
// Plan assumes (line 574):
self.connection_manager.get_pool(&self.tenant_context.tenant_id).await?

// Actual WebSocketConnection (web_socket_connection.rs:18-28):
pub struct WebSocketConnection {
    connection_id: ConnectionId,
    tenant_context: TenantContext,
    config: ConnectionConfig,
    metrics: Metrics,
    rate_limiter: ConnectionRateLimiter,
    broadcaster: TenantBroadcaster,
    subscriptions: ClientSubscriptions,
    rate_limit_violations: u32,
    // ❌ No connection_manager field
}
```

**Fix Required**: Need to add connection_manager field to WebSocketConnection or pass pool differently.

#### 7b. Missing send_response_and_broadcast Method
```rust
// Plan calls (lines 583, 587, 591):
self.send_response_and_broadcast(response, broadcast).await

// ❌ This method doesn't exist in WebSocketConnection
```

**Fix Required**: Implement this method in Phase 4 modifications.

#### 7c. Missing send_subscribe_ack Method
```rust
// Plan calls (line 600):
self.send_subscribe_ack(ack).await

// ❌ This method doesn't exist
// ❌ SubscribeAck doesn't exist in protobuf (see Issue #1)
```

**Fix Required**: Remove this call since subscription doesn't need acknowledgment.

---

### Issue #8: Missing Error Response Export

**Location**: Phase 4, line 616

**Problem**:
Plan calls `handlers::build_error_response()` but it's not publicly exported.

**Evidence**:
```rust
// Plan calls:
handlers::build_error_response(&message.message_id, e.to_proto_error())

// lib.rs exports (lines 42-45):
pub use handlers::response_builder::{
    build_work_item_created_response,
    build_work_item_deleted_response,
    build_work_item_updated_response,
    build_work_items_list_response,
    // ❌ build_error_response NOT exported
};
```

**Fix Required**:
- Add `build_error_response` to public exports in lib.rs, OR
- Use full path: `handlers::response_builder::build_error_response(...)`

---

### Issue #9: Test Helper Functions Not Implemented

**Location**: Phase 5, lines 664-965

**Problem**:
Test code references many helper functions that are declared but not implemented.

**Missing Helpers**:
- `create_test_pool()` (line 959)
- `run_migrations()` (line 960)
- `create_test_story()` (line 961)
- `create_test_epic()` (line 962)
- `create_test_story_with_parent()` (line 963)
- `update_item_version()` (line 964)
- `create_test_story_with_timestamp()` (line 942)

**Fix Required**:
- Implement all helper functions in test file, OR
- Reference existing test fixtures from `pm-db/tests/common/fixtures.rs`

---

## Secondary Issues (Non-Critical)

### S1: Plan Uses Wrong Error Location Pattern

**Location**: Phase 2, multiple locations

**Problem**: Plan uses non-existent `ErrLoc::caller()` shorthand. The correct pattern is `ErrorLocation::from(Location::caller())`.

**Evidence**:
```rust
// Plan incorrectly uses:
location: ErrLoc::caller(),  // ❌ ErrLoc doesn't exist

// Actual correct pattern (used throughout existing code):
use error_location::ErrorLocation;
use std::panic::Location;

location: ErrorLocation::from(Location::caller()),  // ✅
```

**Fix Required**: Replace all instances of `ErrLoc::caller()` with `ErrorLocation::from(Location::caller())`

---

### S2: Idempotency Cache Deserialization Not Implemented

**Location**: Phase 2, lines 447-454

**Problem**: Function returns error instead of implementing deserialization.

```rust
fn deserialize_cached_response(json: &str) -> Result<...> {
    Err(WsError::Internal {
        message: "Cached response replay not yet implemented".to_string(),
        // ...
    })
}
```

**Impact**: Idempotency checking will fail on replay attempts.

**Fix Required**: Implement proper deserialization or remove idempotency check call from create handler.

---

### S3: apply_updates Has Logic Error with assignee_id

**Location**: Phase 2, lines 420-423

**Problem**:
```rust
if let Some(ref assignee) = request.assignee_id {
    work_item.assignee_id = Some(Uuid::parse_str(assignee).ok()).flatten();
    //                      ^^^^ Creates Option<Option<Uuid>>, then flattens
}
```

**Better Approach**:
```rust
if let Some(ref assignee) = request.assignee_id {
    work_item.assignee_id = Uuid::parse_str(assignee).ok();
}
```

---

### S4: Missing use Statements

The plan snippets are missing various use statements that would be needed:

**Phase 2 needs**:
```rust
use pm_core::{WorkItem, WorkItemType, ActivityLog};
use pm_db::{WorkItemRepository, ActivityLogRepository};
use pm_proto::{CreateWorkItemRequest, UpdateWorkItemRequest, DeleteWorkItemRequest};
use sqlx::SqlitePool;
use std::panic::Location;
```

---

## Architectural Questions Requiring Decision

### Q1: How Should Pool Be Passed to Handlers? ✅ RESOLVED

**Architecture Understanding:**

The flow is:
1. **AppState** holds `Arc<TenantConnectionManager>` (needs to be added)
2. **WebSocketConnection** gets reference to connection_manager from AppState (needs to be added)
3. **Per-message**: Get pool from connection_manager, create HandlerContext with pool
4. **Handler**: Uses `ctx.pool.begin()` to start transaction
5. **Repository**: Uses executor pattern with transaction

**Correct Flow:**
```rust
// In WebSocketConnection::handle_binary_message:
let pool = self.connection_manager.get_pool(&self.tenant_context.tenant_id).await?;
let ctx = HandlerContext::new(message_id, tenant_id, user_id, pool);

// In handler:
let mut tx = ctx.pool.begin().await?;
WorkItemRepository::create(&mut tx, &work_item).await?;
tx.commit().await?;
```

**Resolution**:
- Add `connection_manager: Arc<TenantConnectionManager>` to AppState
- Add `connection_manager: Arc<TenantConnectionManager>` to WebSocketConnection
- Plan's dispatcher code needs to call `self.connection_manager.get_pool()` as shown

**User Decision**: Approved architecture (2026-01-16)

---

### Q2: Should Idempotency Replay Be Implemented?

**Current Status**: Function stub returns error.

**Options**:
1. Implement full deserialization (complex, requires storing typed responses)
2. Remove idempotency checking for MVP (add later)
3. Store/replay binary protobuf bytes (simpler than JSON deserialization)

**Recommendation**: Option 3 - Store encoded WebSocketMessage bytes, replay directly.

---

### Q3: Sprint-Level Subscriptions?

**Current Status**: Protobuf supports sprint_ids, plan ignores them.

**Options**:
1. Implement sprint subscriptions now (more work)
2. Log and ignore for now (include TODO comment)
3. Return validation error if sprint_ids provided

**User Decision**: Option 2 - Log and ignore for now (matches Issue #1 approach).

---

## Next Steps

1. ⏳ Resolve Issue #1 (Subscribe/Unsubscribe) - in discussion
2. ⏳ Resolve Issue #2 (Repository API)
3. ⏳ Resolve Issues #3-6 (Phase 2 handler corrections)
4. ⏳ Resolve Issue #7 (Phase 4 integration)
5. ⏳ Resolve Issue #8 (exports)
6. ⏳ Resolve Issue #9 (test helpers)
7. ⏳ Address secondary issues
8. ⏳ Make architectural decisions (Q1-Q3)
9. ⏳ Create corrected session-30.3-plan-corrected.md

---

## Files Requiring Updates

| File | Issues | Priority |
|------|--------|----------|
| `pm-ws/src/handlers/subscription.rs` | #1 | HIGH |
| `pm-ws/src/handlers/work_item.rs` | #2, #3, #4, #5, #6, S2, S3 | HIGH |
| `pm-ws/src/handlers/query.rs` | #2, #6 | HIGH |
| `pm-ws/src/web_socket_connection.rs` | #7a, #7b, #7c | HIGH |
| `pm-ws/src/lib.rs` | #8 | MEDIUM |
| `pm-ws/tests/work_item_handler_tests.rs` | #9 | MEDIUM |

---

**Document Created**: 2026-01-16
**Analysis By**: Claude (Teaching Mode)
**Status**: Issues documented, awaiting systematic resolution
