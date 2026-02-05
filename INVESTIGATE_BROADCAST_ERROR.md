# WebSocket Broadcast Investigation - Real-Time Sync Gap

**Date**: 2026-02-05
**Issue**: Multi-client real-time sync broken for ALL entity types
**Session**: 100.3 - CLI REST API integration
**Scope**: Systemic architecture issue (not just CLI/REST API)

## Problem Statement

When creating a work item via the CLI (REST API), the database is updated successfully, but the Blazor UI does not receive a real-time update via WebSocket. The user must navigate away and reload to see the new work item.

**Expected Behavior**: CLI changes appear instantly in Blazor UI (as promised in Session 100 plan)
**Actual Behavior**: UI only updates after manual navigation/reload

## Test Case

```bash
./pm work-item create \
  --project-id cc8dc131-5a26-489c-829f-fa9fa066c850 \
  --type epic \
  --title "e2"
```

**Result**:
- ✅ Work item created: `1c798df9-6f4f-42d9-bb0e-c1ca9d24fd61`
- ✅ Database updated (verified via sqlite3)
- ❌ Blazor UI didn't update automatically
- ✅ UI showed new item after navigate away + reload

## Timeline from Server Logs

```
[06:31:14] - WebSocket connection 05380f78 established
[06:31:31] - Client subscribes to project cc8dc131-5a26-489c-829f-fa9fa066c850
[06:31:31] - Client gets 3 work items for project
[06:31:36] - REST API creates work item 1c798df9 (work item #4) ← THE PROBLEM
           - NO broadcast success/failure logged
           - NO "Failed to broadcast" warning
[06:31:46] - Client navigates away and back, re-subscribes
[06:31:46] - Client NOW gets 4 work items (including new one)
```

**Key Finding**: No broadcast logs at all - neither success nor failure.

## Code Analysis

### REST API Broadcast Code

**File**: `backend/pm-server/src/api/work_items/work_items.rs:218-237`

```rust
// 9. Broadcast to WebSocket clients
let event = build_activity_log_created_event(&activity);
let bytes = event.encode_to_vec();
let message = Message::Binary(bytes.into());
if let Err(e) = state
    .registry
    .broadcast_activity_log_created(
        &project_id.to_string(),
        Some(&work_item.id.to_string()),
        None,
        message,
    )
    .await
{
    log::warn!(
        "Failed to broadcast work item creation to WebSocket clients: {}",
        e
    );
    // This is OK - database operation succeeded, UI will update on next refresh
}

log::info!(
    "Created work item {} ({}) via REST API",
    work_item.id,
    work_item.item_number
);
```

**Problem**: Only logs errors, doesn't log successful deliveries or zero-delivery success.

### Broadcast Implementation

**File**: `backend/crates/pm-ws/src/connection_registry.rs:162-206`

```rust
pub async fn broadcast_activity_log_created(
    &self,
    project_id: &str,
    work_item_id: Option<&str>,
    sprint_id: Option<&str>,
    message: Message,
) -> WsErrorResult<usize> {
    // Get all connections
    let inner = self.inner.read().await;
    let connections: Vec<(ClientSubscriptions, mpsc::Sender<Message>)> = inner
        .connections
        .values()
        .map(|info| (info.subscriptions.clone(), info.sender.clone()))
        .collect();
    drop(inner);

    let mut delivered = 0;
    for (subscriptions, sender) in connections {
        let should_receive = if let Some(work_item_id) = work_item_id {
            SubscriptionFilter::should_receive_work_item_event(
                &subscriptions,
                project_id,
                work_item_id,
            )
        } else if let Some(sprint_id) = sprint_id {
            SubscriptionFilter::should_receive_sprint_event(
                &subscriptions,
                project_id,
                sprint_id,
            )
        } else {
            subscriptions.is_subscribed_to_project(project_id)
        };

        if should_receive {
            if sender.send(message.clone()).await.is_ok() {
                delivered += 1;
            } else {
                debug!("Broadcast send failed; skipping connection");
            }
        }
    }

    Ok(delivered)  // ← Returns count but REST API doesn't log it
}
```

**Returns**: `Ok(delivered)` where `delivered` is the count of successful deliveries.

### Subscription Filter Logic

**File**: `backend/crates/pm-ws/src/subscription_filter.rs:8-16`

```rust
pub fn should_receive_work_item_event(
    subscriptions: &ClientSubscriptions,
    project_id: &str,
    work_item_id: &str,
) -> bool {
    // Receive if subscribed to the project OR the specific work item
    subscriptions.is_subscribed_to_project(project_id)
        || subscriptions.is_subscribed_to_work_item(work_item_id)
}
```

**Logic**: Client receives event if:
1. Subscribed to the project (should be true based on logs), OR
2. Subscribed to the specific work item (won't be true for new work items)

## ROOT CAUSE IDENTIFIED

**The broadcast works correctly, but the wrong event type is being sent for work item updates.**

### Architecture Discovery

The WebSocket protocol has TWO different event types:

| Event Type | Purpose | Broadcasted To |
|------------|---------|----------------|
| `ActivityLogCreated` | Activity feed updates | All subscribed clients |
| `WorkItemCreated` | Work item store updates | **Only the calling client** (as response) |

### WebSocket Handler Behavior (`work_item.rs:162-173`)

```rust
// Broadcast ActivityLogCreated to ALL clients (for activity feed)
ctx.registry
    .broadcast_activity_log_created(&project_id_str, Some(&work_item_id_str), None, message)
    .await?;

// Build WorkItemCreated response (returned ONLY to calling client)
let response = build_work_item_created_response(&ctx.message_id, &work_item, ctx.user_id);
```

### Blazor Client Event Subscriptions

**WorkItemStore.cs:33-35** (updates Kanban board):
```csharp
_client.OnWorkItemCreated += HandleWorkItemCreated;   // ← Only from direct responses
_client.OnWorkItemUpdated += HandleWorkItemUpdated;
_client.OnWorkItemDeleted += HandleWorkItemDeleted;
// ❌ Does NOT subscribe to OnActivityLogCreated
```

**ActivityFeed.razor:86** (updates activity feed only):
```csharp
Client.OnActivityLogCreated += HandleActivityCreated;
```

### Why This Affects ALL Multi-Client Scenarios

This is not just a CLI/REST API issue. Even with **two Blazor users**:
- User A creates work item → User A sees it (via direct response)
- User B only receives `ActivityLogCreated` → User B's work item list doesn't update!

The CLI via REST API has the same limitation because it only broadcasts `ActivityLogCreated`.

### SYSTEMIC ISSUE: All Entity Types Affected

The same pattern exists across ALL entity handlers:

| Entity | Broadcast (all clients) | Response (caller only) | Multi-client sync? |
|--------|------------------------|------------------------|-------------------|
| WorkItem | `ActivityLogCreated` | `WorkItemCreated/Updated/Deleted` | ❌ Broken |
| Comment | `ActivityLogCreated` | `CommentCreated/Updated/Deleted` | ❌ Broken |
| Sprint | `ActivityLogCreated` | `SprintCreated/Updated/Deleted` | ❌ Broken |
| Project | `ActivityLogCreated` | `ProjectCreated/Updated/Deleted` | ❌ Broken |

**Every entity type follows this pattern:**
```rust
// Broadcast metadata only (to all subscribed clients)
ctx.registry.broadcast_activity_log_created(...).await?;

// Return full data (only to the client that made the request)
let response = build_<entity>_created_response(...);
```

This means **no real-time collaboration works** - users must refresh to see each other's changes.

## Recommended Fixes

### Fix Option A: Add WorkItemCreated Broadcast (Backend Change)

Add a new broadcast for `WorkItemCreated` events so all subscribed clients receive work item changes.

**Step 1: Add broadcast method to ConnectionRegistry**

**File**: `backend/crates/pm-ws/src/connection_registry.rs`

```rust
/// Broadcast WorkItemCreated event to matching subscribers
pub async fn broadcast_work_item_created(
    &self,
    project_id: &str,
    message: Message,
) -> WsErrorResult<usize> {
    let inner = self.inner.read().await;
    let connections: Vec<(ClientSubscriptions, mpsc::Sender<Message>)> = inner
        .connections
        .values()
        .map(|info| (info.subscriptions.clone(), info.sender.clone()))
        .collect();
    drop(inner);

    let mut delivered = 0;
    for (subscriptions, sender) in connections {
        if subscriptions.is_subscribed_to_project(project_id) {
            if sender.send(message.clone()).await.is_ok() {
                delivered += 1;
            }
        }
    }

    Ok(delivered)
}
```

**Step 2: Create helper function for WorkItemCreated event**

**File**: `backend/crates/pm-ws/src/handlers/response_builder.rs`

```rust
pub fn build_work_item_created_broadcast(work_item: &WorkItem, project_key: &str) -> WebSocketMessage {
    WebSocketMessage {
        message_id: Uuid::new_v4().to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(ProtoWorkItemCreated(WorkItemCreated {
            work_item: Some(work_item_to_proto(work_item, project_key)),
        })),
    }
}
```

**Step 3: Add broadcast in REST API handler**

**File**: `backend/pm-server/src/api/work_items/work_items.rs`

After the existing `ActivityLogCreated` broadcast, add:

```rust
// 10. Broadcast WorkItemCreated to all subscribers (for WorkItemStore)
let work_item_event = build_work_item_created_broadcast(&work_item, &project.key);
let work_item_bytes = work_item_event.encode_to_vec();
let work_item_message = Message::Binary(work_item_bytes.into());
let _ = state
    .registry
    .broadcast_work_item_created(&project_id.to_string(), work_item_message)
    .await;
```

**Step 4: Add broadcast in WebSocket handler**

**File**: `backend/crates/pm-ws/src/handlers/work_item.rs`

After the existing `ActivityLogCreated` broadcast, add:

```rust
// 10b. Broadcast WorkItemCreated to OTHER subscribers
let work_item_event = build_work_item_created_broadcast(&work_item, &project.key);
let work_item_bytes = work_item_event.encode_to_vec();
let work_item_message = Message::Binary(work_item_bytes.into());
let _ = ctx.registry
    .broadcast_work_item_created(&project_id_str, work_item_message)
    .await;
```

**Pros**: Clean separation of concerns, follows existing pattern
**Cons**: More backend changes, duplicate broadcast (calling client gets response + broadcast)

---

### Fix Option B: WorkItemStore Reacts to ActivityLogCreated (Frontend Change)

Make `WorkItemStore` listen to `OnActivityLogCreated` and fetch new work items.

**File**: `frontend/ProjectManagement.Services/State/WorkItemStore.cs`

```csharp
public WorkItemStore(IWebSocketClient client)
{
    _client = client;
    _client.OnWorkItemCreated += HandleWorkItemCreated;
    _client.OnWorkItemUpdated += HandleWorkItemUpdated;
    _client.OnWorkItemDeleted += HandleWorkItemDeleted;
    _client.OnActivityLogCreated += HandleActivityLogCreated;  // ADD THIS
}

private void HandleActivityLogCreated(ActivityLog entry)
{
    // Only handle work_item entity types
    if (entry.EntityType != "work_item")
        return;

    switch (entry.Action)
    {
        case "created":
            // Fetch the new work item if we don't have it
            if (!_items.ContainsKey(entry.EntityId))
            {
                // Fire-and-forget fetch (or use a background queue)
                _ = FetchAndAddWorkItemAsync(entry.EntityId);
            }
            break;
        case "deleted":
            // Already handled by OnWorkItemDeleted, but backup
            _items.TryRemove(entry.EntityId, out _);
            NotifyChanged();
            break;
    }
}

private async Task FetchAndAddWorkItemAsync(Guid workItemId)
{
    try
    {
        var workItem = await _client.GetWorkItemAsync(workItemId);
        if (workItem != null)
        {
            HandleWorkItemCreated(workItem);
        }
    }
    catch (Exception ex)
    {
        _logger.LogWarning(ex, "Failed to fetch work item {Id} from ActivityLog event", workItemId);
    }
}
```

**Pros**: Minimal backend changes, leverages existing broadcast
**Cons**: Extra round-trip to fetch work item details, slightly more complex

---

### Recommended: Fix Option A

Option A is cleaner because:
1. Work item data is already available at broadcast time - no need for extra fetch
2. Follows the existing pattern for other entity types
3. More efficient - single broadcast contains full work item

The duplicate broadcast to the calling client is harmless - `HandleWorkItemCreated` should handle idempotency (check if item already exists).

## Implementation Plan (Simplified - Everything Exists!)

All proto messages, frontend handlers, and response builders already exist.
**Just add broadcast calls next to existing response builders.**

### The Pattern (same for all handlers)

```rust
// EXISTING: Build response (already done in every handler)
let response = build_work_item_created_response(&ctx.message_id, &work_item, ctx.user_id);

// ADD: Broadcast the same response to all project subscribers
let broadcast = build_work_item_created_response(&Uuid::new_v4().to_string(), &work_item, ctx.user_id);
let bytes = broadcast.encode_to_vec();
ctx.registry
    .broadcast_to_project(&project_id_str, Message::Binary(bytes.into()))
    .await?;

// EXISTING: Return response to caller
Ok(response)
```

### Phase 1: Work Items

**WebSocket handlers** (`backend/crates/pm-ws/src/handlers/work_item.rs`):
- `handle_create` (~line 170): Add WorkItemCreated broadcast
- `handle_update` (~line 316): Add WorkItemUpdated broadcast
- `handle_delete` (~line 401): Add WorkItemDeleted broadcast

**REST API handlers** (`backend/pm-server/src/api/work_items/work_items.rs`):
- `create_work_item`: Add WorkItemCreated broadcast
- `update_work_item`: Add WorkItemUpdated broadcast
- `delete_work_item`: Add WorkItemDeleted broadcast

### Phase 2: Comments

**WebSocket handlers** (`backend/crates/pm-ws/src/handlers/comment.rs`):
- `handle_create`: Add CommentCreated broadcast
- `handle_update`: Add CommentUpdated broadcast
- `handle_delete`: Add CommentDeleted broadcast

**REST API handlers** (`backend/pm-server/src/api/comments/comments.rs`):
- Add CommentCreated/Updated/Deleted broadcasts

### Phase 3: Sprints

**WebSocket handlers** (`backend/crates/pm-ws/src/handlers/sprint.rs`):
- Add SprintCreated/Updated/Deleted broadcasts

### Note: May Need Generic Broadcast Method

The existing `broadcast_activity_log_created()` filters by project + work_item + sprint.
May need a simpler `broadcast_to_project(project_id, message)` method, or just reuse
the existing one with appropriate parameters.

### Testing Strategy

**Phase 1 Test (Work Items)**:
1. Start server and Blazor UI
2. Open Blazor UI to project view (Kanban board)
3. Run CLI: `./pm work-item create --project-id <id> --type task --title "Test"`
4. Verify work item appears in Blazor UI without refresh
5. Test with two Blazor windows (User A creates, User B should see it)

**Phase 2 Test (Comments)**:
1. Open work item detail in two browser windows
2. Add comment in Window A
3. Verify comment appears in Window B without refresh

**Phase 3 Test (Sprints)**:
1. Open sprint board in two browser windows
2. Create/modify sprint in Window A
3. Verify changes appear in Window B without refresh

## Files to Modify

### Maybe: Add generic broadcast method
1. **backend/crates/pm-ws/src/connection_registry.rs**
   - Add `broadcast_to_project(project_id, message)` if needed
   - Or reuse existing `broadcast_activity_log_created()` infrastructure

### Phase 1: Work Items (add ~6 broadcast calls)
2. **backend/crates/pm-ws/src/handlers/work_item.rs**
   - Add broadcast in `handle_create`, `handle_update`, `handle_delete`

3. **backend/pm-server/src/api/work_items/work_items.rs**
   - Add broadcast in `create_work_item`, `update_work_item`, `delete_work_item`

### Phase 2: Comments (add ~6 broadcast calls)
4. **backend/crates/pm-ws/src/handlers/comment.rs**
   - Add broadcast in create/update/delete handlers

5. **backend/pm-server/src/api/comments/comments.rs**
   - Add broadcast in create/update/delete handlers

### Phase 3: Sprints (add ~3 broadcast calls)
6. **backend/crates/pm-ws/src/handlers/sprint.rs**
   - Add broadcast in create/update/delete handlers

## Related Code References

### Backend (Rust)
- REST API create handler: `backend/pm-server/src/api/work_items/work_items.rs:110-248`
- WebSocket work item handler: `backend/crates/pm-ws/src/handlers/work_item.rs:40-200`
- Broadcast implementation: `backend/crates/pm-ws/src/connection_registry.rs:162-206`
- Event builder: `backend/crates/pm-ws/src/handlers/response_builder.rs:620-628`

### Frontend (C#)
- WorkItemStore event subscriptions: `frontend/ProjectManagement.Services/State/WorkItemStore.cs:33-35`
- ActivityFeed event subscription: `frontend/ProjectManagement.Components/Activity/ActivityFeed.razor:86`
- WebSocket event dispatch: `frontend/ProjectManagement.Services/WebSocket/WebSocketClient.cs:689-802`

## Key Discovery

The broadcast system works correctly. The issue is that:

1. **Only `ActivityLogCreated` is broadcast** to all clients
2. **`WorkItemCreated` is only returned** as a direct response to the calling client
3. **WorkItemStore doesn't subscribe** to `OnActivityLogCreated`

This is a **design gap**, not a bug. The current architecture doesn't support real-time work item synchronization across multiple clients.

## Next Session Goals

### Session 100.4 - Fix Real-Time Sync (~1 hour total)
1. Add generic `broadcast_to_project()` method (if needed)
2. Add WorkItem broadcasts (3 handlers × 2 locations = 6 changes)
3. Add Comment broadcasts (3 handlers × 2 locations = 6 changes)
4. Add Sprint broadcasts (3 handlers = 3 changes)
5. Test with CLI + multiple Blazor windows
6. Verify no duplicate handling issues (frontend stores should be idempotent)

### Total: ~15 broadcast calls to add

## Success Criteria

### Phase 1: Work Items (Minimum for Session 100)
- [ ] CLI work item creation triggers real-time UI update
- [ ] CLI work item update triggers real-time UI update
- [ ] CLI work item delete triggers real-time UI update
- [ ] Multiple Blazor users see each other's work item changes

### Phase 2: Comments
- [ ] Multiple users see each other's comments in real-time

### Phase 3: Sprints
- [ ] Multiple users see sprint changes in real-time

### Overall
- [ ] No manual navigation/reload required for any entity type
- [ ] True real-time collaboration enabled

---

**Status**: ROOT CAUSE IDENTIFIED - Broadcasts never implemented (but everything else exists!)
**Scope**: ALL entity types affected (WorkItems, Comments, Sprints, Projects)
**Priority**: High - real-time collaboration completely broken
**Fix Complexity**: LOW - all infrastructure already exists!

**What already exists:**
- ✅ Proto messages (all 12 CRUD events defined)
- ✅ Frontend WebSocketClient (dispatches all 12 event types)
- ✅ Frontend stores (subscribe to all events)
- ✅ Backend response builders (`build_*_created/updated/deleted_response()`)
- ❌ Backend broadcasts - **just need to add broadcast calls**

**Estimated Fix Time**:
- Phase 1 (Work Items): 15-30 minutes (add 3 broadcast calls)
- Phase 2 (Comments): 15-30 minutes (add 3 broadcast calls)
- Phase 3 (Sprints): 15-30 minutes (add 3 broadcast calls)
- Total: ~1 hour for all entity types
