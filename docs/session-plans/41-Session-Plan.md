# Session 41: Project as First-Class Entity

**Status**: Planning (2026-01-21)
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
| **[41.1](41.1-Session-Plan.md)** | Backend Database & Models | ~30-40k | Pending |
| **[41.2](41.2-Session-Plan.md)** | Backend Protobuf & Handlers | ~40-50k | Pending |
| **[41.3](41.3-Session-Plan.md)** | Frontend Models & WebSocket | ~30-40k | Pending |
| **[41.4](41.4-Session-Plan.md)** | Frontend State & UI | ~40-50k | Pending |

---

## Session 41.1: Backend Database & Models

**Files Created:**
- `pm-db/migrations/YYYYMMDDHHMMSS_create_projects_table.sql` - Full migration with data migration
- `pm-core/src/models/project.rs` - Project struct + ProjectStatus enum
- `pm-db/src/repositories/project_repository.rs` - CRUD operations

**Files Modified:**
- `pm-core/src/models/mod.rs` - Export project module
- `pm-db/src/repositories/mod.rs` - Export ProjectRepository

**Verification:** `cargo check --workspace && cargo test -p pm-db`

---

## Session 41.2: Backend Protobuf & Handlers

**Files Created:**
- `pm-ws/src/handlers/project.rs` - Create/Update/Delete/List handlers
- `pm-db/tests/project_repository_tests.rs` - Repository tests
- `pm-ws/tests/project_handler_tests.rs` - Handler tests

**Files Modified:**
- `proto/messages.proto` - Project messages + Payload variants
- `pm-ws/src/handlers/mod.rs` - Export project module
- `pm-ws/src/handlers/dispatcher.rs` - Add dispatch arms
- `pm-ws/src/handlers/response_builder.rs` - Add `build_project_*` functions
- `pm-ws/src/handlers/validation.rs` - Add `validate_key` function

**Verification:** `cargo check --workspace && cargo test --workspace`

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

- [ ] `cargo test --workspace` passes (166+ tests)
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

| File | Change |
|------|--------|
| `proto/messages.proto` | Add Project messages + Payload variants |
| `pm-core/src/models/mod.rs` | Export project module |
| `pm-db/src/repositories/mod.rs` | Export ProjectRepository |
| `pm-ws/src/handlers/mod.rs` | Export project module |
| `pm-ws/src/handlers/dispatcher.rs` | Add dispatch arms |
| `pm-ws/src/handlers/response_builder.rs` | Add build_project_* functions |
| `pm-ws/src/handlers/validation.rs` | Add validate_key function |
| `Core/Models/WorkItemType.cs` | Remove Project enum value |
| `Core/Interfaces/IWebSocketClient.cs` | Add Project events + operations |
| `Services/WebSocket/WebSocketClient.cs` | Implement Project methods |
| `Services/WebSocket/ProtoConverter.cs` | Add Project converters |
| `Core/ViewModels/ViewModelFactory.cs` | Add Create(Project) overload |
| `Services/State/AppState.cs` | Add Projects property |
| `Wasm/Program.cs` | Register IProjectStore |

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
