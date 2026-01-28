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
| **[60.2](60.2-Session-Plan.md)** | Backend Handlers (Time Entry & Dependency) | ~45-55k | ✅ Complete (2026-01-27) |
| **[60.3](60.3-Session-Plan.md)** | Frontend Models & WebSocket Integration | ~35-45k | ✅ Complete (2026-01-27) |
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

## Session 60.2: Backend Handlers ✅

**Status**: Complete (2026-01-27)

**Files Created (2):**
- `pm-ws/src/handlers/time_entry.rs` - 7 handlers with atomic timer operations (582 lines)
- `pm-ws/src/handlers/dependency.rs` - 3 handlers with BFS cycle detection (456 lines)

**Files Modified (4):**
- `pm-config/src/lib.rs` - Exported MAX_BLOCKING/BLOCKED_DEPENDENCIES constants
- `pm-ws/src/handlers/dispatcher.rs` - Added 10 handler dispatch cases
- `pm-ws/src/handlers/mod.rs` - Exported time_entry and dependency modules
- `pm-ws/tests/sprint_handler_tests.rs` - Fixed clippy warning (as_deref)

**What Was Delivered:**
- ✅ Atomic timer operations (check-stop-create in single transaction)
- ✅ Owner-only mutations for time entries (update, delete)
- ✅ BFS cycle detection with path reconstruction
- ✅ Complete business rule validation (self-ref, same-project, duplicates, limits)
- ✅ Activity logging for all mutations
- ✅ Idempotency support
- ✅ Permission checks (Edit for mutations, View for queries)
- ✅ Soft deletes throughout

**Verification:** ✅ `just check-backend && just clippy-backend && just test-backend` (219 tests passing, 0 warnings)

---

## Session 60.3: Frontend Models & WebSocket ✅

**Status**: Complete (2026-01-27)

**Files Created (7):**
- `frontend/ProjectManagement.Core/Models/TimeEntry.cs` - Domain model with computed properties
- `frontend/ProjectManagement.Core/Models/Dependency.cs` - Domain model for dependency relationships
- `frontend/ProjectManagement.Core/Models/DependencyType.cs` - Enum for Blocks/RelatesTo (separated for clarity)
- `frontend/ProjectManagement.Core/Models/StartTimerRequest.cs` - Request DTO for starting timer
- `frontend/ProjectManagement.Core/Models/CreateTimeEntryRequest.cs` - Request DTO for manual entry
- `frontend/ProjectManagement.Core/Models/UpdateTimeEntryRequest.cs` - Request DTO for updating entry
- `frontend/ProjectManagement.Core/Models/CreateDependencyRequest.cs` - Request DTO for creating dependency

**Files Modified (4):**
- `frontend/ProjectManagement.Core/Converters/ProtoConverter.cs` - Added 6 conversion methods (TimeEntry ↔ Proto, Dependency ↔ Proto, enum conversions)
- `frontend/ProjectManagement.Core/Interfaces/IWebSocketClient.cs` - Added 10 operations + 7 events
- `frontend/ProjectManagement.Services/WebSocket/WebSocketClient.cs` - Implemented event declarations, 10 operations, 7 broadcast handlers
- `frontend/ProjectManagement.Services/Resilience/ResilientWebSocketClient.cs` - Added event pass-throughs + operation wrappers

**What Was Delivered:**
- ✅ Domain models with computed properties (`IsRunning`, `Elapsed`, `ElapsedFormatted`)
- ✅ Request DTOs following existing codebase pattern (individual files)
- ✅ Proto converters with proper null checks for optional message fields
- ✅ WebSocket operations matching backend protocol exactly
- ✅ Broadcast event handlers for real-time updates
- ✅ Tuple return types for multi-value operations (`StartTimerAsync`)
- ✅ Resilience layer integration

**Verification:** ✅ `just build-frontend && just test-frontend` passes

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

### Create (25 files)

| File | Purpose |
|------|---------|
| `pm-ws/src/handlers/time_entry.rs` | Time entry handlers with atomic timer |
| `pm-ws/src/handlers/dependency.rs` | Dependency handlers with cycle detection |
| `TimeEntry.cs` | C# domain model with computed properties |
| `Dependency.cs` | C# domain model for dependency relationships |
| `DependencyType.cs` | C# enum for Blocks/RelatesTo |
| `StartTimerRequest.cs` | C# request DTO for starting timer |
| `CreateTimeEntryRequest.cs` | C# request DTO for manual time entry |
| `UpdateTimeEntryRequest.cs` | C# request DTO for updating time entry |
| `CreateDependencyRequest.cs` | C# request DTO for creating dependency |
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

### Modify (13 files)

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
| `ProtoConverter.cs` | Add TimeEntry/Dependency converters (6 methods) |
| `IWebSocketClient.cs` | Add operations + events (10 ops + 7 events) |
| `WebSocketClient.cs` | Implement operations + event handlers |
| `ResilientWebSocketClient.cs` | Add resilience wrappers for new operations |
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
