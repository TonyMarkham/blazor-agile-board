# Session 101: Real-Time Entity Broadcasts

## Summary

This session fixes a systemic design gap where entity CRUD events (WorkItem, Comment, Sprint) are only returned to the calling client, not broadcast to other subscribed clients. This breaks real-time collaboration.

**Root Cause**: All handlers broadcast `ActivityLogCreated` (metadata only) but return entity-specific responses (`WorkItemCreated`, etc.) only to the caller. Other clients never receive the actual entity data.

**Fix**: Add broadcast calls for entity-specific events. All infrastructure already exists:
- Proto messages: All 12 CRUD events defined
- Frontend dispatch: Handles all event types
- Frontend stores: Subscribe to all events
- Backend response builders: `build_*_response()` exist

**We just need to add ~15 broadcast calls.**

---

## Sub-Session Breakdown

| Session | Scope | Est. Time | Status |
|---------|-------|-----------|--------|
| **[101.1](101.1-Session-Plan.md)** | Work Item Broadcasts | 15-30 min | ✅ Complete (2026-02-05) |
| **[101.2](101.2-Session-Plan.md)** | Sprint Broadcasts | 15-30 min | Pending |
| **[101.3](101.3-Session-Plan.md)** | Comment Broadcasts | 15-30 min | Pending |

**Total estimated time: ~1 hour (101.1 completed in ~25 min)**

---

## Background

### The Problem

When User A creates/updates/deletes an entity:
- User A sees the change (via direct response)
- User B (subscribed to same project) does NOT see it until refresh

This affects ALL entity types and ALL CRUD operations.

### Current Architecture

```
Client Request → Handler → DB Write → ActivityLogCreated broadcast → Response to caller only
                                              ↓
                                    All clients get metadata
                                    (entity_id, action, timestamp)
                                              ↓
                                    ActivityFeed updates ✅
                                    WorkItemStore ignores ❌
```

### Fixed Architecture

```
Client Request → Handler → DB Write → ActivityLogCreated broadcast → Response to caller
                                    → EntityCreated broadcast ────→ All clients get full entity
                                              ↓                              ↓
                                    ActivityFeed updates ✅        WorkItemStore updates ✅
```

---

## What Already Exists

| Layer | Status |
|-------|--------|
| Proto messages (`WorkItemCreated`, etc.) | ✅ All 12 defined |
| Frontend `WebSocketClient` dispatch | ✅ Handles all 12 event types |
| Frontend stores (`WorkItemStore`, etc.) | ✅ Subscribe to events |
| Backend response builders | ✅ `build_*_created/updated/deleted_response()` exist |
| Backend broadcasts | ❌ **Only `ActivityLogCreated` is broadcast** |

---

## Implementation Pattern

Every handler follows this pattern. We just add the broadcast call:

```rust
// EXISTING: Build response for caller
let response = build_work_item_created_response(&ctx.message_id, &work_item, ctx.user_id);

// ADD THIS: Broadcast to all project subscribers
let broadcast = build_work_item_created_response(
    &Uuid::new_v4().to_string(),  // New message ID for broadcast
    &work_item,
    ctx.user_id
);
let bytes = broadcast.encode_to_vec();
ctx.registry
    .broadcast_to_project(&project_id_str, Message::Binary(bytes.into()))
    .await?;

// EXISTING: Return response to caller
Ok(response)
```

---

## Pre-Implementation Checklist

Before starting any sub-session:

- [ ] `cargo check --workspace` passes
- [ ] `cargo test --workspace` passes
- [ ] Server runs and Blazor UI connects

---

## Files Summary

### May Need to Create (1 file)

| File | Purpose |
|------|---------|
| (Optional) Generic broadcast method in `connection_registry.rs` | Simplify broadcast calls |

### Files to Modify

| File | Changes |
|------|---------|
| `pm-ws/src/handlers/work_item.rs` | Add 3 broadcast calls |
| `pm-ws/src/handlers/comment.rs` | Add 3 broadcast calls |
| `pm-ws/src/handlers/sprint.rs` | Add 3 broadcast calls |
| `pm-server/src/api/work_items/work_items.rs` | Add 3 broadcast calls |
| `pm-server/src/api/comments/comments.rs` | Add 3 broadcast calls |

**Total: ~15 broadcast calls across 5 files**

---

## Testing Strategy

### Per Sub-Session

1. Build and run server
2. Open Blazor UI in two browser windows
3. Perform CRUD operation in Window A
4. Verify change appears in Window B without refresh

### CLI Integration (Session 101.1)

1. Open Blazor UI to project view
2. Run CLI: `./pm work-item create --project-id <id> --type task --title "Test"`
3. Verify work item appears in Blazor UI without refresh

---

## Success Criteria

- [x] CLI work item CRUD triggers real-time UI updates (✅ 101.1)
- [x] Multiple Blazor users see each other's work item changes (✅ 101.1)
- [ ] Multiple Blazor users see each other's comment changes (101.3)
- [ ] Multiple Blazor users see each other's sprint changes (101.2)
- [x] No manual navigation/reload required (✅ 101.1 - near-instant updates)

---

## Related Documentation

- Investigation: `INVESTIGATE_BROADCAST_ERROR.md`
- Proto definitions: `proto/messages.proto`
- Frontend handlers: `frontend/ProjectManagement.Services/WebSocket/WebSocketClient.cs`
