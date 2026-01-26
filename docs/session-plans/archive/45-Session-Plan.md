# Session 45: Kanban Drag-and-Drop Fix + Type-Specific Cards

## Scope

This session fixes two Kanban board usability issues:

1. **Drag-and-Drop Reliability** - HTML5 drag-and-drop events don't fire reliably in Blazor WASM. Replace with Radzen's `RadzenDropZoneContainer` / `RadzenDropZone` components.

2. **Visual Hierarchy** - All work item types (Epic/Story/Task) render identically. Add type-specific card rendering with progress bars showing child item completion.

---

## Critical Design Constraint

**No JavaScript** - All functionality must be pure Blazor/C#. JS interop is forbidden except where absolutely required by WASM runtime.

---

## Sub-Session Breakdown

This plan has been split into sub-sessions to fit within token budgets:

| Session | Scope | Est. Tokens | Status |
|---------|-------|-------------|--------|
| **[45.1](45.1-Session-Plan.md)** | Radzen Drag-and-Drop Integration | ~35-40k | ✅ **Complete** (2026-01-25) |
| **[45.2](45.2-Session-Plan.md)** | Type-Specific Cards with Progress Bars | ~30-35k | ✅ **Complete** (2026-01-26) |

**Session 45 Status**: ✅ **FULLY COMPLETE**

**Notes:**
- Session 45.1 completed successfully but differently than planned. The root cause was Tauri's `dragDropEnabled` config, not Blazor/Radzen implementation. See 45.1 plan for actual implementation details.
- Session 45.2 completed with all visual hierarchy features working as specified. Epic cards show both Story and Task progress, Story cards show Task progress, Task cards remain simple.

---

## Session 45.1: Radzen Drag-and-Drop Integration

**Steps:**
1. Add CSS for Radzen drag states
2. Add CardTemplate infrastructure (RenderFragment)
3. Add RadzenDropZoneContainer to KanbanBoard
4. Restructure KanbanColumn with RadzenDropZone
5. Remove unused drag callbacks
6. Simplify KanbanCard (remove HTML5 drag)
7. Update tests (delete 6 drag tests, update 1 param)

**Files Modified (4):**
- `ProjectManagement.Components/wwwroot/css/kanban.css`
- `ProjectManagement.Components/WorkItems/KanbanBoard.razor`
- `ProjectManagement.Components/WorkItems/KanbanColumn.razor`
- `ProjectManagement.Components/WorkItems/KanbanCard.razor`

**Files Modified - Tests (2):**
- `ProjectManagement.Components.Tests/WorkItems/KanbanCardTests.cs`
- `ProjectManagement.Components.Tests/WorkItems/KanbanBoardTests.cs`

**Key Concepts:**
- `RadzenDropZoneContainer<TItem>` wraps all columns and provides `ItemSelector` and `Drop` callbacks
- `RadzenDropZone<TItem>` replaces each column's body, rendering items via `Template`
- Column header stays outside the DropZone
- Keyboard drag preserved via existing `_draggedItem` state

**Verification:** `just build-cs-components && just test-cs-components`

---

## Session 45.2: Type-Specific Cards with Progress Bars

**Steps:**
8. Create ChildProgress model
9. Update WorkItemViewModel with progress properties
10. Update ViewModelFactory to compute progress
11. Create ChildProgressBar component
12. Add progress bar CSS
13. Update KanbanCard with conditional progress section
14. Add progress bar tests

**Files Created (2):**
- `ProjectManagement.Core/ViewModels/ChildProgress.cs`
- `ProjectManagement.Components/WorkItems/ChildProgressBar.razor`

**Files Modified (4):**
- `ProjectManagement.Core/ViewModels/WorkItemViewModel.cs`
- `ProjectManagement.Services/State/ViewModelFactory.cs`
- `ProjectManagement.Components/wwwroot/css/kanban.css`
- `ProjectManagement.Components/WorkItems/KanbanCard.razor`

**Files Modified - Tests (1):**
- `ProjectManagement.Components.Tests/WorkItems/KanbanCardTests.cs`

**Key Concepts:**
- `ChildProgress` record tracks item counts by status
- `ViewModelFactory` computes progress from cached work items
- Progress bar shows colored segments per status (swimlane visualization)
- Epic cards show both Story progress AND Task progress bars
- Story cards show Task progress only; Task cards show no progress

**Verification:** `just build-cs-components && just test-cs-components`

---

## Pre-Implementation Checklist

Before starting **any** sub-session:

- [ ] `just check-frontend` passes
- [ ] `just test-cs-components` passes
- [ ] Application runs: `just dev`
- [ ] Existing Kanban board displays work items

---

## Files Summary

### Create (2 files)

| File | Purpose |
|------|---------|
| `ProjectManagement.Core/ViewModels/ChildProgress.cs` | Progress tracking model for Epic/Story cards |
| `ProjectManagement.Components/WorkItems/ChildProgressBar.razor` | Reusable swimlane progress bar component |

### Modify (8 files)

| File | Changes |
|------|---------|
| `ProjectManagement.Components/wwwroot/css/kanban.css` | Radzen DropZone styles + progress bar styles |
| `ProjectManagement.Components/WorkItems/KanbanBoard.razor` | Add `RadzenDropZoneContainer`, drop handler, CardTemplate |
| `ProjectManagement.Components/WorkItems/KanbanColumn.razor` | Restructure: header outside, DropZone = body |
| `ProjectManagement.Components/WorkItems/KanbanCard.razor` | Remove HTML5 drag, add conditional progress section |
| `ProjectManagement.Core/ViewModels/WorkItemViewModel.cs` | Add `ChildProgress` computed properties |
| `ProjectManagement.Services/State/ViewModelFactory.cs` | Compute child progress from AppState cache |
| `ProjectManagement.Components.Tests/.../KanbanCardTests.cs` | Delete 6 drag tests, add 4 progress bar tests |
| `ProjectManagement.Components.Tests/.../KanbanBoardTests.cs` | Update 1 test (parameter rename) |

---

## Card Design Specification

```
┌─────────────────────────────────────────────────────────┐
│ EPIC CARD                                               │
│ ┌─────────────────────────────────────────────────────┐ │
│ │ 📦 Epic: User Authentication System        🔴 High  │ │
│ │ Stories [▓▓▓▓▓▓▓▓░░░░░░░░░░░░░░░░░░░░░░] 3/12      │ │
│ │ Tasks   [▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓░░░░░░░░░░░░░░░] 8/20 [✏️] │ │
│ └─────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────┐
│ STORY CARD                                              │
│ ┌─────────────────────────────────────────────────────┐ │
│ │ 📖 Story: Login Form                       🟡 Med   │ │
│ │ Tasks [▓▓░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░] 2/8  [✏️] │ │
│ │        ↑backlog ↑todo ↑progress ↑review ↑done       │ │
│ └─────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────┐
│ TASK CARD (simplest - no progress bar)                  │
│ ┌─────────────────────────────────────────────────────┐ │
│ │ ✓ Task: Implement validation             🔴 High   │ │
│ │                                          3 pts [✏️] │ │
│ └─────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────┘
```

**Progress Bar Colors (matching Kanban columns):**
- Backlog: `var(--rz-base-400)` (gray)
- To Do: `var(--rz-info)` (blue)
- In Progress: `var(--rz-warning)` (amber)
- Review: `var(--rz-secondary)` (purple)
- Done: `var(--rz-success)` (green)

---

## Key Architecture Decisions

### Drag-and-Drop (Session 45.1)

1. **Header outside, DropZone = body** - Column header renders outside DropZone; the DropZone itself becomes the scrollable body containing cards

2. **KanbanCard via Template** - Rendered by Radzen's Template parameter, preserving all card UI (click, edit, accessibility)

3. **Hybrid keyboard support** - Mouse drag via Radzen, keyboard drag via existing custom code (`_draggedItem` state preserved)

4. **Draggability via Attributes** - Use `args.Attributes["draggable"] = "false"` in ItemRender (no dedicated property)

### Type-Specific Cards (Session 45.2)

5. **Single KanbanCard component** - Keep one `KanbanCard.razor` with conditional `@if` blocks rather than multiple components
   - **Rationale:** 80% of card code is shared; Radzen DropZone Template expects uniform type; simpler mental model

6. **ChildProgress computed from AppState cache** - No extra backend queries; all project work items already loaded

7. **Swimlane progress bar** - Progress bar shows colored segments for each status, not just "percent complete"
   - **Rationale:** Visual feedback about WHERE work is stuck (e.g., items in "review" = bottleneck)

8. **Epic tracks grandchildren** - Epic's task progress includes tasks under its Stories
   - **Rationale:** Reflects true completion state; users want "how much of this Epic is done"

---

## Verification Checklist

### Build & Tests
1. `just build-cs-components` - compiles without errors
2. `just test-cs-components` - all tests pass

### Mouse Drag & Drop
3. Drag card from Backlog → To Do → card moves, notification shows
4. Drag card from To Do → In Progress → card moves
5. Drop in same column → no backend call, no notification
6. Visual feedback: column highlights on hover during drag

### Keyboard Navigation
7. Tab to card → press Space → card is "picked up"
8. Arrow keys → target column changes
9. Space again → card drops to target column
10. Escape → drag cancelled

### Type-Specific Cards
11. Task cards show NO progress bar
12. Story cards show task progress bar with swimlane colors
13. Epic cards show BOTH story progress AND task progress bars
14. Progress bar segments colored by status
15. Hover on progress bar shows tooltip with counts

---

## Rollback Plan

If Radzen DropZone doesn't work as expected:

1. **First choice:** Use Radzen's TreeView with drag-and-drop
2. **Second choice:** Try different Radzen component or pattern
3. **Descope:** Remove drag-and-drop; use explicit "Move to..." buttons

**Note:** Per Critical Design Constraints, JS interop solutions are not acceptable.

**Git strategy:** Create feature branch `fix/kanban-radzen-dropzone` before starting. If issues, revert to `main`.

---

## Final Verification

After both sub-sessions complete:

```bash
# Full frontend check
just check-frontend

# Run all component tests
just test-cs-components

# Build and run application
just dev

# Manual testing
# - Drag cards between columns
# - Verify progress bars on Epic/Story cards
# - Test keyboard navigation
```

---

## Actual Implementation (Session 45.1 Complete)

**Full details in [45.1-Session-Plan.md](45.1-Session-Plan.md#actual-implementation-session-451-complete)**

### Root Cause Discovery

After 4+ debugging sessions, the issue was **NOT** with Blazor, Radzen, or HTML5 drag-drop APIs. The root cause was:

**Tauri's native drag-drop handler intercepts HTML5 drag events.**

Tauri's `dragDropEnabled` setting (default: `true`) enables native OS-level file drag-drop onto the application window. This intercepts all drag events at the webview level, preventing them from reaching the browser's HTML5 drag-drop API that Radzen relies on.

### Fixes Applied

#### 1. Tauri Configuration (Critical Fix)

**File:** `desktop/src-tauri/tauri.conf.json`

```json
"windows": [
  {
    "title": "Project Manager",
    ...
    "dragDropEnabled": false  // <-- ADD THIS LINE
  }
]
```

This disables Tauri's native file drag-drop, allowing HTML5 drag events to reach Radzen's JavaScript handlers.

#### 2. Backend Status Validation

**File:** `backend/crates/pm-ws/src/handlers/work_item.rs`

The `validate_status()` function was missing "backlog" as a valid status, causing drops to Backlog column to fail validation.

```rust
// BEFORE (missing backlog):
"todo" | "in_progress" | "review" | "done" | "blocked" => Ok(()),

// AFTER:
"backlog" | "todo" | "in_progress" | "review" | "done" | "blocked" => Ok(()),
```

#### 3. Backend Tests Updated

**File:** `backend/crates/pm-ws/src/tests/property_tests.rs`

Added "backlog" to valid status test cases.

#### 4. KanbanBoard Restructured

**File:** `frontend/ProjectManagement.Components/WorkItems/KanbanBoard.razor`

- Uses `RadzenDropZoneContainer` wrapping all columns
- Each `KanbanColumn` contains a `RadzenDropZone`
- `KanbanCard` rendered via the container's `Template`
- Filters, accessibility, and connection state handling preserved

### Key Learnings

1. **Tauri + HTML5 Drag-Drop Conflict:** Anyone using Radzen (or any HTML5 drag-drop library) inside Tauri MUST set `dragDropEnabled: false` in the window config. This is not documented in Radzen's docs because it's a Tauri-specific issue.

2. **Test in Browser First:** When debugging drag-drop in Tauri, test the same code in a standalone browser to isolate whether the issue is Tauri's webview or the component code.

3. **Check Backend Validation:** Frontend drag-drop can appear "broken" when the backend rejects the status value. Always verify the full request/response flow.

### Files Modified

| File | Change |
|------|--------|
| `desktop/src-tauri/tauri.conf.json` | Added `dragDropEnabled: false` |
| `backend/crates/pm-ws/src/handlers/work_item.rs` | Added "backlog" to valid statuses |
| `backend/crates/pm-ws/src/tests/property_tests.rs` | Updated status validation tests |
| `frontend/.../WorkItems/KanbanBoard.razor` | Restructured with RadzenDropZoneContainer |
| `frontend/.../WorkItems/KanbanColumn.razor` | Contains RadzenDropZone |
| `frontend/.../WorkItems/KanbanCard.razor` | Rendered via Template |

---

## ✅ Session 45 Completion Summary

**Completed**: 2026-01-26
**Status**: ✅ **FULLY COMPLETE** (both sub-sessions)

### What Was Delivered

**Session 45.1 (Drag-and-Drop):**
- ✅ Fixed Tauri drag-drop conflict (`dragDropEnabled: false`)
- ✅ Integrated Radzen DropZone components
- ✅ Fixed backend "backlog" status validation
- ✅ Drag-and-drop working reliably (mouse + keyboard)

**Session 45.2 (Progress Bars):**
- ✅ ChildProgress model for tracking child item status
- ✅ ViewModelFactory computes progress from cached data
- ✅ ChildProgressBar component with swimlane visualization
- ✅ Epic cards show Story + Task progress bars
- ✅ Story cards show Task progress bar
- ✅ Task cards remain simple (no progress)
- ✅ 4 comprehensive tests + bonus accessibility fix
- ✅ 273 tests passing (100% pass rate)

### Key Achievements

1. **Visual Hierarchy**: Cards now show their type and completion state at a glance
2. **Bottleneck Detection**: Swimlane progress bars reveal where work is stuck
3. **No Extra Queries**: Progress computed from AppState cache
4. **Production Quality**: Comprehensive tests, accessibility, reduced motion support
5. **Better Than JIRA**: Immediate visual feedback without drilling through multiple screens

### Verification

```bash
✅ just build-cs-components  # Clean build, 0 warnings
✅ just test-cs-components   # 273 tests passing
✅ just dev                  # Drag-and-drop works, progress bars render correctly
```

### Files Summary

**Total Files Modified**: 14
**Total Files Created**: 2
**Total Tests Added**: 4
**Test Pass Rate**: 100% (273/273)

---

## Sources

- [Radzen DropZone Demo](https://blazor.radzen.com/dropzone)
- [RadzenDropZoneItemRenderEventArgs API](https://blazor.radzen.com/docs/api/Radzen.RadzenDropZoneItemRenderEventArgs-1.html)
- [Radzen GitHub - RadzenDropZone.razor](https://github.com/radzenhq/radzen-blazor/blob/master/Radzen.Blazor/RadzenDropZone.razor)
- [Tauri Window Configuration](https://tauri.app/reference/config/#windowconfig) - `dragDropEnabled` setting
