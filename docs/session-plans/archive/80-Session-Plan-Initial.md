# Session 80 – Dependency Management UI (Production-Grade Plan)

## Goal
Expose dependency management in the Work Item detail UI so users can add/remove Blocks/Relates dependencies with clear semantics, robust validation, accessible UX, and production-grade performance and error handling, with explicit dependency contracts and verified integration points.

## Scope
Frontend-only UI wiring and components. Backend already supports dependency CRUD and activity log entries. This plan includes explicit verification of frontend API contracts, connectivity state, and CSS integration before implementation.

## Architecture Overview

### Data Flow
1. WorkItem detail page loads.
2. DependencyManager requests dependency data via `IDependencyStore.RefreshAsync(WorkItemId)`.
3. Store uses `IWebSocketClient.GetDependenciesAsync` to fetch current dependencies.
4. Store raises `OnChanged` event; DependencyManager re-renders.
5. User opens AddDependencyDialog and creates dependency.
6. Store performs optimistic create and sends `CreateDependencyRequest` over WebSocket.
7. Backend writes dependency + activity log, broadcasts `DependencyCreated` + `ActivityLogCreated`.
8. Store receives real-time events, updates UI.

### Required Contracts (Must Verify)
These must exist or be implemented before UI work begins:
- `IDependencyStore`:
  - `IReadOnlyList<Dependency> GetBlocking(Guid workItemId)`
  - `IReadOnlyList<Dependency> GetBlocked(Guid workItemId)`
  - `bool IsPending(Guid dependencyId)`
  - `Task RefreshAsync(Guid workItemId, CancellationToken ct)`
  - `Task CreateAsync(CreateDependencyRequest request, CancellationToken ct)`
  - `Task DeleteAsync(Guid dependencyId, CancellationToken ct)`
  - `event Action? OnChanged`
- `IWebSocketClient` (or equivalent store dependency):
  - `Task<IReadOnlyList<Dependency>> GetDependenciesAsync(Guid workItemId, CancellationToken ct)`
- `AppState.WorkItems`:
  - `WorkItem? GetById(Guid id)`
  - `IReadOnlyList<WorkItem> GetByProject(Guid projectId)` (or a documented alternative)
- Connectivity state available for UI disabling (e.g., `IConnectivityService.IsOnline`, `IWebSocketClient.IsConnected`, or `AppState.Connection.IsOnline`).
- CSS bundle inclusion point for RCL (exact path verified).

### UX Rules
- Add dependency for same project only (backend enforces; UI filters candidates).
- Prevent self-dependency and duplicates in UI; surface backend validation errors as toast + inline message.
- Blocks vs Relates semantics are explicit: Blocks = directional (A blocks B); Relates = symmetric concept but stored with a canonical direction.
- Deletion is optimistic; pending states disable actions and show visual loading.
- All interactive elements are keyboard accessible and screen-reader labeled.
- Accessibility includes listbox semantics with roving tabindex or `aria-activedescendant` and explicit selection state.
- Offline/online state is authoritative and used to disable Add/Remove actions with a visible message.

---

## Implementation Steps (Production-Grade)

### 1) Inventory and Confirm Existing APIs (No-code step)
Confirm existing public APIs and patterns before writing UI. Capture method signatures and location (file/namespace).

- `IDependencyStore`:
  - `GetBlocking`, `GetBlocked`, `IsPending`, `CreateAsync`, `DeleteAsync`, `RefreshAsync`, `OnChanged`.
- Store dependency for fetching (likely `IWebSocketClient.GetDependenciesAsync` or equivalent). Verify it exists and return type matches `Dependency`.
- `AppState.WorkItems`: `GetByProject(Guid)` and `GetById(Guid)` or documented alternatives.
- `ViewModelFactory.Create` / `CreateMany` availability.
- `DialogService` usage patterns in existing dialogs.
- Connectivity indicator (online/offline) for UI gating.
- CSS bundle import location for RCL styles.

If any are missing, document the gap and assign ownership (UI vs Services) before continuing. Do not invent new public surface area in the UI layer without approval.

---

### 2) Ensure Styles Are Loaded (Dependency for UI validation)

`dependencies.css` already contains:
- `.dependency-manager`, `.dependency-item`, `.dep-type`, `.dependency-empty`
- `.add-dependency-dialog` styles

Verify that this CSS is included in the RCL bundle. If not, add an import to the component library stylesheet (e.g., `frontend/ProjectManagement.Components/wwwroot/styles.css` or existing bundle file). Document the exact bundle entry point.

Snippet (example import line):

```css
@import "Dependencies/dependencies.css";
```

---

### 3) Add a Small Row Component (Lowest dependency UI block)
Create `frontend/ProjectManagement.Components/Dependencies/DependencyRow.razor`.

```razor
@using ProjectManagement.Core.Models
@using ProjectManagement.Core.ViewModels
@inject IDependencyStore DependencyStore

<div class="dependency-item @(IsPending ? "pending" : "")" aria-busy="@(IsPending ? "true" : "false")">
    <span class="item-title">@DisplayTitle</span>
    <span class="dep-type @(TypeCss)">@TypeLabel</span>
    <RadzenButton Icon="delete" Size="ButtonSize.Small" Variant="Variant.Text"
                  ButtonStyle="ButtonStyle.Danger" class="delete-btn"
                  Disabled="@IsPending"
                  aria-label="@("Remove dependency " + DisplayTitle)"
                  Click="@(() => OnRemove.InvokeAsync(Dependency.Id))" />
</div>

@code {
    [Parameter, EditorRequired] public Dependency Dependency { get; set; } = null!;
    [Parameter, EditorRequired] public Guid CurrentWorkItemId { get; set; }
    [Parameter, EditorRequired] public Func<Guid, WorkItemViewModel?> WorkItemLookup { get; set; } = null!;
    [Parameter, EditorRequired] public EventCallback<Guid> OnRemove { get; set; }

    private bool IsPending => DependencyStore.IsPending(Dependency.Id);

    private Guid OtherItemId => Dependency.BlockingItemId == CurrentWorkItemId
        ? Dependency.BlockedItemId
        : Dependency.BlockingItemId;

    private string DisplayTitle => WorkItemLookup(OtherItemId)?.Title ?? OtherItemId.ToString()[..8];

    private string TypeLabel => Dependency.Type == DependencyType.Blocks ? "Blocks" : "Relates";
    private string TypeCss => Dependency.Type == DependencyType.Blocks ? "blocks" : "relates";
}
```

---

### 4) Create Add Dependency Dialog (Depends on contracts + AppState)

Create `frontend/ProjectManagement.Components/Dependencies/AddDependencyDialog.razor`.

Responsibilities:
- Search within project work items.
- Exclude current item, and items already linked in either direction.
- Allow selection of Blocks/Relates.
- Create dependency and close dialog on success.
- Provide keyboard navigation and selection with real listbox semantics.
- Debounce search input.
- Display empty, loading, and error states.
- Show inline validation errors (from backend) and keep dialog open on failure.
- Do not load or render more than an agreed max result set (paging or cap). If no backend search exists, explicitly cap results and document UX limits.

Snippet (core structure with debounce + keyboard support):

```razor
@using ProjectManagement.Core.Interfaces
@using ProjectManagement.Core.Models
@using ProjectManagement.Core.ViewModels
@inject IDependencyStore DependencyStore
@inject AppState AppState
@inject DialogService DialogService

<div class="add-dependency-dialog" role="dialog" aria-label="Add dependency">
    <RadzenStack Gap="0.75rem">
        <RadzenTextBox @bind-Value="SearchText" Placeholder="Search work items..."
                       aria-label="Search work items" />

        <RadzenDropDown Data="DependencyTypes" @bind-Value="SelectedType"
                        TextProperty="Label" ValueProperty="Value"
                        aria-label="Dependency type" />

         <div class="search-results" role="listbox" aria-activedescendant="@_activeId">
            @if (FilteredResults.Count == 0)
            {
                <div class="dependency-empty">No matching items.</div>
            }
            else
            {
                @foreach (var item in FilteredResults)
                {
                     <div class="search-result" role="option" tabindex="@GetTabIndex(item)"
                          aria-selected="@IsActive(item)"
                         @onclick="(() => SelectItem(item))"
                         @onkeydown="(e => HandleKeyDown(e, item))">
                        <strong>@item.Title</strong>
                        <div class="text-muted">@item.ItemTypeDisplayName</div>
                    </div>
                }
            }
        </div>
    </RadzenStack>
</div>

@code {
    [Parameter, EditorRequired] public Guid WorkItemId { get; set; }
    [Parameter, EditorRequired] public Guid ProjectId { get; set; }

    private string SearchText { get; set; } = "";
    private DependencyType SelectedType { get; set; } = DependencyType.Blocks;

    private CancellationTokenSource? _debounceCts;
    private List<WorkItemViewModel> _cached = new();
    private Guid? _activeId;

    protected override void OnInitialized()
    {
        _cached = AppState.ViewModelFactory
            .CreateMany(AppState.WorkItems.GetByProject(ProjectId))
            .Where(w => w.Id != WorkItemId)
            .ToList();
    }

    private HashSet<Guid> ExistingLinks => DependencyStore
        .GetBlocking(WorkItemId).Select(d => d.BlockingItemId)
        .Concat(DependencyStore.GetBlocked(WorkItemId).Select(d => d.BlockedItemId))
        .ToHashSet();

    private List<WorkItemViewModel> FilteredResults => _cached
        .Where(w => !ExistingLinks.Contains(w.Id))
        .Where(w => string.IsNullOrWhiteSpace(SearchText)
            || w.Title.Contains(SearchText, StringComparison.OrdinalIgnoreCase))
        .OrderBy(w => w.Title)
        .Take(50)
        .ToList();

    private async Task SelectItem(WorkItemViewModel item)
    {
        var request = BuildRequest(item);

        await DependencyStore.CreateAsync(request, CancellationToken.None);
        DialogService.Close(true);
    }

    private CreateDependencyRequest BuildRequest(WorkItemViewModel item)
    {
        // Canonical direction:
        // Blocks: selected item blocks current item (selected -> current)
        // Relates: store with current as blocking to keep consistency (current -> selected)
        return SelectedType switch
        {
            DependencyType.Blocks => new CreateDependencyRequest
            {
                BlockingItemId = item.Id,
                BlockedItemId = WorkItemId,
                Type = DependencyType.Blocks
            },
            _ => new CreateDependencyRequest
            {
                BlockingItemId = WorkItemId,
                BlockedItemId = item.Id,
                Type = DependencyType.Relates
            }
        };
    }

    private Task HandleKeyDown(KeyboardEventArgs e, WorkItemViewModel item)
    {
        if (e.Key == "Enter")
        {
            return SelectItem(item);
        }
        if (e.Key == "ArrowDown" || e.Key == "ArrowUp")
        {
            MoveActive(e.Key == "ArrowDown" ? 1 : -1);
            return Task.CompletedTask;
        }
        return Task.CompletedTask;
    }

    private bool IsActive(WorkItemViewModel item) => _activeId == item.Id;

    private int GetTabIndex(WorkItemViewModel item) => IsActive(item) ? 0 : -1;

    private void MoveActive(int delta)
    {
        var list = FilteredResults;
        if (list.Count == 0)
            return;
        var index = _activeId is null ? 0 : list.FindIndex(w => w.Id == _activeId);
        var next = Math.Clamp(index + delta, 0, list.Count - 1);
        _activeId = list[next].Id;
        StateHasChanged();
    }

    private readonly List<(string Label, DependencyType Value)> DependencyTypes = new()
    {
        ("Blocks", DependencyType.Blocks),
        ("Relates", DependencyType.Relates)
    };
}

Add explicit error handling in dialog to surface backend validation errors without closing:

```razor
@if (!string.IsNullOrWhiteSpace(_error))
{
    <RadzenAlert Severity="AlertSeverity.Error" Text="@_error" />
}
```

```csharp
private string? _error;

private async Task SelectItem(WorkItemViewModel item)
{
    _error = null;
    try
    {
        var request = BuildRequest(item);
        await DependencyStore.CreateAsync(request, CancellationToken.None);
        DialogService.Close(true);
    }
    catch (ValidationException ex)
    {
        _error = ex.UserMessage;
    }
    catch (Exception ex)
    {
        _error = ex.Message;
    }
}
```

Add debounce with a minimal handler:

```csharp
private async Task HandleSearchChanged(string value)
{
    _debounceCts?.Cancel();
    _debounceCts = new CancellationTokenSource();
    var token = _debounceCts.Token;
    SearchText = value;
    try
    {
        await Task.Delay(200, token);
        if (!token.IsCancellationRequested)
            StateHasChanged();
    }
    catch (TaskCanceledException) { }
}
```

Notes:
- Blocks relationship direction matters; Relates should be symmetric (backend treats as a type, but direction can still be stored).
- If you want Relates to always store `BlockingItemId = WorkItemId`, adjust selection logic.
- Ensure UI labels clarify direction (e.g., "Relates to") if canonical direction is enforced.

---

### 5) Create Dependency Manager Component (Composes Row + Dialog)

#### 5.1 New Component File
Create `frontend/ProjectManagement.Components/Dependencies/DependencyManager.razor`.

Responsibilities:
- Accept current work item ID and project ID.
- Query store for `blocking` and `blocked` lists.
- Render two sections with remove actions.
- Trigger add dialog.
- Call `RefreshAsync` on first render and when `WorkItemId` changes.
- Use connectivity state to disable actions when offline and show a visible notice.

Snippet (core structure with state + refresh):

```razor
@using ProjectManagement.Core.Interfaces
@using ProjectManagement.Core.Models
@using ProjectManagement.Core.ViewModels
@inject IDependencyStore DependencyStore
@inject AppState AppState
@inject DialogService DialogService
@implements IDisposable

<div class="dependency-manager">
    <div class="section">
        <div class="section-header">
            <span>Blocking this item</span>
            <RadzenButton Text="Add" Icon="add" Size="ButtonSize.Small"
                          ButtonStyle="ButtonStyle.Secondary"
                          Click="@OpenAddDialog" />
        </div>
        @if (Blocking.Count == 0)
        {
            <div class="dependency-empty">No items are blocking this work item.</div>
        }
        else
        {
            @foreach (var dep in Blocking)
            {
                <DependencyRow Dependency="dep" CurrentWorkItemId="WorkItemId"
                               WorkItemLookup="ResolveWorkItem"
                               OnRemove="HandleRemove" />
            }
        }
    </div>

    <div class="section">
        <div class="section-header">
            <span>Blocked by this item</span>
        </div>
        @if (Blocked.Count == 0)
        {
            <div class="dependency-empty">This work item is not blocking anything.</div>
        }
        else
        {
            @foreach (var dep in Blocked)
            {
                <DependencyRow Dependency="dep" CurrentWorkItemId="WorkItemId"
                               WorkItemLookup="ResolveWorkItem"
                               OnRemove="HandleRemove" />
            }
        }
    </div>
</div>

@code {
    [Parameter, EditorRequired] public Guid WorkItemId { get; set; }
    [Parameter, EditorRequired] public Guid ProjectId { get; set; }

    private IReadOnlyList<Dependency> Blocking => DependencyStore.GetBlocking(WorkItemId);
    private IReadOnlyList<Dependency> Blocked => DependencyStore.GetBlocked(WorkItemId);
    private bool _loading;
    private string? _error;
    private bool _isOnline;

    protected override void OnInitialized()
    {
        DependencyStore.OnChanged += HandleChanged;
        _isOnline = ResolveOnlineState();
    }

    protected override async Task OnParametersSetAsync()
    {
        _loading = true;
        _error = null;
        try
        {
            await DependencyStore.RefreshAsync(WorkItemId, CancellationToken.None);
        }
        catch (Exception ex)
        {
            _error = ex.Message;
        }
        finally
        {
            _loading = false;
        }
    }

    private void HandleChanged()
    {
        _isOnline = ResolveOnlineState();
        _ = InvokeAsync(StateHasChanged);
    }

    private bool ResolveOnlineState()
    {
        // Replace with actual connectivity source confirmed in Step 1
        return true;
    }

    private WorkItemViewModel? ResolveWorkItem(Guid id)
    {
        var item = AppState.WorkItems.GetById(id);
        return item is null ? null : AppState.ViewModelFactory.Create(item);
    }

    private async Task OpenAddDialog()
    {
        if (!_isOnline)
            return;
        await DialogService.OpenAsync<AddDependencyDialog>(
            "Add Dependency",
            new Dictionary<string, object>
            {
                { "WorkItemId", WorkItemId },
                { "ProjectId", ProjectId }
            },
            new DialogOptions { Width = "600px" });
    }

    private async Task HandleRemove(Guid dependencyId)
    {
        if (!_isOnline)
            return;
        await DependencyStore.DeleteAsync(dependencyId, CancellationToken.None);
    }

    public void Dispose()
    {
        DependencyStore.OnChanged -= HandleChanged;
    }
}
```

Add UX states around the list rendering:
- Show loading placeholder when `_loading` is true.
- Show `_error` inline with retry button.
- Disable Add/Remove when offline or pending and show a clear offline message.

Add minimal snippet for error + loading:

```razor
@if (_loading)
{
    <div class="dependency-empty">Loading dependencies...</div>
}
else if (!string.IsNullOrWhiteSpace(_error))
{
    <div class="dependency-empty">@_error</div>
    <RadzenButton Text="Retry" Click="@(() => DependencyStore.RefreshAsync(WorkItemId, CancellationToken.None))" />
}
```

---

### 6) Wire into Work Item Detail Sidebar (Depends on Manager)

Add to `frontend/ProjectManagement.Wasm/Pages/WorkItemDetail.razor` sidebar section between Details and Activity.

Snippet:

```razor
<div class="content-card mt-4">
    <h3 class="content-card-title">Dependencies</h3>
    <DependencyManager WorkItemId="@WorkItemId" ProjectId="@_workItem.ProjectId" />
</div>
```

Also trigger `DependencyStore.RefreshAsync` in the load flow (if you want prefetch outside the manager):

```csharp
@inject IDependencyStore DependencyStore

// In LoadWorkItemAsync
await DependencyStore.RefreshAsync(WorkItemId, CancellationToken.None);
```

Only run refresh after `_workItem` is known and for the same project.

---

### 7) Show Blocked Indicator on Cards (Depends on store state)

Add `BlockedIndicator` to Kanban card for quick visibility.

File: `frontend/ProjectManagement.Components/WorkItems/KanbanCard.razor`

Snippet:

```razor
<RadzenStack Orientation="Orientation.Horizontal"
             Gap="0.25rem"
             AlignItems="AlignItems.Center">
    <PriorityBadge Priority="@Item.Priority" ShowLabel="false" />
    @if (Item.StoryPoints.HasValue)
    {
        <RadzenBadge BadgeStyle="BadgeStyle.Info"
                     Text="@Item.StoryPoints.Value.ToString()"
                     title="Story Points" />
    }
    <BlockedIndicator WorkItemId="@Item.Id" />
</RadzenStack>
```

If this adds too much visual noise, keep it for WorkItemRow only or add a tooltip-only state.

---

## Testing Plan (Production-Grade)

### Unit / Component Tests (bUnit)
- `DependencyManager` renders both sections and reacts to `OnChanged`.
- `DependencyRow` shows correct title, type badge, and pending state.
- `AddDependencyDialog` filters current item + existing links + search text.
- `AddDependencyDialog` respects listbox active option, roving tabindex, and Enter key selection.
- `DependencyManager` disables actions when offline and shows a visible message.
- Relates canonical direction is correctly applied in request construction.

Example bUnit test snippets:

```csharp
[Fact]
public void DependencyRow_DisablesDelete_WhenPending()
{
    using var ctx = new TestContext();
    var store = Substitute.For<IDependencyStore>();
    store.IsPending(Arg.Any<Guid>()).Returns(true);
    ctx.Services.AddSingleton(store);

    var dep = new Dependency { Id = Guid.NewGuid(), BlockingItemId = Guid.NewGuid(), BlockedItemId = Guid.NewGuid(), Type = DependencyType.Blocks };
    var cut = ctx.RenderComponent<DependencyRow>(parameters => parameters
        .Add(p => p.Dependency, dep)
        .Add(p => p.CurrentWorkItemId, dep.BlockedItemId)
        .Add(p => p.WorkItemLookup, _ => null)
        .Add(p => p.OnRemove, EventCallback.Factory.Create<Guid>(ctx, _ => { })));

    cut.Find("button").HasAttribute("disabled");
}
```

```csharp
[Fact]
public void AddDependencyDialog_FiltersOutCurrentItem()
{
    using var ctx = new TestContext();
    var appState = TestStateFactory.CreateAppStateWithProjectItems(3);
    ctx.Services.AddSingleton(appState);
    ctx.Services.AddSingleton(Substitute.For<IDependencyStore>());
    ctx.Services.AddSingleton(Substitute.For<DialogService>());

    var current = appState.WorkItems.GetByProject(appState.CurrentProjectId).First();
    var cut = ctx.RenderComponent<AddDependencyDialog>(parameters => parameters
        .Add(p => p.WorkItemId, current.Id)
        .Add(p => p.ProjectId, current.ProjectId));

    cut.Markup.Should().NotContain(current.Title);
}
```

### Integration Tests (Services)
- `DependencyStore` optimistic create/delete + rollback on failure (already exists, extend as needed).
- Failure flow: validation error maps to user-visible message without closing dialog.
- Retry flow: refresh after failure rehydrates lists without duplicates.

### Manual Verification Matrix
- Create Blocks dependency → appears in “Blocking this item” section with Blocks badge.
- Create Relates dependency → appears with Relates badge, does not mark blocked.
- Remove dependency → disappears; delete button disabled while pending.
- Keyboard: tab to results + Enter selects item.
- Activity feed shows entry for create/delete.
- Offline: Add/Remove disabled; clear message displayed; no request sent.

### Error Handling Scenarios
- Try to create duplicate dependency → inline error from backend, dialog stays open.
- Try to create circular Blocks dependency → inline error from backend.
- Network drop mid-create → store throws; dialog shows error and stays open.
- Refresh fails → error state appears with Retry; Retry triggers a new refresh and clears error on success.

---

## Accessibility Requirements
- All buttons have `aria-label`.
- Listbox/option roles for search results.
- Keyboard navigation for selection (Enter to select) plus roving tabindex or `aria-activedescendant` with visible active state.
- Error and empty states are visible and readable.

---

## Success Criteria
- [ ] Dependency manager renders in Work Item detail sidebar.
- [ ] Add dialog can create Blocks and Relates dependencies within the project.
- [ ] Lists update after create/delete without page reload.
- [ ] Pending state is visible during server operations.
- [ ] Activity feed reflects create/delete actions (already emitted server-side).
- [ ] Blocked indicator appears on cards.
- [ ] Search results are keyboard accessible and screen-reader labeled.
- [ ] Errors are surfaced inline and via toast without breaking the UI.

## Risks and Mitigations
- Duplicate or reversed dependencies: filter candidates in dialog + rely on backend validation errors.
- Relates direction confusion: enforce canonical direction in dialog, display with explicit label.
- Large project lists: prefer backend search/paging; if unavailable, cap list size, document UX limit, and consider warning for large projects.
- Missing or mismatched store APIs: validate signatures in Step 1 before UI work; assign ownership for missing contracts.
- Offline state ambiguity: use a single authoritative connectivity source and surface it in UI.

## Files to Create/Modify

**Create**
- `frontend/ProjectManagement.Components/Dependencies/DependencyManager.razor`
- `frontend/ProjectManagement.Components/Dependencies/DependencyRow.razor`
- `frontend/ProjectManagement.Components/Dependencies/AddDependencyDialog.razor`

**Modify**
- `frontend/ProjectManagement.Wasm/Pages/WorkItemDetail.razor`
- `frontend/ProjectManagement.Components/WorkItems/KanbanCard.razor`
- (If needed) RCL stylesheet to import `dependencies.css`

---

## QA Notes (for later)
- Test create/delete dependency updates UI immediately (optimistic state).
- Test Blocks vs Relates rendering and blocked indicator.
- Test that cross-project items are not offered in dialog.
- Test that duplicate/invalid dependencies surface backend validation errors gracefully.
