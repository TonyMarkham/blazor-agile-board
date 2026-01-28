# Session 60: Time Tracking & Dependencies

## Overview

This session implements time tracking (running timers + manual entries) and dependency management (blocking relationships with cycle detection). Both features require full-stack implementation from protocol definition through UI components.

**Existing Infrastructure (Ready to Use):**
- Database: `pm_time_entries`, `pm_dependencies` tables with migrations
- Rust Models: `TimeEntry`, `Dependency`, `DependencyType` in pm-core
- Repositories: `TimeEntryRepository`, `DependencyRepository` in pm-db
- Proto Entities: `TimeEntry`, `Dependency` message definitions (but NOT WebSocket commands/events)

---

## Sub-Session Breakdown

This plan has been split into sub-sessions to fit within token budgets:

| Session | Scope | Est. Tokens | Status |
|---------|-------|-------------|--------|
| **[60.1](60.1-Session-Plan.md)** | Protocol Definition & Backend Infrastructure | ~40-50k | ✅ Complete (2026-01-27) |
| **[60.2](60.2-Session-Plan.md)** | Backend Handlers (Time Entry & Dependency) | ~45-55k | Pending |
| **[60.3](60.3-Session-Plan.md)** | Frontend Models & WebSocket Integration | ~35-45k | Pending |
| **[60.4](60.4-Session-Plan.md)** | Frontend State Management & UI Components | ~40-50k | Pending |
| **[60.5](60.5-Session-Plan.md)** | Tests & Integration Verification | ~35-45k | Pending |

---

## Business Rules (Critical)

### Time Tracking Rules
1. **One active timer per user** - Starting a new timer auto-stops any existing timer
2. **Owner-only mutations** - Only the user who created a time entry can edit/delete it
3. **Atomicity** - StartTimer must be atomic (check-stop-create in single transaction)
4. **UTC timestamps** - All timestamps stored and transmitted as UTC Unix seconds
5. **Duration calculated on stop** - `duration_seconds = ended_at - started_at`
6. **Soft deletes** - All queries filter `WHERE deleted_at IS NULL`
7. **Max description** - 1000 characters
8. **Max duration** - 24 hours (86400 seconds) for manual entries
9. **No future timestamps** - started_at/ended_at cannot be in the future (60s tolerance for clock drift)

### Dependency Rules
1. **No self-reference** - Item cannot block itself
2. **No duplicates** - Same (blocking, blocked) pair cannot exist twice
3. **No cycles for Blocks type** - A→B→C→A detected and rejected with path in error
4. **RelatesTo allows bidirectional** - A relates_to B and B relates_to A is valid
5. **Same-project only** - Dependencies can only be created between items in the same project
6. **Edit permission required** - Must have Edit on the project to create/delete dependencies
7. **Soft deletes** - All queries filter `WHERE deleted_at IS NULL`
8. **Max dependencies** - 50 blocking + 50 blocked per item (prevent graph explosion)

---

## Session 60.1: Protocol & Backend Infrastructure ✅

**Status**: Complete (2026-01-27)

**Files Created:**
- None (modifications only)

**Files Modified:**
- `proto/messages.proto` - Added 20+ message types (138 lines)
- `pm-config/src/validation_config.rs` - Added 7 validation constants (17 lines)
- `pm-ws/src/handlers/message_validator.rs` - Added 3 validation methods (76 lines)
- `pm-ws/src/handlers/response_builder.rs` - Added 2 converters + 10 builders (207 lines)
- `pm-db/src/repositories/time_entry_repository.rs` - Added pagination (88 lines)
- `pm-db/src/repositories/dependency_repository.rs` - Added 5 helper methods (92 lines)

**Total**: 6 files modified, ~618 lines added

**Verification:** ✅ `just check-backend` passes

---

## Session 60.2: Backend Handlers

**Files Created:**
- `pm-ws/src/handlers/time_entry.rs` - 7 handlers with atomic timer operations
- `pm-ws/src/handlers/dependency.rs` - 3 handlers with cycle detection

**Files Modified:**
- `pm-ws/src/handlers/dispatcher.rs` - Wire 10 new handlers to dispatcher
- `pm-ws/src/handlers/mod.rs` - Export new handler modules

**Verification:** `just check-backend && just clippy-backend`

---

## Session 60.3: Frontend Models & WebSocket

**Files Created:**
- `frontend/ProjectManagement.Core/Models/TimeEntry.cs` - Domain model
- `frontend/ProjectManagement.Core/Models/Dependency.cs` - Domain model + enum
- `frontend/ProjectManagement.Core/Models/TimeEntryRequests.cs` - Request DTOs
- `frontend/ProjectManagement.Core/Models/DependencyRequests.cs` - Request DTOs

**Files Modified:**
- `frontend/ProjectManagement.Core/Converters/ProtoConverter.cs` - Add TimeEntry/Dependency converters
- `frontend/ProjectManagement.Services/WebSocket/IWebSocketClient.cs` - Add 10 operations + events
- `frontend/ProjectManagement.Services/WebSocket/WebSocketClient.cs` - Implement operations

**Verification:** `just build-frontend`

---

## Session 60.4: Frontend State & UI

**Files Created:**
- `frontend/ProjectManagement.Core/Interfaces/ITimeEntryStore.cs` - Store interface
- `frontend/ProjectManagement.Core/Interfaces/IDependencyStore.cs` - Store interface
- `frontend/ProjectManagement.Services/State/TimeEntryStore.cs` - Store implementation
- `frontend/ProjectManagement.Services/State/DependencyStore.cs` - Store implementation
- `frontend/ProjectManagement.Components/TimeTracking/time-tracking.css` - Timer/entry styles
- `frontend/ProjectManagement.Components/Dependencies/dependencies.css` - Dependency styles
- `frontend/ProjectManagement.Components/TimeTracking/TimerWidget.razor` - Timer widget
- `frontend/ProjectManagement.Components/TimeTracking/TimeEntryList.razor` - Entry list
- `frontend/ProjectManagement.Components/TimeTracking/TimeEntryDialog.razor` - Create/edit dialog
- `frontend/ProjectManagement.Components/Dependencies/DependencyManager.razor` - Dependency UI
- `frontend/ProjectManagement.Components/Dependencies/BlockedIndicator.razor` - Blocked badge
- `frontend/ProjectManagement.Components/Dependencies/AddDependencyDialog.razor` - Add dialog

**Files Modified:**
- `frontend/ProjectManagement.Wasm/Program.cs` - Register new stores

**Verification:** `just build-frontend`

---

## Session 60.5: Tests & Integration

**Files Created:**
- `backend/crates/pm-ws/tests/time_entry_handler_tests.rs` - 10 handler tests
- `backend/crates/pm-ws/tests/dependency_handler_tests.rs` - 10 handler tests
- `frontend/ProjectManagement.Core.Tests/Converters/TimeEntryConverterTests.cs` - Converter tests
- `frontend/ProjectManagement.Core.Tests/Converters/DependencyConverterTests.cs` - Converter tests
- `frontend/ProjectManagement.Services.Tests/State/TimeEntryStoreTests.cs` - Store tests
- `frontend/ProjectManagement.Services.Tests/State/DependencyStoreTests.cs` - Store tests

**Verification:** `just test`

---

## Pre-Implementation Checklist

Before starting **any** sub-session:

- [ ] `just check` passes (all code compiles)
- [ ] `just test` passes (all existing tests green)
- [ ] Database has `pm_time_entries` and `pm_dependencies` tables (run migrations if needed)

---

## Files Summary

### Create (22 files)

| File | Purpose |
|------|---------|
| `pm-ws/src/handlers/time_entry.rs` | Time entry handlers with atomic timer |
| `pm-ws/src/handlers/dependency.rs` | Dependency handlers with cycle detection |
| `TimeEntry.cs` | C# domain model |
| `Dependency.cs` | C# domain model + DependencyType enum |
| `TimeEntryRequests.cs` | C# request DTOs |
| `DependencyRequests.cs` | C# request DTOs |
| `ITimeEntryStore.cs` | Store interface |
| `IDependencyStore.cs` | Store interface |
| `TimeEntryStore.cs` | Optimistic update store |
| `DependencyStore.cs` | Dependency state store |
| `time-tracking.css` | Timer widget styles |
| `dependencies.css` | Dependency UI styles |
| `TimerWidget.razor` | Start/stop timer component |
| `TimeEntryList.razor` | Paginated entry list |
| `TimeEntryDialog.razor` | Manual entry dialog |
| `DependencyManager.razor` | Blocking/blocked lists |
| `BlockedIndicator.razor` | "Blocked" badge |
| `AddDependencyDialog.razor` | Work item search dialog |
| `time_entry_handler_tests.rs` | Backend time entry tests |
| `dependency_handler_tests.rs` | Backend dependency tests |
| `TimeEntryConverterTests.cs` | Proto converter tests |
| `DependencyConverterTests.cs` | Proto converter tests |
| `TimeEntryStoreTests.cs` | Store unit tests |
| `DependencyStoreTests.cs` | Store unit tests |

### Modify (12 files)

| File | Change |
|------|--------|
| `proto/messages.proto` | Add 20+ message types |
| `pm-config/src/validation_config.rs` | Add validation constants |
| `pm-ws/src/handlers/message_validator.rs` | Add validation methods |
| `pm-ws/src/handlers/response_builder.rs` | Add converters + builders |
| `pm-ws/src/handlers/dispatcher.rs` | Wire 10 new handlers |
| `pm-ws/src/handlers/mod.rs` | Export new modules |
| `pm-db/src/repositories/time_entry_repository.rs` | Add pagination |
| `pm-db/src/repositories/dependency_repository.rs` | Add helper methods |
| `ProtoConverter.cs` | Add TimeEntry/Dependency converters |
| `IWebSocketClient.cs` | Add operations + events |
| `WebSocketClient.cs` | Implement operations |
| `Program.cs` | Register stores |

---

## Success Criteria

### Time Tracking
- [ ] Only ONE running timer per user (atomic check-stop-create)
- [ ] Starting new timer auto-stops previous with notification
- [ ] Manual time entry creation with timestamp validation
- [ ] Owner-only edit/delete for time entries
- [ ] Pagination for time entries list (default 100, max 500)
- [ ] Max duration validation (24 hours)
- [ ] No future timestamps (60s tolerance)

### Dependencies
- [ ] Self-referential dependency rejected
- [ ] Circular dependency detected with path in error message
- [ ] Duplicate dependency rejected
- [ ] Same-project only for dependencies
- [ ] Max 50 blocking + 50 blocked per item enforced
- [ ] BlockedIndicator shows on blocked items

### Infrastructure
- [ ] Activity logging for all mutations
- [ ] Soft delete filtering (`deleted_at IS NULL`) in all queries
- [ ] UTC timestamps throughout
- [ ] Broadcast events to other connected clients
- [ ] Running timer state recovery on reconnect

### Quality
- [ ] All existing tests still pass
- [ ] 35+ new tests passing (20 backend, 15 frontend)
- [ ] `just clippy-backend` clean
- [ ] CSS styling for all new components

---

## Final Verification

After all five sub-sessions are complete:

```bash
# Full workspace check
just check

# Run all tests
just test

# Lint check
just lint

# Build and run
just dev

# Manual Integration Test:
# 1. Start timer on work item A → timer widget shows elapsed time
# 2. Start timer on work item B → A's timer auto-stops, B starts
# 3. Stop B's timer → duration calculated and displayed
# 4. Create manual time entry → appears in list
# 5. Create dependency: A blocks B → appears in dependency manager
# 6. Try B blocks A → circular dependency error with path
# 7. BlockedIndicator appears on B in Kanban board
# 8. Delete dependency → indicator disappears
```
