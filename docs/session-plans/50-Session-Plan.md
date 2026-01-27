# Session 50: Sprints & Comments - Production Implementation

## Production-Grade Score Target: 9.5/10

This session implements Sprint and Comment functionality with full WebSocket integration:

- Sprint CRUD with optimistic locking and status transitions
- Comment CRUD with author-only edit/delete permissions
- Real-time WebSocket events for collaborative editing
- Frontend state management with optimistic updates
- Comprehensive validation and authorization

---

## Sub-Session Breakdown

This plan has been split into sub-sessions to fit within token budgets:

| Session | Scope | Est. Tokens | Actual | Status |
|---------|-------|-------------|--------|--------|
| **[50.1](50.1-Session-Plan.md)** | Proto Schema + Backend Sprint Infrastructure | ~35-40k | ~38k | ✅ Complete (2026-01-27) |
| **[50.2](50.2-Session-Plan.md)** | Backend Comment Handler + Dispatcher Wiring | ~35-40k | ~32k | ✅ Complete (2026-01-27) |
| **[50.3](50.3-Session-Plan.md)** | Frontend Models + WebSocket Integration | ~40-45k | ~35k | ✅ Complete (2026-01-27) |
| **[50.4](50.4-Session-Plan.md)** | State Management + UI Components | ~40-45k | ~30k | ✅ Complete (2026-01-27) |
| **[50.5](50.5-Session-Plan.md)** | Testing (Backend + Frontend) | ~35-40k | TBD | Pending |

---

## Session 50.1: Proto Schema + Backend Sprint Infrastructure

**Files Modified:**
- `proto/messages.proto` - Add Sprint/Comment messages and WebSocket payloads

**Files Modified:**
- `pm-core/src/models/sprint.rs` - Add version field
- `pm-db/src/repositories/sprint_repository.rs` - Update queries for version

**Files Created:**
- `pm-ws/src/handlers/sprint.rs` - Sprint CRUD handlers

**Verification:** `just build-backend && just test-backend`

---

## Session 50.2: Backend Comment Handler + Dispatcher Wiring ✅

**Status**: Complete (2026-01-27)

**Note**: Sprint response builders were completed in Session 50.1.

---

**Files Created:**
- `pm-ws/src/handlers/comment.rs` - Comment CRUD handlers (287 lines)

**Files Modified:**
- `pm-ws/src/handlers/response_builder.rs` - Comment response builders (Sprint already done)
- `pm-ws/src/handlers/dispatcher.rs` - Route Sprint/Comment messages (8 new types)
- `pm-ws/src/handlers/mod.rs` - Export comment module
- `pm-ws/src/lib.rs` - Public exports for Sprint/Comment handlers

**Note**: `message_validator.rs` already had Sprint/Comment validation from Session 50.1

**Verification:** ✅ All tests passing (74 tests), 0 clippy warnings

---

## Session 50.3: Frontend Models + WebSocket Integration ✅

**Status**: Complete (2026-01-27)

**Files Modified:**
- `ProjectManagement.Core/Models/Sprint.cs` - Added Version property
- `ProjectManagement.Core/Models/UpdateSprintRequest.cs` - Added ExpectedVersion + Status
- `ProjectManagement.Core/Converters/ProtoConverter.cs` - Sprint/Comment conversions (+153 lines)
- `ProjectManagement.Core/Interfaces/IWebSocketClient.cs` - Sprint/Comment events and operations (+92 lines)
- `ProjectManagement.Services/WebSocket/WebSocketClient.cs` - Implementation (+231 lines)
- `ProjectManagement.Services/Resilience/ResilientWebSocketClient.cs` - Decorator forwarding (+114 lines)
- `ProjectManagement.Services/State/SprintStore.cs` - WebSocket integration with optimistic updates (+174 lines)
- `ProjectManagement.Services.Tests/State/SprintStoreTests.cs` - Mock setup (+21 lines)

**Files Created:**
- `ProjectManagement.Core/Models/Comment.cs` - Comment model (1361 bytes)
- `ProjectManagement.Core/Models/CreateCommentRequest.cs` - Create request (451 bytes)
- `ProjectManagement.Core/Models/UpdateCommentRequest.cs` - Update request (493 bytes)

**Deliverables:**
- Sprint and Comment WebSocket operations fully implemented
- Optimistic updates with temporary IDs and server confirmation
- Event deduplication to prevent double-updates
- Error rollback to maintain consistency
- All 365 frontend tests passing

**Verification:** ✅ `just build-frontend && just test-frontend` - All tests passing

---

## Session 50.4: State Management + UI Components ✅

**Status**: Complete (2026-01-27)

**Files Created:**
- `ProjectManagement.Core/Interfaces/ICommentStore.cs` - Comment store interface (47 lines)
- `ProjectManagement.Services/State/CommentStore.cs` - Comment state management (235 lines)
- `ProjectManagement.Components/Sprints/SprintCard.razor` - Sprint card component (95 lines)
- `ProjectManagement.Components/Sprints/SprintCard.razor.css` - Sprint card styles (42 lines)
- `ProjectManagement.Components/Sprints/SprintDialog.razor` - Create/edit sprint dialog (168 lines)
- `ProjectManagement.Components/Comments/CommentList.razor` - Comment thread UI (106 lines)
- `ProjectManagement.Components/Comments/CommentList.razor.css` - Comment list styles (45 lines)
- `ProjectManagement.Components/Comments/CommentEditor.razor` - Comment input (52 lines)
- `ProjectManagement.Components/Comments/CommentEditor.razor.css` - Comment editor styles (14 lines)

**Files Modified:**
- `ProjectManagement.Wasm/Program.cs` - ICommentStore service registration

**Deliverables:**
- CommentStore with optimistic updates and WebSocket integration
- Sprint UI components with status-based actions and progress tracking
- Comment UI components with inline editing and author-only permissions
- All 365 frontend tests passing

**Verification:** ✅ `just build-frontend && just test-frontend` - All tests passing

---

## Session 50.5: Testing

**Files Created:**
- `pm-ws/tests/sprint_handler_tests.rs` - Sprint handler integration tests
- `pm-ws/tests/comment_handler_tests.rs` - Comment handler integration tests
- `ProjectManagement.Core.Tests/Converters/SprintConverterTests.cs` - Sprint converter tests
- `ProjectManagement.Core.Tests/Converters/CommentConverterTests.cs` - Comment converter tests
- `ProjectManagement.Services.Tests/State/SprintStoreTests.cs` - Sprint store tests
- `ProjectManagement.Services.Tests/State/CommentStoreTests.cs` - Comment store tests

**Verification:** `just test`

---

## Pre-Implementation Checklist

Before starting **any** sub-session:

- [ ] `just check` passes (all code compiles)
- [ ] Database migrations are current
- [ ] `just build-backend` succeeds
- [ ] `just build-frontend` succeeds

---

## Files Summary

### Create (16 files)

| File | Purpose | Session |
|------|---------|---------|
| `pm-ws/src/handlers/sprint.rs` | Sprint CRUD handlers | ✅ 50.1 |
| `pm-ws/src/handlers/field_change_builder.rs` | Generic change tracker | ✅ 50.1 |
| `pm-ws/src/handlers/comment.rs` | Comment CRUD handlers | ✅ 50.2 |
| `ProjectManagement.Core/Models/Comment.cs` | Comment domain model | ✅ 50.3 |
| `ProjectManagement.Core/Models/CreateCommentRequest.cs` | Create comment request | ✅ 50.3 |
| `ProjectManagement.Core/Models/UpdateCommentRequest.cs` | Update comment request | ✅ 50.3 |
| `ProjectManagement.Core/Interfaces/ICommentStore.cs` | Comment store interface | ✅ 50.4 |
| `ProjectManagement.Services/State/CommentStore.cs` | Comment state management | ✅ 50.4 |
| `ProjectManagement.Components/Sprints/SprintCard.razor` | Sprint card component | ✅ 50.4 |
| `ProjectManagement.Components/Sprints/SprintCard.razor.css` | Sprint card styles | ✅ 50.4 |
| `ProjectManagement.Components/Sprints/SprintDialog.razor` | Sprint dialog | ✅ 50.4 |
| `ProjectManagement.Components/Comments/CommentList.razor` | Comment thread UI | ✅ 50.4 |
| `ProjectManagement.Components/Comments/CommentList.razor.css` | Comment list styles | ✅ 50.4 |
| `ProjectManagement.Components/Comments/CommentEditor.razor` | Comment input | ✅ 50.4 |
| `ProjectManagement.Components/Comments/CommentEditor.razor.css` | Comment editor styles | ✅ 50.4 |
| `pm-ws/tests/sprint_handler_tests.rs` | Sprint handler tests | 50.5 |
| `pm-ws/tests/comment_handler_tests.rs` | Comment handler tests | 50.5 |
| `ProjectManagement.Core.Tests/Converters/SprintConverterTests.cs` | Sprint converter tests | 50.5 |
| `ProjectManagement.Services.Tests/State/CommentStoreTests.cs` | Comment store tests | 50.5 |

### Modify (14 files)

| File | Change | Session |
|------|--------|---------|
| `proto/messages.proto` | Add Sprint/Comment WebSocket messages | ✅ 50.1 |
| `pm-core/src/models/sprint.rs` | Add version field | ✅ 50.1 |
| `pm-db/src/repositories/sprint_repository.rs` | Update queries for version | ✅ 50.1 |
| `pm-ws/src/handlers/response_builder.rs` | Add Sprint/Comment response builders | ✅ 50.1 (Sprint), ✅ 50.2 (Comment) |
| `pm-ws/src/handlers/change_tracker.rs` | Add FieldChangeBuilder | ✅ 50.1 |
| `pm-ws/src/handlers/dispatcher.rs` | Route Sprint/Comment messages | ✅ 50.2 |
| `pm-ws/src/handlers/mod.rs` | Export sprint/comment modules | ✅ 50.1 (sprint), ✅ 50.2 (comment) |
| `pm-ws/src/lib.rs` | Public exports | ✅ 50.2 |
| `pm-ws/src/message_validator.rs` | Sprint/Comment validation | ✅ 50.1 |
| `ProjectManagement.Core/Models/Sprint.cs` | Add Version property | ✅ 50.3 |
| `ProjectManagement.Core/Models/UpdateSprintRequest.cs` | Add ExpectedVersion + Status | ✅ 50.3 |
| `ProjectManagement.Core/Converters/ProtoConverter.cs` | Sprint/Comment conversions | ✅ 50.3 |
| `ProjectManagement.Core/Interfaces/IWebSocketClient.cs` | Sprint/Comment events | ✅ 50.3 |
| `ProjectManagement.Services/WebSocket/WebSocketClient.cs` | Implementation | ✅ 50.3 |
| `ProjectManagement.Services/Resilience/ResilientWebSocketClient.cs` | Decorator forwarding | ✅ 50.3 |
| `ProjectManagement.Services/State/SprintStore.cs` | WebSocket integration | ✅ 50.3 |
| `ProjectManagement.Services.Tests/State/SprintStoreTests.cs` | Mock setup | ✅ 50.3 |
| `ProjectManagement.Wasm/Program.cs` | ICommentStore registration | ✅ 50.4 |

---

## Production-Grade Scoring

| Category | Score | Justification |
|----------|-------|---------------|
| Error Handling | 9.5/10 | Comprehensive errors, validation, conflict detection |
| Validation | 9.5/10 | Input validation, status transitions, business rules |
| Authorization | 9.5/10 | Permission checks, author-only edit/delete for comments |
| Data Integrity | 9.5/10 | Optimistic locking, status state machine, soft deletes |
| Idempotency | 9.5/10 | Message deduplication for create operations |
| Audit Trail | 9.5/10 | Activity logging with field-level tracking |
| Real-time | 9.5/10 | WebSocket events, optimistic updates, conflict resolution |
| Testing | 9.5/10 | Unit tests, integration tests, converter tests |

**Overall Score: 9.5/10**

### What Would Make It 10/10

- Sprint burndown chart with real-time updates
- Comment threading (reply to comments)
- @mention support with notifications
- Rich text editor for comments
- Sprint velocity tracking and predictions

---

## Learning Objectives

Each sub-session teaches specific concepts:

| Session | Key Concepts |
|---------|--------------|
| **50.1** | Protobuf schema evolution, optimistic locking, status state machines |
| **50.2** | Authorization patterns (author-only), message routing, response builders |
| **50.3** | Proto converters, WebSocket client events, state synchronization |
| **50.4** | Blazor state management, optimistic UI updates, real-time collaboration |
| **50.5** | Testing WebSocket handlers, testing state management, mock patterns |

---

## Final Verification

After all five sub-sessions are complete:

```bash
# Full workspace check
just check

# Run all tests
just test

# Build everything
just build-release

# Start app
just dev

# Test features:
# 1. Create a sprint with start/end dates
# 2. Start the sprint (status transition)
# 3. Add comments to work items
# 4. Edit/delete your own comments
# 5. Complete the sprint
```
