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

| Session | Scope | Est. Tokens | Status |
|---------|-------|-------------|--------|
| **[50.1](50.1-Session-Plan.md)** | Proto Schema + Backend Sprint Infrastructure | ~35-40k | Pending |
| **[50.2](50.2-Session-Plan.md)** | Backend Comment Handler + Response Builders | ~35-40k | Pending |
| **[50.3](50.3-Session-Plan.md)** | Frontend Models + WebSocket Integration | ~40-45k | Pending |
| **[50.4](50.4-Session-Plan.md)** | State Management + UI Components | ~40-45k | Pending |
| **[50.5](50.5-Session-Plan.md)** | Testing (Backend + Frontend) | ~35-40k | Pending |

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

## Session 50.2: Backend Comment Handler + Response Builders

**Files Created:**
- `pm-ws/src/handlers/comment.rs` - Comment CRUD handlers

**Files Modified:**
- `pm-ws/src/handlers/response_builder.rs` - Sprint/Comment response builders
- `pm-ws/src/handlers/dispatcher.rs` - Route Sprint/Comment messages
- `pm-ws/src/handlers/mod.rs` - Export new modules
- `pm-ws/src/lib.rs` - Public exports
- `pm-ws/src/message_validator.rs` - Sprint/Comment validation

**Verification:** `just check-rs-ws && just test-rs-ws`

---

## Session 50.3: Frontend Models + WebSocket Integration

**Files Modified:**
- `ProjectManagement.Core/Models/Sprint.cs` - Add Version property
- `ProjectManagement.Core/Models/UpdateSprintRequest.cs` - Add ExpectedVersion

**Files Created:**
- `ProjectManagement.Core/Models/Comment.cs` - Comment model
- `ProjectManagement.Core/Models/CreateCommentRequest.cs` - Create request
- `ProjectManagement.Core/Models/UpdateCommentRequest.cs` - Update request

**Files Modified:**
- `ProjectManagement.Core/Converters/ProtoConverter.cs` - Sprint/Comment conversions
- `ProjectManagement.Core/Interfaces/IWebSocketClient.cs` - Sprint/Comment events and operations
- `ProjectManagement.Services/WebSocket/WebSocketClient.cs` - Implementation
- `ProjectManagement.Services/State/SprintStore.cs` - WebSocket integration

**Verification:** `just build-frontend && just test-frontend`

---

## Session 50.4: State Management + UI Components

**Files Created:**
- `ProjectManagement.Core/Interfaces/ICommentStore.cs` - Comment store interface
- `ProjectManagement.Services/State/CommentStore.cs` - Comment state management
- `ProjectManagement.Components/Sprint/SprintCard.razor` - Sprint card component
- `ProjectManagement.Components/Sprint/SprintDialog.razor` - Create/edit sprint dialog
- `ProjectManagement.Components/Comments/CommentList.razor` - Comment thread UI
- `ProjectManagement.Components/Comments/CommentEditor.razor` - Comment input

**Verification:** `just build-frontend && just test-frontend`

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

### Create (15 files)

| File | Purpose |
|------|---------|
| `pm-ws/src/handlers/sprint.rs` | Sprint CRUD handlers |
| `pm-ws/src/handlers/comment.rs` | Comment CRUD handlers |
| `ProjectManagement.Core/Models/Comment.cs` | Comment domain model |
| `ProjectManagement.Core/Models/CreateCommentRequest.cs` | Create comment request |
| `ProjectManagement.Core/Models/UpdateCommentRequest.cs` | Update comment request |
| `ProjectManagement.Core/Interfaces/ICommentStore.cs` | Comment store interface |
| `ProjectManagement.Services/State/CommentStore.cs` | Comment state management |
| `ProjectManagement.Components/Sprint/SprintCard.razor` | Sprint card component |
| `ProjectManagement.Components/Sprint/SprintDialog.razor` | Sprint dialog |
| `ProjectManagement.Components/Comments/CommentList.razor` | Comment thread UI |
| `ProjectManagement.Components/Comments/CommentEditor.razor` | Comment input |
| `pm-ws/tests/sprint_handler_tests.rs` | Sprint handler tests |
| `pm-ws/tests/comment_handler_tests.rs` | Comment handler tests |
| `ProjectManagement.Core.Tests/Converters/SprintConverterTests.cs` | Sprint converter tests |
| `ProjectManagement.Services.Tests/State/CommentStoreTests.cs` | Comment store tests |

### Modify (14 files)

| File | Change |
|------|--------|
| `proto/messages.proto` | Add Sprint/Comment WebSocket messages |
| `pm-core/src/models/sprint.rs` | Add version field |
| `pm-db/src/repositories/sprint_repository.rs` | Update queries for version |
| `pm-ws/src/handlers/response_builder.rs` | Add Sprint/Comment response builders |
| `pm-ws/src/handlers/dispatcher.rs` | Route Sprint/Comment messages |
| `pm-ws/src/handlers/mod.rs` | Export sprint/comment modules |
| `pm-ws/src/lib.rs` | Public exports |
| `pm-ws/src/message_validator.rs` | Sprint/Comment validation |
| `ProjectManagement.Core/Models/Sprint.cs` | Add Version property |
| `ProjectManagement.Core/Models/UpdateSprintRequest.cs` | Add ExpectedVersion + Status |
| `ProjectManagement.Core/Converters/ProtoConverter.cs` | Sprint/Comment conversions |
| `ProjectManagement.Core/Interfaces/IWebSocketClient.cs` | Sprint/Comment events |
| `ProjectManagement.Services/WebSocket/WebSocketClient.cs` | Implementation |
| `ProjectManagement.Services/State/SprintStore.cs` | WebSocket integration |

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
