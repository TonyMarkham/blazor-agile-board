# Session 90: Work Item Enhancements (JIRA-Style IDs + Flexible Hierarchy)

This session adds three interconnected features to work items:

- **Feature A**: JIRA-style identifiers (e.g., "BAG-1", "BAG-2")
- **Feature B**: Allow orphan Stories/Tasks (no mandatory parent)
- **Feature C**: Edit parent assignment on existing work items

---

## Design Decisions

1. **Compute display key at runtime** - Store only `item_number` (integer). The full key "BAG-123" is computed from `project.key + work_item.item_number`. This avoids redundancy and handles potential project key changes.

2. **Atomic counter per project** - Add `next_work_item_number` to `pm_projects` table. Use transaction isolation to atomically assign numbers.

3. **Numbers start at 1** - Simple integers (1, 2, 3...), not zero-padded.

4. **Hierarchy is optional** - The database and backend already support orphan items. Only the UI currently enforces the hierarchy, so changes are minimal.

5. **Parent editing with validation** - Prevent circular references by walking the parent chain before allowing changes.

---

## Sub-Session Breakdown

| Session | Scope | Est. Tokens | Status |
|---------|-------|-------------|--------|
| **[90.1](90.1-Session-Plan.md)** | Foundation: Database Migration + Protobuf | ~25k | ✅ **COMPLETE** |
| **[90.2](90.2-Session-Plan.md)** | Rust Backend: Models, Repos, Handlers | ~40k | ✅ **COMPLETE** |
| **[90.3](90.3-Session-Plan.md)** | C# Frontend: Models, Converter, ViewModel | ~25k | ✅ **COMPLETE** |
| **[90.4](90.4-Session-Plan.md)** | Blazor UI: All Components | ~35k | Ready |

**Session 90.1 Implementation Notes**:
- Migration includes FK preservation pattern (recreates dependent tables with constraints)
- Transaction wrapper removed (SQLx handles automatically - was causing "transaction within transaction" error)
- Rollback migration skipped (deemed unnecessary)
- Added unplanned compilation fixes to response_builder.rs and work_item_repository.rs
- Migration tests created but in non-standard location
- All builds clean, all tests passing

**Session 90.2 Implementation Notes**:
- Type-safe atomic counter enforces Transaction parameter (prevents race conditions at compile time)
- All error messages include contextual information (work item IDs, project IDs)
- Counter gap behavior documented (expected with transaction rollbacks)
- Circular reference prevention with max depth check (100 levels)
- Three-state parent logic using `update_parent` proto flag
- Orphan items explicitly documented as intentional design
- Test fixtures updated for new fields
- Full backend workspace compiles cleanly, all tests passing

**Session 90.3 Implementation Notes**:
- Critical bug fixed: WebSocketClient was missing ParentId/UpdateParent field mapping
- Created new ProtoConverter.ToProto(UpdateWorkItemRequest) method for consistency
- Refactored WebSocketClient.UpdateWorkItemAsync from 29 lines manual mapping to 1 line converter call
- Enhanced null safety: GetDisplayKey() uses ThrowIfNullOrWhiteSpace (catches whitespace edge case)
- Added 7 new unit tests (4 for WorkItem, 3 for ProtoConverter)
- All proto field mappings verified (ItemNumber, NextWorkItemNumber, UpdateParent)
- Full frontend builds clean, all 473 tests passing

---

## Session 90.1: Foundation (Database + Protobuf)

**Steps:** 1-3
**Files Created:** 1 migration file
**Files Modified:** 1 proto file

This session creates the database migration for `item_number` and `next_work_item_number`, updates the protobuf schema for all three features, and runs the migration.

**Verification:**
```bash
sqlite3 /path/to/dev.db ".schema pm_projects" | grep next_work_item_number
sqlite3 /path/to/dev.db ".schema pm_work_items" | grep item_number
```

---

## Session 90.2: Rust Backend

**Steps:** 4-13
**Files Modified:** 6 Rust files

This session implements:
- Domain models (Project, WorkItem) with new fields
- Repository methods with atomic counter increment
- Response builder updates for proto serialization
- Work item create handler with transactional number assignment
- Change tracker and update handler for parent editing
- Circular reference prevention
- Documentation updates for optional hierarchy

**Verification:**
```bash
just check-backend
just clippy-backend
just test-backend
```

---

## Session 90.3: C# Frontend

**Steps:** 14-18
**Files Modified:** 5 C# files

This session implements:
- C# WorkItem model with `ItemNumber` and `GetDisplayKey()`
- C# Project model with `NextWorkItemNumber`
- C# UpdateWorkItemRequest with `ParentId` and `UpdateParent`
- ProtoConverter updates for new fields
- WorkItemViewModel exposing new properties

**Verification:**
```bash
just build-frontend
just test-frontend
```

---

## Session 90.4: Blazor UI

**Steps:** 19-23
**Files Modified:** 5 Razor files, CSS updates

This session implements:
- KanbanCard showing JIRA-style ID
- KanbanBoard passing ProjectKey
- WorkItemRow with ID column
- WorkItemDetail with ID in header, sidebar, breadcrumb
- WorkItemDialog with:
  - Optional parent selection (Feature B)
  - Parent editing in edit mode (Feature C)
  - Circular reference prevention in UI

**Verification:**
- Manual UI testing
- Accessibility check (ARIA labels)

---

## Files Summary (All Features)

| Step | Layer | File | Feature | Sub-Session |
|------|-------|------|---------|-------------|
| 1 | DB | `migrations/20260203000001_add_work_item_numbers.sql` | A | 90.1 |
| 2 | Proto | `proto/messages.proto` | A, C | 90.1 |
| 3 | DB | **Run migration** | A | 90.1 |
| 4 | Rust | `pm-core/src/models/project.rs` | A | 90.2 |
| 5 | Rust | `pm-core/src/models/work_item.rs` | A | 90.2 |
| 6 | Rust | `pm-db/src/repositories/project_repository.rs` | A | 90.2 |
| 7 | Rust | `pm-db/src/repositories/work_item_repository.rs` | A | 90.2 |
| 8 | Rust | `pm-ws/src/handlers/response_builder.rs` | A | 90.2 |
| 9 | Rust | `pm-ws/src/handlers/work_item.rs` (create) | A | 90.2 |
| 10 | Rust | `pm-ws/src/handlers/change_tracker.rs` | C | 90.2 |
| 11 | Rust | `pm-ws/src/handlers/work_item.rs` (handle_update) | C | 90.2 |
| 12 | Rust | `pm-ws/src/handlers/work_item.rs` (apply_updates) | C | 90.2 |
| 13 | Rust | `pm-ws/src/handlers/hierarchy_validator.rs` | B | 90.2 |
| 14 | C# | `ProjectManagement.Core/Models/WorkItem.cs` | A | 90.3 |
| 15 | C# | `ProjectManagement.Core/Models/Project.cs` | A | 90.3 |
| 16 | C# | `ProjectManagement.Core/Models/UpdateWorkItemRequest.cs` | C | 90.3 |
| 17 | C# | `ProjectManagement.Core/Converters/ProtoConverter.cs` | A | 90.3 |
| 18 | C# | `ProjectManagement.Core/ViewModels/WorkItemViewModel.cs` | A | 90.3 |
| 19 | Blazor | `ProjectManagement.Components/WorkItems/KanbanCard.razor` | A | 90.4 |
| 20 | Blazor | `ProjectManagement.Components/WorkItems/KanbanBoard.razor` | A | 90.4 |
| 21 | Blazor | `ProjectManagement.Components/WorkItems/WorkItemRow.razor` | A | 90.4 |
| 22 | Blazor | `ProjectManagement.Wasm/Pages/WorkItemDetail.razor` | A | 90.4 |
| 23 | Blazor | `ProjectManagement.Components/WorkItems/WorkItemDialog.razor` | B, C | 90.4 |

---

## Pre-Implementation Checklist

Before starting **any** sub-session:

- [ ] `just check` passes (both frontend and backend)
- [ ] Database migrations are current
- [ ] `cargo build -p pm-proto` succeeds

---

## Final Verification

After all sub-sessions are complete:

```bash
# Full build and test
just check

# Manual integration test
# 1. Create a new project with key "TEST"
# 2. Create 3 work items (epic, story, task)
# 3. Verify they get sequential numbers 1, 2, 3
# 4. Verify display as "TEST-1", "TEST-2", "TEST-3"
# 5. Create orphan story (no parent) - should succeed
# 6. Edit story to assign parent - should succeed
# 7. Try to create circular reference - should fail
```

---

## Teaching Notes

This session demonstrates several important patterns:

1. **Full-stack feature implementation** - Same feature touches DB, backend, proto, frontend, UI
2. **SQLite table recreation** - How to add NOT NULL columns without defaults
3. **Atomic counters in SQLite** - Transaction isolation instead of RETURNING
4. **Optional validation** - Only validate when value is provided
5. **Circular reference prevention** - Walking parent chain in both backend and frontend
6. **Proto field numbering** - Adding new fields without breaking compatibility
