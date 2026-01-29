# Postmortem: Added Changes Above Session 70.1

This document lists the code added **beyond** the 70.1 plan, why it was added, and its current state.

> **Status:** These additions are incomplete and not production‑grade. They should be reviewed, fixed, or removed.

---

## Summary of Added Changes (Above 70.1)

### 1) WebSocket Broadcast Plumbing (pm-ws)

**Goal (intended):** enable real‑time `ActivityLogCreated` broadcasts to connected clients.

**Added/Modified Files:**
- `backend/crates/pm-ws/src/connection_info.rs`
  - Added `sender: mpsc::Sender<Message>`
  - Added `subscriptions: ClientSubscriptions`
- `backend/crates/pm-ws/src/connection_registry.rs`
  - `register(...)` now accepts a sender and stores subscriptions
- `backend/crates/pm-ws/src/handlers/context.rs`
  - Added `registry: ConnectionRegistry` to `HandlerContext`
- `backend/crates/pm-ws/src/web_socket_connection.rs`
  - Added `registry`, `outgoing_rx`, `outgoing_tx` fields
  - Constructor updated to accept these fields
  - Handler context now receives `registry`
- `backend/crates/pm-ws/src/app_state.rs`
  - Moved channel creation + registry registration into `handle_socket`
  - `WebSocketConnection::new(...)` now receives `rx` and `tx`

**Why it was added:**
Session 70.2 **Part 5: WebSocket Client Methods** (“Activity query and real‑time subscriptions”)
depends on real‑time `ActivityLogCreated` broadcasts for the activity feed UI.
This plumbing was added to provide a server → client broadcast path.

**Intended Implementation Thinking (production‑grade):**
1. **Broadcast plumbing**
   - Each active connection stores a sender (`mpsc::Sender<Message>`) and subscriptions (`ClientSubscriptions`).
   - `ConnectionRegistry` owns this state; `HandlerContext` exposes the registry to handlers.
2. **Subscriptions**
   - Implement `Subscribe/Unsubscribe` handlers to mutate `ClientSubscriptions`.
   - Filter outbound events using `SubscriptionFilter` (already in the codebase).
3. **Broadcast helper**
   - Registry-level helper builds `ActivityLogCreated` and sends only to subscribed clients.
   - Record metrics via `Metrics::broadcast_published`.
4. **Emit on writes**
   - After activity‑log writes in mutation handlers (work items, sprints, comments, time entries, dependencies), emit `ActivityLogCreated`.

**Concrete Plan to Finish the Feature (no guessing):**
1. **Implement Subscribe/Unsubscribe** (pm-ws)
   - **File:** `backend/crates/pm-ws/src/handlers/dispatcher.rs`
     - Replace match arm at lines ~130–139:
       ```rust
       Some(Payload::Subscribe(_)) | Some(Payload::Unsubscribe(_)) => {
           return build_error_response(... NOT_IMPLEMENTED ...);
       }
       ```
       with calls to `handle_subscribe(req, ctx)` and `handle_unsubscribe(req, ctx)`.
   - **Create:** `backend/crates/pm-ws/src/handlers/subscription.rs`
     - `handle_subscribe(req, ctx)`
     - `handle_unsubscribe(req, ctx)`
   - **Update:** `backend/crates/pm-ws/src/handlers/mod.rs` to export `subscription` module.
   - **Storage:** mutate `ClientSubscriptions` inside `ConnectionRegistry` for the caller’s connection.
   - **Tests:** add `backend/crates/pm-ws/tests/subscription_handler_tests.rs`.
2. **Broadcast Helper in ConnectionRegistry**
   - **File:** `backend/crates/pm-ws/src/connection_registry.rs`
   - Add a helper like `broadcast_to_project(project_id, message_bytes)`.
   - **Serialization:** `WebSocketMessage::encode_to_vec()` and send as `Message::Binary`.
   - **Filtering:** use `SubscriptionFilter`:
     - work item → `should_receive_work_item_event`
     - sprint → `should_receive_sprint_event`
     - comment → `should_receive_comment_event`
     - time_entry / dependency → treat like work item (project_id + work_item_id)
   - **Metrics:** call `Metrics::broadcast_published("ActivityLogCreated", subscriber_count)`.
3. **Emit ActivityLogCreated on Mutation**
   - **Handlers to update:**
     - `backend/crates/pm-ws/src/handlers/work_item.rs`
     - `backend/crates/pm-ws/src/handlers/sprint.rs`
     - `backend/crates/pm-ws/src/handlers/comment.rs`
     - `backend/crates/pm-ws/src/handlers/time_entry.rs`
     - `backend/crates/pm-ws/src/handlers/dependency.rs`
   - **Project resolution:**
     - Use the existing entity’s `project_id` when available (work_item, sprint).
     - For comment/time_entry/dependency, resolve project_id via the related work item.
   - **Payload:** build `ActivityLogCreated` using `ActivityLogEntry` mapping in
     `backend/crates/pm-ws/src/handlers/response_builder.rs` (reuse `activity_log_to_proto`).
   - **Send:** call `ctx.registry.broadcast_to_project(...)` after DB commit succeeds.
4. **Add Tests**
   - **Registry filtering:** unit test for subscribed vs non‑subscribed connections.
   - **Integration:** create work item → ensure subscribed client receives `ActivityLogCreated`.
   - **Files:**
     - `backend/crates/pm-ws/tests/activity_log_broadcast_tests.rs`
     - `backend/crates/pm-ws/tests/subscription_handler_tests.rs`
   - **Test harness hint:** create `mpsc::channel`, register connection, call broadcast helper,
     and assert received `Message::Binary` decodes to `ActivityLogCreated`.
5. **Verification**
   - `just check-rs-ws`
   - `just test-rs-ws`

**Current state:**
- **No broadcast helper exists**
- **No `ActivityLogCreated` emission** in mutation handlers
- **Subscribe/Unsubscribe is still NOT_IMPLEMENTED** in dispatcher
- Registry stores subscriptions but **they are unused**
- **Not production‑grade**: uses `unwrap()`, introduces warnings, and has incomplete lifecycle handling

---

## Impact and Risk

- Adds complexity with no functional behavior
- Introduces warnings and non‑production patterns
- Risks confusion for future development

---

## Recommendation

Either:
1) **Finish the broadcast feature properly** (implement subscriptions + broadcast filtering + emit events), or
2) **Remove this plumbing** and return to the last known good state.

---

## Concrete Files to Review/Remove (if rolling back)

- `backend/crates/pm-ws/src/connection_info.rs`
- `backend/crates/pm-ws/src/connection_registry.rs`
- `backend/crates/pm-ws/src/handlers/context.rs`
- `backend/crates/pm-ws/src/web_socket_connection.rs`
- `backend/crates/pm-ws/src/app_state.rs`

---

## Closing Note

These additions were made while attempting to add real‑time `ActivityLogCreated` broadcasts. The work stopped before delivering any actual broadcast behavior.
