# Session 51: Sprint & Comment UI Integration

## Production-Grade Score Target: 9/10

This session integrates the existing Sprint and Comment UI components (built in Session 50) into the main application pages:

- Sprint management tab in ProjectDetail with full CRUD operations
- Comment thread section in WorkItemDetail page
- Real active sprints count on Home dashboard
- CommentStore exposed through AppState for consistency
- Integration tests for all new UI

---

## Sub-Session Breakdown

This plan has been split into sub-sessions to fit within token budgets:

| Session | Scope | Est. Tokens | Status |
|---------|-------|-------------|--------|
| **[51.1](51.1-Session-Plan.md)** | AppState + Test Helpers | ~20-25k | Pending |
| **[51.2](51.2-Session-Plan.md)** | UI Integration + Tests | ~35-40k | Pending |

---

## Session 51.1: AppState + Test Helpers

**Files Modified:**
- `ProjectManagement.Services/State/AppState.cs` - Add ICommentStore property
- `ProjectManagement.Components.Tests/Pages/PageIntegrationTests.cs` - Update AppState mock
- `ProjectManagement.Components.Tests/Shared/SharedComponentTests.cs` - Update CreateMockAppState

**Verification:** `just test-frontend`

---

## Session 51.2: UI Integration + Tests

**Files Modified:**
- `ProjectManagement.Wasm/Pages/ProjectDetail.razor` - Add Sprints tab
- `ProjectManagement.Wasm/Pages/WorkItemDetail.razor` - Add Comments section
- `ProjectManagement.Wasm/Pages/Home.razor` - Fix active sprints count
- `ProjectManagement.Components.Tests/Pages/PageIntegrationTests.cs` - Add integration tests

**Verification:** `just test-frontend`

---

## Pre-Implementation Checklist

Before starting **any** sub-session:

- [ ] `just check` passes (all code compiles)
- [ ] `just test` passes (all 615 tests passing)
- [ ] Session 50 components exist:
  - `SprintCard.razor`, `SprintDialog.razor`
  - `CommentList.razor`, `CommentEditor.razor`
  - `SprintStore`, `CommentStore`

---

## Files Summary

### Modify (6 files)

| File | Purpose | Session |
|------|---------|---------|
| `ProjectManagement.Services/State/AppState.cs` | Add ICommentStore property | 51.1 |
| `ProjectManagement.Components.Tests/Pages/PageIntegrationTests.cs` | Update mock + add tests | 51.1, 51.2 |
| `ProjectManagement.Components.Tests/Shared/SharedComponentTests.cs` | Update CreateMockAppState | 51.1 |
| `ProjectManagement.Wasm/Pages/ProjectDetail.razor` | Add Sprints tab | 51.2 |
| `ProjectManagement.Wasm/Pages/WorkItemDetail.razor` | Add Comments section | 51.2 |
| `ProjectManagement.Wasm/Pages/Home.razor` | Fix active sprints count | 51.2 |

---

## Production-Grade Scoring

| Category | Score | Justification |
|----------|-------|---------------|
| Error Handling | 9/10 | Try/catch with user notifications on all operations |
| Validation | 9/10 | Uses existing validated components from Session 50 |
| Authorization | 9/10 | Author-only comment edit/delete, connection state checks |
| UX Polish | 9/10 | Loading states, empty states, confirmation dialogs |
| Real-time | 9/10 | State subscriptions, automatic UI refresh |
| Testing | 9/10 | Integration tests for Sprint tab and Comments section |

**Overall Score: 9/10**

### What Would Make It 10/10

- User name lookup service instead of truncated GUIDs
- Keyboard navigation for sprint list
- Comment count badge on work item cards
- Sprint burndown mini-chart in SprintCard

---

## Learning Objectives

Each sub-session teaches specific concepts:

| Session | Key Concepts |
|---------|--------------|
| **51.1** | DI patterns, centralized state management, test mock setup |
| **51.2** | Blazor component composition, tab navigation, event subscriptions, integration testing |

---

## Final Verification

After both sub-sessions are complete:

```bash
# Full workspace check
just check

# Run all tests (should be 620+ now)
just test

# Build everything
just build-release

# Start app
just dev

# Test workflow:
# 1. Home page shows Active Sprints count
# 2. Project page has Sprints tab with loading spinner
# 3. Create/Start/Complete/Delete sprints work
# 4. Work item page shows Comments section
# 5. Create/Edit/Delete comments work
```
