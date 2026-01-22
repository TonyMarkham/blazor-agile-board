# Future Enhancements & Technical Debt

This file tracks improvements that are nice-to-have but not required for MVP.

---

## URL-Based State Management (Medium Priority)

### Problem Statement
Blazor WASM can leak memory over time. When the app slows down, users will refresh the browser. They should be able to pick up where they left off since data is synced to the backend - only the UI state is ephemeral.

### Current Gap
If UI state lives only in memory (current project, view mode, filters, selected items), a refresh dumps the user back to the home screen and they lose their context.

### Solution: URL as State Recovery Strategy

**Principle**: If it matters for context, it goes in the URL.

**Route Design**:
```
/projects/{projectId}/board?filter=status:active&swimlane=assignee
/projects/{projectId}/sprints/{sprintId}
/projects/{projectId}/epics/{epicId}/edit
/projects/{projectId}/items/{itemId}
```

**What URL Captures**:
- Current project
- Current view mode (board/sprint/epic)
- Selected item being viewed/edited
- Filters, sorts, groupings (query params)

**Blazor Implementation**:
```razor
@page "/projects/{ProjectId}/board"

@code {
    [Parameter] public string ProjectId { get; set; }

    [SupplyParameterFromQuery]
    public string? Filter { get; set; }

    [SupplyParameterFromQuery]
    public string? Swimlane { get; set; }

    protected override async Task OnParametersSetAsync()
    {
        var parsedFilter = FilterParser.Parse(Filter);
        await AppState.LoadProjectAsync(ProjectId);
        await AppState.LoadBoardItemsAsync(ProjectId, parsedFilter);
    }
}
```

**Flow on Refresh**:
1. Same URL → same route → same component
2. Component parses route params and query strings
3. Fetches fresh data from Rust backend via WebSocket
4. UI rebuilds in same context

### Benefits (All Free from Good URL Design)
- **Refresh recovery** - Land back where you were
- **Bookmarks** - Save "My active sprint" as browser bookmark
- **Shareability** - Share link with teammate, they see exact same view
- **Browser history** - Back/forward buttons work naturally
- **Deep linking** - Email notifications link directly to specific items

### What URL Cannot Cover
- Unsaved form input mid-typing
- Scroll position
- Expanded/collapsed accordion panels

### Supplemental: LocalStorage for Form Drafts
For long forms, auto-save drafts to LocalStorage:
- Key: `{userId}:{projectId}:{itemId}:draft`
- On reload: Prompt "Recover unsaved changes?"
- User can accept or discard

### Implementation Checklist
- [ ] Audit existing routes - do they capture meaningful state?
- [ ] Add query param support for filters/sorts
- [ ] Document URL design conventions for consistency
- [ ] Add form draft auto-save for work item creation/editing
- [ ] Test refresh recovery across all major views

### Estimated Effort
- Route refactoring: ~2-3 hours
- Query param parsing utilities: ~1 hour
- Form draft persistence: ~2 hours
- Testing: ~1 hour

**Priority**: Medium (improves UX significantly, prevents user frustration on refresh)
**Session**: 43 or later

---

## Known Issues (Safe to Ignore)

### Source Map Warning in Development
**Warning**: `Source Map "http://127.0.0.1:1430/_framework/dotnet.runtime.js.map" has SyntaxError: JSON Parse error`
**Cause**: Blazor debug builds + Tauri dev server source map handling
**Impact**: None (cosmetic browser console warning only)
**Fix**: Not required - goes away in Release builds

---

## Future Sessions (Post-MVP)

### Session 50: Sprints & Comments
- Sprint planning UI
- Comment threads
- Real-time collaboration

### Session 60: Time Tracking & Dependencies
- Running timers
- Dependency management
- Circular dependency detection

### Session 70: Activity Logging & Polish
- Activity feed
- Error handling polish
- Loading states
- Documentation

### Session 80+: Advanced Features
- REST API for LLM integration
- Offline support with sync
- Multi-tenant SaaS deployment
- Advanced reporting & analytics
- Import/export (JIRA, CSV)
