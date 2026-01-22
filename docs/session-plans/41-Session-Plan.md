# Session 41: Project as First-Class Entity

**Status**: In Progress (Session 41.1 completed 2026-01-21)
**Production-Grade Score Target**: 9/10

---

## Problem

The "Create Project" button opens `WorkItemDialog`, showing irrelevant fields (Status, Priority, Story Points, Sprint). Root cause: Projects are in `pm_work_items` but are fundamentally different - they're organizational containers, not work items.

**Solution**: Promote Projects to first-class entities with dedicated table, model, repository, handlers, and UI.

---

## Sub-Session Breakdown

This plan has been split into sub-sessions to fit within token budgets:

| Session | Scope | Est. Tokens | Status |
|---------|-------|-------------|--------|
| **[41.1](41.1-Session-Plan.md)** | Backend Database & Models | ~100k actual | ✅ **Completed** |
| **[41.2](41.2-Session-Plan.md)** | Backend Protobuf & Handlers | ~40-50k est, ~120k actual | ✅ **Completed** |
| **[41.3](41.3-Session-Plan.md)** | Frontend Models & WebSocket | ~30-40k | Pending |
| **[41.4](41.4-Session-Plan.md)** | Frontend State & UI | ~40-50k | Pending |

---

## Session 41.1: Backend Database & Models ✅ COMPLETED

**Completion Date**: 2026-01-21
**Actual Token Usage**: ~100k tokens (due to extensive debugging and test updates)

**Files Created (6):**
- `pm-db/migrations/20260121000001_create_projects_table.sql` - Full migration with data migration
- `pm-core/src/models/project.rs` - Project struct
- `pm-core/src/models/project_status.rs` - ProjectStatus enum (split per codebase pattern)
- `pm-db/src/repositories/project_repository.rs` - CRUD operations

**Files Modified (5):**
- `pm-core/src/models/mod.rs` - Export project module
- `pm-core/src/error/mod.rs` - Added InvalidProjectStatus error variant
- `pm-core/src/lib.rs` - Added Project and ProjectStatus to public exports
- `pm-db/src/repositories/mod.rs` - Export ProjectRepository
- `pm-db/src/lib.rs` - Added ProjectRepository to public exports
- `pm-db/tests/common/fixtures.rs` - Updated to return Project instead of WorkItem
- All 6 test files in `pm-db/tests/` - Updated to use ProjectRepository

**Test Results:**
- 146 tests passing across workspace
- All pm-db integration tests updated and passing
- Clean build with no errors

**Key Achievements:**
- Database migration successfully extracts projects from pm_work_items
- Project entity with unique `key` field and Active/Archived status
- Full repository with CRUD operations following existing patterns
- All test fixtures updated to use dedicated ProjectRepository

**Verification:** ✅ `cargo check --workspace && cargo test --workspace` passes

---

## Session 41.2: Backend Protobuf & Handlers ✅ COMPLETED

**Completion Date**: 2026-01-21
**Actual Token Usage**: ~120k tokens (teaching mode session with extensive explanation)

**Files Created (1):**
- `pm-db/tests/project_repository_tests.rs` - Repository tests (10 comprehensive tests)

**Files Modified (5):**
- `proto/messages.proto` - Project messages + Payload variants (fields 90-97)
- `pm-ws/src/lib.rs` - Export project handlers at crate level
- `pm-ws/src/handlers/dispatcher.rs` - Add 4 dispatch arms + handler names
- `pm-ws/src/handlers/response_builder.rs` - Add 4 `build_project_*` functions
- `pm-ws/src/message_validator.rs` - Add `validate_project_create` function

**Test Results:**
- 188 tests passing across workspace (up from 166)
- 10 new project repository tests (find_by_id, find_by_key, find_all, soft delete, etc.)
- Clean workspace build (no warnings in backend crates)

**Key Achievements:**
- Full WebSocket CRUD for Projects (Create/Update/Delete/List)
- Production-grade handlers with idempotency, validation, activity logging
- Optimistic locking with version tracking
- Delete protection (blocks if work items exist)
- Comprehensive repository test coverage using googletest

**Verification:** ✅ `cargo check --workspace && cargo test --workspace` passes

---

## Session 41.3: Frontend Models & WebSocket

**Files Created:**
- `Core/Models/ProjectStatus.cs` - Enum
- `Core/Models/Project.cs` - Record
- `Core/Models/CreateProjectRequest.cs` - DTO
- `Core/Models/UpdateProjectRequest.cs` - DTO

**Files Modified:**
- `Core/Models/WorkItemType.cs` - Remove `Project` enum value
- `Core/Interfaces/IWebSocketClient.cs` - Add Project events + operations
- `Services/WebSocket/WebSocketClient.cs` - Implement Project methods
- `Services/WebSocket/ProtoConverter.cs` - Add Project ↔ proto converters

**Verification:** `dotnet build frontend/ProjectManagement.sln`

---

## Session 41.4: Frontend State & UI

**Files Created:**
- `Core/ViewModels/ProjectViewModel.cs` - Wraps Project
- `Core/Interfaces/IProjectStore.cs` - Interface with async CRUD
- `Services/State/ProjectStore.cs` - Implementation with optimistic updates
- `Components/Projects/ProjectDialog.razor` - Create/edit dialog
- `ProjectManagement.Core.Tests/ViewModels/ProjectViewModelTests.cs`
- `ProjectManagement.Services.Tests/State/ProjectStoreTests.cs`
- `ProjectManagement.Components.Tests/Projects/ProjectDialogTests.cs`

**Files Modified:**
- `Core/ViewModels/ViewModelFactory.cs` - Add `Create(Project)` overload
- `Services/State/AppState.cs` - Add `Projects` property
- `Wasm/Program.cs` - Register `IProjectStore`
- `Wasm/Layout/MainLayout.razor` - Show current project in header
- `Wasm/Pages/ProjectDetail.razor` - Set `CurrentProject` on load
- `Wasm/Pages/Home.razor` - Use Projects store, wire Create button

**Verification:** `dotnet build && dotnet test`

---

## Pre-Implementation Checklist

Before starting **any** sub-session:

- [x] `cargo test --workspace` passes (188 tests after 41.2)
- [ ] `dotnet build frontend/ProjectManagement.sln` succeeds
- [ ] `dotnet test` passes (256+ tests)

---

## Files Summary

### Create (14 files)

| File | Purpose |
|------|---------|
| `pm-db/migrations/..._create_projects.sql` | Database schema + data migration |
| `pm-core/src/models/project.rs` | Rust Project struct + enum |
| `pm-db/src/repositories/project_repository.rs` | Rust CRUD operations |
| `pm-ws/src/handlers/project.rs` | WebSocket handlers |
| `Core/Models/ProjectStatus.cs` | C# enum |
| `Core/Models/Project.cs` | C# record |
| `Core/Models/CreateProjectRequest.cs` | C# DTO |
| `Core/Models/UpdateProjectRequest.cs` | C# DTO |
| `Core/ViewModels/ProjectViewModel.cs` | C# ViewModel |
| `Core/Interfaces/IProjectStore.cs` | C# interface |
| `Services/State/ProjectStore.cs` | C# state management |
| `Components/Projects/ProjectDialog.razor` | Blazor dialog |
| `pm-db/tests/project_repository_tests.rs` | Backend tests |
| `pm-ws/tests/project_handler_tests.rs` | Backend tests |

### Modify (14 files)

| File | Change | Status |
|------|--------|--------|
| `proto/messages.proto` | Add Project messages + Payload variants | ✅ 41.2 |
| `pm-core/src/models/mod.rs` | Export project module | ✅ 41.1 |
| `pm-db/src/repositories/mod.rs` | Export ProjectRepository | ✅ 41.1 |
| `pm-ws/src/lib.rs` | Export project handlers | ✅ 41.2 |
| `pm-ws/src/handlers/dispatcher.rs` | Add dispatch arms | ✅ 41.2 |
| `pm-ws/src/handlers/response_builder.rs` | Add build_project_* functions | ✅ 41.2 |
| `pm-ws/src/message_validator.rs` | Add validate_project_create function | ✅ 41.2 |
| `Core/Models/WorkItemType.cs` | Remove Project enum value | Pending |
| `Core/Interfaces/IWebSocketClient.cs` | Add Project events + operations | Pending |
| `Services/WebSocket/WebSocketClient.cs` | Implement Project methods | Pending |
| `Services/WebSocket/ProtoConverter.cs` | Add Project converters | Pending |
| `Core/ViewModels/ViewModelFactory.cs` | Add Create(Project) overload | Pending |
| `Services/State/AppState.cs` | Add Projects property | Pending |
| `Wasm/Program.cs` | Register IProjectStore | Pending |

---

## Production-Grade Scoring

| Category | Score | Justification |
|----------|-------|---------------|
| Error Handling | 9.5/10 | Comprehensive errors, From impls, validation |
| Validation | 9.5/10 | Title length, key format, description limits |
| Data Integrity | 9.5/10 | Transactions, optimistic locking, soft delete |
| Idempotency | 9.5/10 | Message deduplication via idempotency check |
| Audit Trail | 9.5/10 | Activity logging with field-level tracking |
| Testing | 9.0/10 | Repository + handler + component tests |
| Thread Safety | 9.5/10 | ConcurrentDictionary, optimistic updates |
| UX | 9.0/10 | Auto-generated key, header context, optimistic UI |

**Overall Score: 9/10**

### What Would Make It 10/10

- Property-based tests for handlers
- Integration tests with real WebSocket connection
- Accessibility testing for ProjectDialog
- Performance benchmarks for ProjectStore

---

## Final Verification

After all sub-sessions are complete:

```bash
# Backend
cargo check --workspace
cargo test --workspace
cargo clippy --workspace -- -D warnings

# Frontend
dotnet build frontend/ProjectManagement.sln
dotnet test

# Manual E2E
# 1. Create project via UI → appears in list
# 2. Edit project title → updates in header
# 3. Archive project → removed from active list
# 4. Delete project with work items → blocked with error message
```
