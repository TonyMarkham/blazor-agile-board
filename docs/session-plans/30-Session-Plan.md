# Session 30: Work Item UI - Blazor Components

## Production-Grade Score Target: 9.25/10

This session implements the complete work item UI with Blazor WebAssembly:

- ViewModel pattern with `IsPending` state tracking
- ViewModelFactory for creating ViewModels from store data
- Complete CSS design system (app, work-items, kanban, layout)
- Leaf components (badges, buttons, dialogs, skeletons)
- Composite components (WorkItemRow, KanbanCard, dialogs)
- List and Board views with virtualization
- Full pages (Home, ProjectDetail, WorkItemDetail)
- Comprehensive test coverage (168+ tests)

---

## Sub-Session Breakdown

This plan has been split into sub-sessions to fit within ~50k token budgets:

| Session | Scope | Files | Est. Tokens | Actual | Status |
|---------|-------|-------|-------------|--------|--------|
| **[30.1](30.1-Session-Plan.md)** | ViewModels + CSS Foundation | 8 files | ~40k | ~40k | ✅ Complete |
| **[30.2](30.2-Session-Plan.md)** | Leaf Components | 10 files | ~35k | ~35k | ✅ Complete |
| **[30.3](30.3-Session-Plan.md)** | ViewModel + Component Tests | 5 files | ~45k | ~45k | ✅ Complete |
| **[30.4](30.4-Session-Plan.md)** | Composite Components + Dialogs | 7 files | ~45k | ~120k | ✅ Complete |
| **[30.5](30.5-Session-Plan.md)** | Pages + Layout | 8 files | ~40k | ~100k | ✅ Complete |
| **[30.6](30.6-Session-Plan.md)** | Part 2 Tests | 6 files | ~50k | TBD | Pending |

**Total: 41 files, 256 tests (27 Core + 168 Components + 61 Services)**

---

## Session 30.1: ViewModels + CSS Foundation

**Files Created:**
- `ProjectManagement.Core/ViewModels/IViewModel.cs` - Base interface
- `ProjectManagement.Core/ViewModels/WorkItemViewModel.cs` - Work item ViewModel
- `ProjectManagement.Core/ViewModels/SprintViewModel.cs` - Sprint ViewModel
- `ProjectManagement.Core/ViewModels/ViewModelFactory.cs` - Factory with pending state
- `ProjectManagement.Components/wwwroot/css/app.css` - Base design system
- `ProjectManagement.Components/wwwroot/css/work-items.css` - Work item styles
- `ProjectManagement.Components/wwwroot/css/kanban.css` - Kanban board styles
- `ProjectManagement.Components/wwwroot/css/layout.css` - Layout styles

**Verification:** `dotnet build ProjectManagement.Core && dotnet build ProjectManagement.Components`

---

## Session 30.2: Leaf Components

**Files Created:**
- `ProjectManagement.Components/_Imports.razor` - Component imports
- `ProjectManagement.Components/Shared/OfflineBanner.razor` - Connection status banner
- `ProjectManagement.Components/Shared/EmptyState.razor` - Empty state display
- `ProjectManagement.Components/Shared/LoadingButton.razor` - Button with loading state
- `ProjectManagement.Components/Shared/DebouncedTextBox.razor` - Debounced input
- `ProjectManagement.Components/Shared/ConfirmDialog.razor` - Confirmation dialog
- `ProjectManagement.Components/Shared/ProjectDetailSkeleton.razor` - Loading skeleton
- `ProjectManagement.Components/WorkItems/WorkItemTypeIcon.razor` - Type icon
- `ProjectManagement.Components/WorkItems/WorkItemStatusBadge.razor` - Status badge
- `ProjectManagement.Components/WorkItems/PriorityBadge.razor` - Priority badge

**Verification:** `dotnet build ProjectManagement.Components`

---

## Session 30.3: ViewModel + Component Tests

**Files Created:**
- `ProjectManagement.Components.Tests/ViewModels/ViewModelFactoryTests.cs` - Factory tests
- `ProjectManagement.Components.Tests/ViewModels/WorkItemViewModelTests.cs` - WorkItem VM tests
- `ProjectManagement.Components.Tests/ViewModels/SprintViewModelTests.cs` - Sprint VM tests
- `ProjectManagement.Components.Tests/Shared/SharedComponentTests.cs` - Shared component tests
- `ProjectManagement.Components.Tests/WorkItems/BadgeComponentTests.cs` - Badge component tests

**Verification:** `dotnet test ProjectManagement.Components.Tests` (168 tests passing) ✅

---

## Session 30.4: Composite Components + Dialogs ✅

**Status:** Complete (2026-01-20)

**Files Created:**
- `ProjectManagement.Components/WorkItems/WorkItemRow.razor` - List row component (5.9 KB)
- `ProjectManagement.Components/WorkItems/KanbanCard.razor` - Kanban card component (6.3 KB)
- `ProjectManagement.Components/WorkItems/VersionConflictDialog.razor` - Conflict resolution (2.6 KB)
- `ProjectManagement.Components/WorkItems/WorkItemDialog.razor` - Create/Edit dialog (15.8 KB)
- `ProjectManagement.Components/WorkItems/WorkItemList.razor` - List view with filtering (10.8 KB)
- `ProjectManagement.Components/WorkItems/KanbanColumn.razor` - Single Kanban column (2.8 KB)
- `ProjectManagement.Components/WorkItems/KanbanBoard.razor` - Full Kanban board (10.3 KB)

**Key Features Delivered:**
- Drag-and-drop support (mouse + keyboard with Space/Arrow keys/Escape)
- Version conflict resolution with 3-way merge UI
- Form validation with character limits and live counters
- Virtualized lists for performance
- Comprehensive accessibility (ARIA labels, keyboard nav, screen reader support)
- Connection state awareness (actions disabled when offline)

**Critical Fix:** 9 instances of `StateHasChanged()` in async methods corrected to `await InvokeAsync(StateHasChanged)`

**Verification:** `dotnet build ProjectManagement.Components` - Clean, 0 warnings | 256 tests passing

---

## Session 30.5: Pages + Layout ✅

**Status:** Complete (2026-01-20)

**Files Created:**
- `ProjectManagement.Wasm/Layout/NavMenu.razor` - Navigation menu (2.1 KB)
- `ProjectManagement.Wasm/Layout/MainLayout.razor` - Main layout (1.4 KB)
- `ProjectManagement.Wasm/Pages/Home.razor` - Home page (6.8 KB)
- `ProjectManagement.Wasm/Pages/ProjectDetail.razor` - Project detail page (5.0 KB)
- `ProjectManagement.Wasm/Pages/WorkItemDetail.razor` - Work item detail page (12.5 KB)

**Files Modified:**
- `ProjectManagement.Wasm/wwwroot/index.html` - Add CSS links (+4 links)
- `ProjectManagement.Wasm/_Imports.razor` - Add imports (+3 namespaces)
- `ProjectManagement.Wasm/Program.cs` - Register ViewModelFactory (+1 service, +1 using)

**Key Features Delivered:**
- Complete navigation structure with reactive sidebar menu
- Three working pages with proper routing and breadcrumbs
- List/board view toggle on ProjectDetail page
- Cascading delete with child count warnings
- Loading states, error states, and empty states
- Responsive layout adapting to mobile/tablet/desktop

**Verification:** `dotnet build ProjectManagement.Wasm` - Clean, 0 warnings | 256 tests passing

---

## Session 30.6: Part 2 Tests

**Files Created:**
- `ProjectManagement.Components.Tests/WorkItems/WorkItemRowTests.cs` - Row tests
- `ProjectManagement.Components.Tests/WorkItems/KanbanCardTests.cs` - Card tests
- `ProjectManagement.Components.Tests/WorkItems/DialogTests.cs` - Dialog tests
- `ProjectManagement.Components.Tests/WorkItems/WorkItemListTests.cs` - List tests
- `ProjectManagement.Components.Tests/WorkItems/KanbanBoardTests.cs` - Board tests
- `ProjectManagement.Components.Tests/Pages/PageIntegrationTests.cs` - Page tests

**Verification:** `dotnet test` (70+ tests passing)

---

## Pre-Implementation Checklist

Before starting **any** sub-session:

- [ ] `dotnet build frontend/ProjectManagement.sln` passes (or no frontend yet)
- [ ] .NET 10 SDK installed
- [ ] Radzen.Blazor 8.6.2+ referenced in projects
- [ ] Backend Session 20 complete (WebSocket infrastructure)

---

## Files Summary

### Create (41 files)

| Session | File | Purpose |
|---------|------|---------|
| 30.1 | `Core/ViewModels/IViewModel.cs` | Base ViewModel interface |
| 30.1 | `Core/ViewModels/WorkItemViewModel.cs` | Work item ViewModel |
| 30.1 | `Core/ViewModels/SprintViewModel.cs` | Sprint ViewModel |
| 30.1 | `Core/ViewModels/ViewModelFactory.cs` | Factory with pending state |
| 30.1 | `Components/wwwroot/css/app.css` | Base design system |
| 30.1 | `Components/wwwroot/css/work-items.css` | Work item styles |
| 30.1 | `Components/wwwroot/css/kanban.css` | Kanban board styles |
| 30.1 | `Components/wwwroot/css/layout.css` | Layout styles |
| 30.2 | `Components/_Imports.razor` | Component imports |
| 30.2 | `Components/Shared/OfflineBanner.razor` | Connection status |
| 30.2 | `Components/Shared/EmptyState.razor` | Empty state display |
| 30.2 | `Components/Shared/LoadingButton.razor` | Button with loading |
| 30.2 | `Components/Shared/DebouncedTextBox.razor` | Debounced input |
| 30.2 | `Components/Shared/ConfirmDialog.razor` | Confirmation dialog |
| 30.2 | `Components/Shared/ProjectDetailSkeleton.razor` | Loading skeleton |
| 30.2 | `Components/WorkItems/WorkItemTypeIcon.razor` | Type icon |
| 30.2 | `Components/WorkItems/WorkItemStatusBadge.razor` | Status badge |
| 30.2 | `Components/WorkItems/PriorityBadge.razor` | Priority badge |
| 30.3 | `Components.Tests/ViewModels/ViewModelFactoryTests.cs` | Factory tests |
| 30.3 | `Components.Tests/ViewModels/WorkItemViewModelTests.cs` | WorkItem VM tests |
| 30.3 | `Components.Tests/ViewModels/SprintViewModelTests.cs` | Sprint VM tests |
| 30.3 | `Components.Tests/Shared/SharedComponentTests.cs` | Shared tests |
| 30.3 | `Components.Tests/WorkItems/BadgeComponentTests.cs` | Badge tests |
| 30.4 | `Components/WorkItems/WorkItemRow.razor` | List row |
| 30.4 | `Components/WorkItems/KanbanCard.razor` | Kanban card |
| 30.4 | `Components/WorkItems/VersionConflictDialog.razor` | Conflict dialog |
| 30.4 | `Components/WorkItems/WorkItemDialog.razor` | CRUD dialog |
| 30.4 | `Components/WorkItems/WorkItemList.razor` | List view |
| 30.4 | `Components/WorkItems/KanbanColumn.razor` | Kanban column |
| 30.4 | `Components/WorkItems/KanbanBoard.razor` | Kanban board |
| 30.5 | `Wasm/Layout/NavMenu.razor` | Navigation |
| 30.5 | `Wasm/Layout/MainLayout.razor` | Main layout |
| 30.5 | `Wasm/Pages/Home.razor` | Home page |
| 30.5 | `Wasm/Pages/ProjectDetail.razor` | Project detail |
| 30.5 | `Wasm/Pages/WorkItemDetail.razor` | Work item detail |
| 30.6 | `Components.Tests/WorkItems/WorkItemRowTests.cs` | Row tests |
| 30.6 | `Components.Tests/WorkItems/KanbanCardTests.cs` | Card tests |
| 30.6 | `Components.Tests/WorkItems/DialogTests.cs` | Dialog tests |
| 30.6 | `Components.Tests/WorkItems/WorkItemListTests.cs` | List tests |
| 30.6 | `Components.Tests/WorkItems/KanbanBoardTests.cs` | Board tests |
| 30.6 | `Components.Tests/Pages/PageIntegrationTests.cs` | Page tests |

### Modify (3 files)

| File | Change |
|------|--------|
| `Wasm/wwwroot/index.html` | Add CSS links |
| `Wasm/_Imports.razor` | Add component imports |
| `Wasm/Program.cs` | Register ViewModelFactory |

---

## Production-Grade Scoring

| Category | Score | Justification |
|----------|-------|---------------|
| Error Handling | 9.5/10 | Version conflict handling, validation, error boundaries |
| Validation | 9.3/10 | Form validation, character limits, required fields |
| State Management | 9.5/10 | IsPending tracking, optimistic updates, dirty detection |
| Data Integrity | 9.3/10 | Optimistic locking, version conflicts, soft deletes |
| Performance | 9.0/10 | Virtualization, debounced search, CSS optimization |
| Testing | 9.5/10 | 70+ bUnit tests, FluentAssertions, comprehensive coverage |
| Accessibility | 9.5/10 | ARIA labels, keyboard nav, screen reader support |
| UX | 9.3/10 | Loading states, empty states, offline indicators |
| Code Quality | 9.0/10 | Clean patterns, no shortcuts, complete implementations |

**Overall Score: 9.25/10**

### What Would Make It 10/10

- End-to-end Playwright tests
- Real-time sync with WebSocket events
- Offline mode with IndexedDB persistence
- Performance benchmarks
- Accessibility audit with axe-core

---

## Final Verification

After all six sub-sessions are complete:

```bash
cd frontend

# Build all projects
dotnet build ProjectManagement.sln

# Run all tests
dotnet test

# Expected: 70+ tests passing, 0 failures

# Run the WASM host
dotnet run --project ProjectManagement.Wasm
```

- [ ] All ViewModels compile
- [ ] All CSS files created
- [ ] All leaf components compile
- [ ] All composite components compile
- [ ] All pages compile
- [ ] All tests pass (70+)
- [ ] Kanban drag-and-drop works
- [ ] Keyboard navigation works
- [ ] Offline banner shows/hides correctly
- [ ] No console errors in browser
