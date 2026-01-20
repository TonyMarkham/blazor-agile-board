# Session 30: Work Item UI - Implementation Plan (v2 - Corrected)

**Goal**: Functional work item management with Radzen components and real-time updates

**Target Quality**: 9.25+/10 production-grade

**Build Order**: Files are built in strict dependency order. No file references code that hasn't been written yet.

**Prerequisites**: Session 20 complete (88 tests passing, WebSocket client, state management)

---

## Critical Corrections from Original Plan

This plan corrects the following issues discovered during Session 20 implementation review:

| Issue | Original Plan | Corrected Approach |
|-------|---------------|-------------------|
| Store return types | Assumed ViewModels | Session 20 returns raw `WorkItem`/`Sprint` - we add ViewModel layer in Session 30 |
| `IAppState` interface | Referenced non-existent interface | Use concrete `AppState` class directly |
| Sprint date nullability | Assumed `DateTime?` | Actual is `DateTime` (non-nullable) - adjust SprintViewModel |
| Radzen `Change` events | Used `Change="@MethodName"` | Must use `Change="@(_ => MethodName())"` for void methods |
| `RadzenNumeric` binding | Used `Change` event | Use `@bind-Value` with `ValueChanged` for proper two-way binding |

---

## Complete Ordered File List

| # | File | Depends On |
|---|------|------------|
| 1 | `ProjectManagement.Core/ViewModels/IViewModel.cs` | — |
| 2 | `ProjectManagement.Core/ViewModels/WorkItemViewModel.cs` | #1, WorkItem |
| 3 | `ProjectManagement.Core/ViewModels/SprintViewModel.cs` | #1, Sprint |
| 4 | `ProjectManagement.Core/ViewModels/ViewModelFactory.cs` | #2, #3, IWorkItemStore, ISprintStore |
| 5 | `ProjectManagement.Components/wwwroot/css/app.css` | — |
| 6 | `ProjectManagement.Components/Shared/OfflineBanner.razor` | AppState |
| 7 | `ProjectManagement.Components/Shared/EmptyState.razor` | — |
| 8 | `ProjectManagement.Components/Shared/LoadingButton.razor` | — |
| 9 | `ProjectManagement.Components/Shared/DebouncedTextBox.razor` | — |
| 10 | `ProjectManagement.Components/Shared/ConfirmDialog.razor` | — |
| 11 | `ProjectManagement.Components/Shared/ProjectDetailSkeleton.razor` | — |
| 12 | `ProjectManagement.Components/wwwroot/css/work-items.css` | — |
| 13 | `ProjectManagement.Components/WorkItems/WorkItemTypeIcon.razor` | WorkItemType |
| 14 | `ProjectManagement.Components/WorkItems/WorkItemStatusBadge.razor` | — |
| 15 | `ProjectManagement.Components/WorkItems/PriorityBadge.razor` | — |
| 16 | `ProjectManagement.Components/WorkItems/WorkItemRow.razor` | #2, #13, #14, #15 |
| 17 | `ProjectManagement.Components/WorkItems/KanbanCard.razor` | #2, #13, #15 |
| 18 | `ProjectManagement.Components/WorkItems/VersionConflictDialog.razor` | DialogService |
| 19 | `ProjectManagement.Components/WorkItems/WorkItemDialog.razor` | #2, #3, #4, #8, #18 |
| 20 | `ProjectManagement.Components/wwwroot/css/kanban.css` | — |
| 21 | `ProjectManagement.Components/WorkItems/WorkItemList.razor` | #2, #4, #7, #9, #10, #16, #19 |
| 22 | `ProjectManagement.Components/WorkItems/KanbanColumn.razor` | #2, #17 |
| 23 | `ProjectManagement.Components/WorkItems/KanbanBoard.razor` | #2, #4, #19, #22 |
| 24 | `ProjectManagement.Components/wwwroot/css/layout.css` | — |
| 25 | `ProjectManagement.Wasm/Layout/NavMenu.razor` (update) | AppState |
| 26 | `ProjectManagement.Wasm/Layout/MainLayout.razor` (update) | #6, #25 |
| 27 | `ProjectManagement.Wasm/Pages/Home.razor` (update) | #2, #4, #7, #8, #19 |
| 28 | `ProjectManagement.Wasm/Pages/ProjectDetail.razor` | #2, #4, #8, #11, #19, #21, #23 |
| 29 | `ProjectManagement.Wasm/Pages/WorkItemDetail.razor` | #2, #4, #7, #10, #11, #13, #14, #15, #16, #19 |
| 30 | `ProjectManagement.Components.Tests/ViewModels/ViewModelFactoryTests.cs` | #4 |
| 31 | `ProjectManagement.Components.Tests/Shared/LeafComponentTests.cs` | #6-15 |
| 32 | `ProjectManagement.Components.Tests/WorkItems/RowCardDialogTests.cs` | #16-19 |
| 33 | `ProjectManagement.Components.Tests/WorkItems/ListBoardTests.cs` | #21-23 |
| 34 | `ProjectManagement.Components.Tests/Pages/PageTests.cs` | #27-29 |

**Total: 34 files, 70+ tests**

---

## Session 20 Types Available (ACTUAL - Verified)

**From ProjectManagement.Core.Models:**
- `WorkItem` - Immutable record implementing 9 interfaces
- `WorkItemType` - Enum: `Project`, `Epic`, `Story`, `Task`
- `Sprint` - Immutable record with **non-nullable** `StartDate`/`EndDate`
- `SprintStatus` - Enum: `Planned`, `Active`, `Completed`
- `CreateWorkItemRequest`, `UpdateWorkItemRequest` - DTOs with `required` properties
- `ConnectionState` - Enum: `Disconnected`, `Connecting`, `Connected`, `Reconnecting`, `Closed`

**From ProjectManagement.Core.Interfaces:**
- `IWorkItemStore` - Returns `WorkItem` (NOT ViewModel)
- `ISprintStore` - Returns `Sprint` (NOT ViewModel)
- `IWebSocketClient` - WebSocket operations

**From ProjectManagement.Services.State:**
- `AppState` - Concrete class (no interface), exposes `WorkItems`, `Sprints`, `ConnectionState`, `OnStateChanged`, `OnConnectionStateChanged`
- `WorkItemStore` - Has internal `_pendingUpdates` dictionary (not exposed)

**CRITICAL**: The `_pendingUpdates` dictionary in `WorkItemStore` is private. Session 30 must either:
1. Add a method to expose pending state (recommended), OR
2. Track pending state separately in UI components

We choose option 1: Add `IsPending(Guid id)` method to stores.

---

## Files 1-4: ViewModel Infrastructure

### 1. ProjectManagement.Core/ViewModels/IViewModel.cs

```csharp
namespace ProjectManagement.Core.ViewModels;

/// <summary>
/// Base interface for all view models.
/// View models combine domain models with transient UI state.
/// </summary>
/// <typeparam name="TModel">The underlying domain model type</typeparam>
public interface IViewModel<out TModel> where TModel : class
{
    /// <summary>
    /// The underlying domain model (from server/database).
    /// </summary>
    TModel Model { get; }

    /// <summary>
    /// True when this item has pending changes being synced to the server.
    /// Used for optimistic UI feedback (shimmer, disabled buttons, etc.)
    /// </summary>
    bool IsPendingSync { get; }
}
```

### 2. ProjectManagement.Core/ViewModels/WorkItemViewModel.cs

```csharp
using ProjectManagement.Core.Models;

namespace ProjectManagement.Core.ViewModels;

/// <summary>
/// View model for WorkItem. Combines domain data with UI state.
/// Exposes commonly-accessed properties directly for Razor binding convenience.
/// </summary>
public sealed class WorkItemViewModel : IViewModel<WorkItem>
{
    public WorkItemViewModel(WorkItem model, bool isPendingSync = false)
    {
        ArgumentNullException.ThrowIfNull(model);
        Model = model;
        IsPendingSync = isPendingSync;
    }

    public WorkItem Model { get; }
    public bool IsPendingSync { get; }

    // Convenience accessors - avoids .Model.Property in Razor
    public Guid Id => Model.Id;
    public WorkItemType ItemType => Model.ItemType;
    public string Title => Model.Title;
    public string? Description => Model.Description;
    public string Status => Model.Status;
    public string Priority => Model.Priority;
    public int? StoryPoints => Model.StoryPoints;
    public int Position => Model.Position;
    public Guid ProjectId => Model.ProjectId;
    public Guid? ParentId => Model.ParentId;
    public Guid? SprintId => Model.SprintId;
    public Guid? AssigneeId => Model.AssigneeId;
    public int Version => Model.Version;
    public DateTime CreatedAt => Model.CreatedAt;
    public DateTime UpdatedAt => Model.UpdatedAt;
    public DateTime? DeletedAt => Model.DeletedAt;

    // Computed properties for UI
    public bool IsDeleted => Model.DeletedAt.HasValue;
    public bool IsCompleted => Model.Status == "done";

    public string StatusDisplayName => Status switch
    {
        "backlog" => "Backlog",
        "todo" => "To Do",
        "in_progress" => "In Progress",
        "review" => "Review",
        "done" => "Done",
        _ => Status
    };

    public string PriorityDisplayName => Priority switch
    {
        "critical" => "Critical",
        "high" => "High",
        "medium" => "Medium",
        "low" => "Low",
        _ => Priority
    };
}
```

### 3. ProjectManagement.Core/ViewModels/SprintViewModel.cs

```csharp
using ProjectManagement.Core.Models;

namespace ProjectManagement.Core.ViewModels;

/// <summary>
/// View model for Sprint. Combines domain data with UI state.
/// Note: Sprint.StartDate and Sprint.EndDate are non-nullable in Session 20.
/// </summary>
public sealed class SprintViewModel : IViewModel<Sprint>
{
    public SprintViewModel(Sprint model, bool isPendingSync = false)
    {
        ArgumentNullException.ThrowIfNull(model);
        Model = model;
        IsPendingSync = isPendingSync;
    }

    public Sprint Model { get; }
    public bool IsPendingSync { get; }

    // Convenience accessors
    public Guid Id => Model.Id;
    public string Name => Model.Name;
    public string? Goal => Model.Goal;
    public Guid ProjectId => Model.ProjectId;
    public DateTime StartDate => Model.StartDate;  // Non-nullable per Session 20
    public DateTime EndDate => Model.EndDate;      // Non-nullable per Session 20
    public SprintStatus Status => Model.Status;
    public DateTime? DeletedAt => Model.DeletedAt;

    // Computed properties for UI display
    public bool IsDeleted => Model.DeletedAt.HasValue;
    public bool IsActive => Model.Status == SprintStatus.Active;
    public bool IsCompleted => Model.Status == SprintStatus.Completed;

    public string DateRangeDisplay => $"{StartDate:MMM d} - {EndDate:MMM d}";

    public int? DaysRemaining => Status == SprintStatus.Active
        ? Math.Max(0, (int)(EndDate.Date - DateTime.UtcNow.Date).TotalDays)
        : null;

    public double ProgressPercent
    {
        get
        {
            if (Status != SprintStatus.Active) return Status == SprintStatus.Completed ? 100 : 0;
            var total = (EndDate - StartDate).TotalDays;
            if (total <= 0) return 100;
            var elapsed = (DateTime.UtcNow - StartDate).TotalDays;
            return Math.Clamp(elapsed / total * 100, 0, 100);
        }
    }

    public string StatusDisplayName => Status switch
    {
        SprintStatus.Planned => "Planned",
        SprintStatus.Active => "Active",
        SprintStatus.Completed => "Completed",
        _ => Status.ToString()
    };
}
```

### 4. ProjectManagement.Core/ViewModels/ViewModelFactory.cs

This factory creates ViewModels from domain models, checking pending state from stores.

```csharp
using ProjectManagement.Core.Interfaces;
using ProjectManagement.Core.Models;

namespace ProjectManagement.Core.ViewModels;

/// <summary>
/// Factory for creating ViewModels with proper pending state.
/// Injected as a scoped service to access stores.
/// </summary>
public sealed class ViewModelFactory
{
    private readonly IWorkItemStore _workItemStore;
    private readonly ISprintStore _sprintStore;

    public ViewModelFactory(IWorkItemStore workItemStore, ISprintStore sprintStore)
    {
        _workItemStore = workItemStore;
        _sprintStore = sprintStore;
    }

    public WorkItemViewModel Create(WorkItem item)
    {
        ArgumentNullException.ThrowIfNull(item);
        var isPending = _workItemStore.IsPending(item.Id);
        return new WorkItemViewModel(item, isPending);
    }

    public SprintViewModel Create(Sprint sprint)
    {
        ArgumentNullException.ThrowIfNull(sprint);
        var isPending = _sprintStore.IsPending(sprint.Id);
        return new SprintViewModel(sprint, isPending);
    }

    public IReadOnlyList<WorkItemViewModel> CreateMany(IEnumerable<WorkItem> items)
    {
        ArgumentNullException.ThrowIfNull(items);
        return items.Select(Create).ToList();
    }

    public IReadOnlyList<SprintViewModel> CreateMany(IEnumerable<Sprint> sprints)
    {
        ArgumentNullException.ThrowIfNull(sprints);
        return sprints.Select(Create).ToList();
    }
}
```

**REQUIRED STORE UPDATES** (before implementing ViewModelFactory):

Add to `IWorkItemStore`:
```csharp
/// <summary>Check if a work item has a pending optimistic update.</summary>
bool IsPending(Guid id);
```

Add to `ISprintStore`:
```csharp
/// <summary>Check if a sprint has a pending optimistic update.</summary>
bool IsPending(Guid id);
```

Implement in `WorkItemStore`:
```csharp
public bool IsPending(Guid id) => _pendingUpdates.ContainsKey(id);
```

Implement in `SprintStore`:
```csharp
public bool IsPending(Guid id) => _pendingUpdates.ContainsKey(id);
```

**Register ViewModelFactory in Program.cs:**
```csharp
builder.Services.AddScoped<ViewModelFactory>();
```

---

## Files 5-11: CSS and Shared Components

### 5. ProjectManagement.Components/wwwroot/css/app.css

```css
/* ===== Skip link for accessibility ===== */
.skip-link {
    position: absolute;
    top: -40px;
    left: 0;
    background: var(--rz-primary);
    color: white;
    padding: 8px 16px;
    z-index: 10000;
    text-decoration: none;
    border-radius: 0 0 4px 0;
}

.skip-link:focus {
    top: 0;
}

/* ===== Visually hidden but accessible ===== */
.visually-hidden {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
}

/* ===== Offline banner ===== */
.offline-banner {
    background: var(--rz-warning-lighter);
    border-bottom: 1px solid var(--rz-warning);
    padding: 0.5rem 1rem;
    color: var(--rz-warning-darker);
}

/* ===== Empty state ===== */
.empty-state {
    padding: 3rem 1rem;
    text-align: center;
}

.empty-state-icon {
    font-size: 3rem;
    color: var(--rz-text-tertiary-color);
}

/* ===== Utilities ===== */
.flex-grow-1 { flex-grow: 1; }
.text-end { text-align: end; }
.text-center { text-align: center; }
.text-muted { color: var(--rz-text-secondary-color); }
.gap-0 { gap: 0; }
.gap-1 { gap: 0.25rem; }
.gap-2 { gap: 0.5rem; }
.gap-3 { gap: 0.75rem; }
.gap-4 { gap: 1rem; }

/* ===== Pending sync shimmer animation ===== */
.pending-sync {
    opacity: 0.7;
    background: linear-gradient(
        90deg,
        var(--rz-base-200) 0%,
        var(--rz-base-100) 50%,
        var(--rz-base-200) 100%
    );
    background-size: 200% 100%;
    animation: shimmer 1.5s infinite;
}

@keyframes shimmer {
    0% { background-position: 200% 0; }
    100% { background-position: -200% 0; }
}

/* ===== Focus visible for keyboard navigation ===== */
.focus-visible:focus-visible {
    outline: 2px solid var(--rz-primary);
    outline-offset: 2px;
}

/* ===== Reduced motion ===== */
@media (prefers-reduced-motion: reduce) {
    .pending-sync {
        animation: none;
    }
}
```

### 6. ProjectManagement.Components/Shared/OfflineBanner.razor

```razor
@using ProjectManagement.Core.Models
@using ProjectManagement.Services.State
@inject AppState AppState
@implements IDisposable

@if (_showBanner)
{
    <div class="offline-banner" role="alert" aria-live="polite">
        <RadzenStack Orientation="Orientation.Horizontal" AlignItems="AlignItems.Center" Gap="0.5rem">
            <RadzenIcon Icon="cloud_off" />
            <span>You're offline. Changes will sync when reconnected.</span>
            @if (_isReconnecting)
            {
                <RadzenProgressBarCircular ShowValue="false" Mode="ProgressBarMode.Indeterminate" Size="ProgressBarCircularSize.Small" />
            }
        </RadzenStack>
    </div>
}

@code {
    private bool _showBanner;
    private bool _isReconnecting;

    protected override void OnInitialized()
    {
        UpdateBannerState(AppState.ConnectionState);
        AppState.OnConnectionStateChanged += HandleConnectionStateChanged;
    }

    private void HandleConnectionStateChanged(ConnectionState state)
    {
        UpdateBannerState(state);
        InvokeAsync(StateHasChanged);
    }

    private void UpdateBannerState(ConnectionState state)
    {
        _showBanner = state != ConnectionState.Connected;
        _isReconnecting = state == ConnectionState.Reconnecting;
    }

    public void Dispose()
    {
        AppState.OnConnectionStateChanged -= HandleConnectionStateChanged;
    }
}
```

### 7. ProjectManagement.Components/Shared/EmptyState.razor

```razor
<div class="empty-state" role="status" aria-label="@Title">
    <RadzenStack AlignItems="AlignItems.Center" Gap="1rem">
        <RadzenIcon Icon="@Icon" class="empty-state-icon" />
        <RadzenText TextStyle="TextStyle.H6" class="m-0">@Title</RadzenText>
        @if (!string.IsNullOrWhiteSpace(Description))
        {
            <RadzenText TextStyle="TextStyle.Body2" class="text-muted text-center" Style="max-width: 300px;">
                @Description
            </RadzenText>
        }
        @if (OnAction.HasDelegate)
        {
            <RadzenButton Text="@ActionText"
                          Icon="@ActionIcon"
                          Click="@HandleAction"
                          ButtonStyle="ButtonStyle.Primary" />
        }
    </RadzenStack>
</div>

@code {
    [Parameter, EditorRequired]
    public string Icon { get; set; } = "inbox";

    [Parameter, EditorRequired]
    public string Title { get; set; } = "No items";

    [Parameter]
    public string? Description { get; set; }

    [Parameter]
    public string ActionText { get; set; } = "Create";

    [Parameter]
    public string ActionIcon { get; set; } = "add";

    [Parameter]
    public EventCallback OnAction { get; set; }

    private async Task HandleAction()
    {
        if (OnAction.HasDelegate)
        {
            await OnAction.InvokeAsync();
        }
    }
}
```

### 8. ProjectManagement.Components/Shared/LoadingButton.razor

```razor
@using ProjectManagement.Core.Models

<RadzenButton Text="@DisplayText"
              Icon="@DisplayIcon"
              IsBusy="@IsBusy"
              Disabled="@IsDisabled"
              ButtonStyle="@ButtonStyle"
              Size="@Size"
              Click="@HandleClick"
              title="@Tooltip"
              @attributes="AdditionalAttributes" />

@code {
    [Parameter, EditorRequired]
    public string Text { get; set; } = "";

    [Parameter]
    public string LoadingText { get; set; } = "Loading...";

    [Parameter]
    public string Icon { get; set; } = "";

    [Parameter]
    public bool IsBusy { get; set; }

    [Parameter]
    public bool Disabled { get; set; }

    [Parameter]
    public ConnectionState ConnectionState { get; set; } = ConnectionState.Connected;

    [Parameter]
    public ButtonStyle ButtonStyle { get; set; } = ButtonStyle.Primary;

    [Parameter]
    public ButtonSize Size { get; set; } = ButtonSize.Medium;

    [Parameter]
    public EventCallback<MouseEventArgs> OnClick { get; set; }

    [Parameter(CaptureUnmatchedValues = true)]
    public Dictionary<string, object>? AdditionalAttributes { get; set; }

    private bool IsConnected => ConnectionState == ConnectionState.Connected;
    private bool IsDisabled => IsBusy || Disabled || !IsConnected;
    private string DisplayText => IsBusy ? LoadingText : Text;
    private string DisplayIcon => IsBusy ? "" : Icon;

    private string Tooltip => !IsConnected
        ? "Offline - action unavailable"
        : IsBusy
            ? "Please wait..."
            : Text;

    private async Task HandleClick(MouseEventArgs args)
    {
        if (!IsDisabled && OnClick.HasDelegate)
        {
            await OnClick.InvokeAsync(args);
        }
    }
}
```

### 9. ProjectManagement.Components/Shared/DebouncedTextBox.razor

```razor
@implements IDisposable

<RadzenTextBox Value="@_currentValue"
               Placeholder="@Placeholder"
               Style="@Style"
               Disabled="@Disabled"
               Change="@HandleChange"
               @attributes="AdditionalAttributes" />

@code {
    [Parameter]
    public string Value { get; set; } = "";

    [Parameter]
    public EventCallback<string> ValueChanged { get; set; }

    [Parameter]
    public string Placeholder { get; set; } = "";

    [Parameter]
    public string Style { get; set; } = "";

    [Parameter]
    public bool Disabled { get; set; }

    [Parameter]
    public int DebounceMs { get; set; } = 300;

    [Parameter(CaptureUnmatchedValues = true)]
    public Dictionary<string, object>? AdditionalAttributes { get; set; }

    private string _currentValue = "";
    private CancellationTokenSource? _debounceCts;

    protected override void OnParametersSet()
    {
        // Only update if the external value changed (not from our own debounce)
        if (_currentValue != Value && _debounceCts is null)
        {
            _currentValue = Value;
        }
    }

    private async Task HandleChange(string newValue)
    {
        _currentValue = newValue;

        // Cancel any pending debounce
        _debounceCts?.Cancel();
        _debounceCts?.Dispose();
        _debounceCts = new CancellationTokenSource();

        try
        {
            await Task.Delay(DebounceMs, _debounceCts.Token);

            // Debounce completed, emit the value
            if (ValueChanged.HasDelegate)
            {
                await ValueChanged.InvokeAsync(newValue);
            }

            _debounceCts?.Dispose();
            _debounceCts = null;
        }
        catch (TaskCanceledException)
        {
            // Debounce was cancelled by new input, ignore
        }
    }

    public void Dispose()
    {
        _debounceCts?.Cancel();
        _debounceCts?.Dispose();
    }
}
```

### 10. ProjectManagement.Components/Shared/ConfirmDialog.razor

```razor
<RadzenStack Gap="1rem">
    <RadzenText>@Message</RadzenText>

    @if (!string.IsNullOrWhiteSpace(WarningMessage))
    {
        <RadzenAlert AlertStyle="AlertStyle.Warning" Shade="Shade.Light" Size="AlertSize.Small">
            @WarningMessage
        </RadzenAlert>
    }

    <RadzenStack Orientation="Orientation.Horizontal" Gap="0.5rem" JustifyContent="JustifyContent.End">
        <RadzenButton Text="@CancelText"
                      ButtonStyle="ButtonStyle.Light"
                      Click="@HandleCancel"
                      Disabled="@IsBusy" />
        <RadzenButton Text="@ConfirmText"
                      ButtonStyle="@ConfirmButtonStyle"
                      Click="@HandleConfirm"
                      IsBusy="@IsBusy" />
    </RadzenStack>
</RadzenStack>

@code {
    [Parameter, EditorRequired]
    public string Message { get; set; } = "";

    [Parameter]
    public string? WarningMessage { get; set; }

    [Parameter]
    public string ConfirmText { get; set; } = "Confirm";

    [Parameter]
    public string CancelText { get; set; } = "Cancel";

    [Parameter]
    public ButtonStyle ConfirmButtonStyle { get; set; } = ButtonStyle.Primary;

    [Parameter]
    public bool IsBusy { get; set; }

    [Parameter]
    public EventCallback OnConfirm { get; set; }

    [Parameter]
    public EventCallback OnCancel { get; set; }

    private async Task HandleConfirm()
    {
        if (OnConfirm.HasDelegate)
        {
            await OnConfirm.InvokeAsync();
        }
    }

    private async Task HandleCancel()
    {
        if (OnCancel.HasDelegate)
        {
            await OnCancel.InvokeAsync();
        }
    }
}
```

### 11. ProjectManagement.Components/Shared/ProjectDetailSkeleton.razor

```razor
<div role="status" aria-label="Loading project details" aria-busy="true">
    <RadzenStack Gap="1rem">
        <RadzenRow AlignItems="AlignItems.Center">
            <RadzenColumn Size="12" SizeMD="8">
                <RadzenStack Orientation="Orientation.Horizontal" AlignItems="AlignItems.Center" Gap="0.5rem">
                    <RadzenSkeleton Shape="SkeletonShape.Circle" Width="24px" Height="24px" />
                    <RadzenSkeleton Width="200px" Height="32px" />
                </RadzenStack>
            </RadzenColumn>
            <RadzenColumn Size="12" SizeMD="4" class="text-end">
                <RadzenSkeleton Width="140px" Height="36px" Style="display: inline-block;" />
            </RadzenColumn>
        </RadzenRow>

        <RadzenRow>
            <RadzenColumn Size="12">
                <RadzenStack Gap="0.5rem">
                    @for (var i = 0; i < RowCount; i++)
                    {
                        <RadzenSkeleton Width="100%" Height="@($"{RowHeight}px")" />
                    }
                </RadzenStack>
            </RadzenColumn>
        </RadzenRow>
    </RadzenStack>
    <span class="visually-hidden">Loading...</span>
</div>

@code {
    [Parameter]
    public int RowCount { get; set; } = 5;

    [Parameter]
    public int RowHeight { get; set; } = 48;
}
```

---

## Files 12-15: Work Item CSS and Badge Components

### 12. ProjectManagement.Components/wwwroot/css/work-items.css

```css
/* ===== Work item list ===== */
.work-item-list {
    border: 1px solid var(--rz-border-color);
    border-radius: 8px;
    overflow: hidden;
    background: var(--rz-surface);
}

.work-item-header {
    display: flex;
    background: var(--rz-base-200);
    border-bottom: 1px solid var(--rz-border-color);
    font-weight: 600;
    font-size: 0.875rem;
}

.work-item-row {
    display: flex;
    border-bottom: 1px solid var(--rz-border-color);
    transition: background-color 0.15s ease;
}

.work-item-row:last-child {
    border-bottom: none;
}

.work-item-row:hover {
    background-color: var(--rz-secondary-lighter);
}

.work-item-row:focus-visible {
    outline: 2px solid var(--rz-primary);
    outline-offset: -2px;
    z-index: 1;
    position: relative;
}

.work-item-row.status-done {
    opacity: 0.7;
}

.work-item-row.status-done .work-item-title {
    text-decoration: line-through;
}

.work-item-row.pending-sync {
    opacity: 0.7;
    background: linear-gradient(
        90deg,
        var(--rz-base-200) 0%,
        var(--rz-base-100) 50%,
        var(--rz-base-200) 100%
    );
    background-size: 200% 100%;
    animation: shimmer 1.5s infinite;
}

/* ===== Work item cells ===== */
.work-item-cell {
    padding: 0.75rem;
    display: flex;
    align-items: center;
}

.type-cell { width: 60px; justify-content: center; flex-shrink: 0; }
.title-cell { flex: 1; min-width: 200px; overflow: hidden; }
.status-cell { width: 120px; flex-shrink: 0; }
.priority-cell { width: 100px; flex-shrink: 0; }
.points-cell { width: 80px; justify-content: center; flex-shrink: 0; }
.actions-cell { width: 100px; justify-content: flex-end; flex-shrink: 0; }

.work-item-title {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}

.hierarchy-indent {
    display: inline-block;
    flex-shrink: 0;
}

/* ===== Responsive ===== */
@media (max-width: 768px) {
    .priority-cell,
    .points-cell {
        display: none;
    }

    .actions-cell {
        width: 80px;
    }

    .title-cell {
        min-width: 150px;
    }
}

/* ===== Reduced motion ===== */
@media (prefers-reduced-motion: reduce) {
    .work-item-row.pending-sync {
        animation: none;
    }
}
```

### 13. ProjectManagement.Components/WorkItems/WorkItemTypeIcon.razor

```razor
@using ProjectManagement.Core.Models

<span class="type-icon" title="@TypeName">
    <RadzenIcon Icon="@IconName" Style="@IconStyle" />
    <span class="visually-hidden">@TypeName</span>
</span>

@code {
    [Parameter, EditorRequired]
    public WorkItemType Type { get; set; }

    [Parameter]
    public string? Size { get; set; }

    private string TypeName => Type switch
    {
        WorkItemType.Project => "Project",
        WorkItemType.Epic => "Epic",
        WorkItemType.Story => "Story",
        WorkItemType.Task => "Task",
        _ => "Unknown"
    };

    private string IconName => Type switch
    {
        WorkItemType.Project => "folder",
        WorkItemType.Epic => "rocket_launch",
        WorkItemType.Story => "description",
        WorkItemType.Task => "task_alt",
        _ => "help_outline"
    };

    private string IconColor => Type switch
    {
        WorkItemType.Project => "var(--rz-primary)",
        WorkItemType.Epic => "#9c27b0",
        WorkItemType.Story => "#2196f3",
        WorkItemType.Task => "#4caf50",
        _ => "var(--rz-text-secondary-color)"
    };

    private string IconStyle => string.IsNullOrEmpty(Size)
        ? $"color: {IconColor};"
        : $"color: {IconColor}; font-size: {Size};";
}
```

### 14. ProjectManagement.Components/WorkItems/WorkItemStatusBadge.razor

```razor
<RadzenBadge BadgeStyle="@BadgeStyle"
             Text="@DisplayText"
             title="@($"Status: {DisplayText}")" />

@code {
    [Parameter, EditorRequired]
    public string Status { get; set; } = "backlog";

    private BadgeStyle BadgeStyle => Status switch
    {
        "backlog" => BadgeStyle.Secondary,
        "todo" => BadgeStyle.Info,
        "in_progress" => BadgeStyle.Warning,
        "review" => BadgeStyle.Primary,
        "done" => BadgeStyle.Success,
        _ => BadgeStyle.Light
    };

    private string DisplayText => Status switch
    {
        "backlog" => "Backlog",
        "todo" => "To Do",
        "in_progress" => "In Progress",
        "review" => "Review",
        "done" => "Done",
        _ => Status
    };
}
```

### 15. ProjectManagement.Components/WorkItems/PriorityBadge.razor

```razor
<span class="priority-badge" title="@($"Priority: {DisplayText}")">
    <RadzenStack Orientation="Orientation.Horizontal" AlignItems="AlignItems.Center" Gap="0.25rem">
        <RadzenIcon Icon="@IconName" Style="@IconStyle" />
        <span>@DisplayText</span>
    </RadzenStack>
</span>

@code {
    [Parameter, EditorRequired]
    public string Priority { get; set; } = "medium";

    private string IconName => Priority switch
    {
        "critical" => "priority_high",
        "high" => "keyboard_arrow_up",
        "medium" => "remove",
        "low" => "keyboard_arrow_down",
        _ => "remove"
    };

    private string IconColor => Priority switch
    {
        "critical" => "#d32f2f",
        "high" => "#f57c00",
        "medium" => "#1976d2",
        "low" => "#388e3c",
        _ => "var(--rz-text-secondary-color)"
    };

    private string IconStyle => $"color: {IconColor};";

    private string DisplayText => Priority switch
    {
        "critical" => "Critical",
        "high" => "High",
        "medium" => "Medium",
        "low" => "Low",
        _ => Priority
    };
}
```

---

## Files 16-19: Row, Card, and Dialog Components

### 16. ProjectManagement.Components/WorkItems/WorkItemRow.razor

```razor
@using ProjectManagement.Core.ViewModels
@using ProjectManagement.Core.Models

<div class="work-item-row @RowCssClass"
     role="row"
     tabindex="0"
     @onkeydown="HandleKeyDown"
     @onclick="HandleClick"
     aria-label="@AriaLabel"
     aria-busy="@Item.IsPendingSync.ToString().ToLowerInvariant()">

    <div class="work-item-cell type-cell" role="cell">
        <WorkItemTypeIcon Type="@Item.ItemType" />
    </div>

    <div class="work-item-cell title-cell" role="cell">
        <RadzenStack Orientation="Orientation.Horizontal" AlignItems="AlignItems.Center" Gap="0.25rem" Style="overflow: hidden;">
            @if (IndentLevel > 0)
            {
                <span class="hierarchy-indent" style="width: @(IndentLevel * 20)px;" aria-hidden="true"></span>
            }
            <span class="work-item-title">@Item.Title</span>
            @if (Item.IsPendingSync)
            {
                <RadzenProgressBarCircular ShowValue="false"
                                           Mode="ProgressBarMode.Indeterminate"
                                           Size="ProgressBarCircularSize.ExtraSmall"
                                           title="Saving..." />
            }
        </RadzenStack>
    </div>

    <div class="work-item-cell status-cell" role="cell">
        <WorkItemStatusBadge Status="@Item.Status" />
    </div>

    <div class="work-item-cell priority-cell" role="cell">
        <PriorityBadge Priority="@Item.Priority" />
    </div>

    <div class="work-item-cell points-cell" role="cell">
        @if (Item.StoryPoints.HasValue)
        {
            <RadzenBadge BadgeStyle="BadgeStyle.Info" Text="@Item.StoryPoints.Value.ToString()" />
        }
    </div>

    <div class="work-item-cell actions-cell" role="cell">
        <RadzenStack Orientation="Orientation.Horizontal" Gap="0.25rem">
            <RadzenButton Icon="edit"
                          ButtonStyle="ButtonStyle.Light"
                          Size="ButtonSize.Small"
                          Click="@(e => OnEdit.InvokeAsync(Item))"
                          Click:stopPropagation="true"
                          Disabled="@(!IsConnected || Item.IsPendingSync)"
                          title="Edit"
                          aria-label="@($"Edit {Item.Title}")" />
            <RadzenButton Icon="delete"
                          ButtonStyle="ButtonStyle.Danger"
                          Size="ButtonSize.Small"
                          Click="@(e => OnDelete.InvokeAsync(Item))"
                          Click:stopPropagation="true"
                          Disabled="@(!IsConnected || Item.IsPendingSync)"
                          title="Delete"
                          aria-label="@($"Delete {Item.Title}")" />
        </RadzenStack>
    </div>
</div>

@code {
    [Parameter, EditorRequired]
    public WorkItemViewModel Item { get; set; } = null!;

    [Parameter]
    public int IndentLevel { get; set; }

    [Parameter]
    public bool IsConnected { get; set; } = true;

    [Parameter]
    public EventCallback<WorkItemViewModel> OnEdit { get; set; }

    [Parameter]
    public EventCallback<WorkItemViewModel> OnDelete { get; set; }

    [Parameter]
    public EventCallback<WorkItemViewModel> OnSelect { get; set; }

    private string RowCssClass
    {
        get
        {
            var classes = new List<string>();
            if (Item.IsCompleted) classes.Add("status-done");
            if (Item.IsPendingSync) classes.Add("pending-sync");
            return string.Join(" ", classes);
        }
    }

    private string AriaLabel
    {
        get
        {
            var parts = new List<string>
            {
                Item.ItemType.ToString(),
                Item.Title,
                $"Status: {Item.StatusDisplayName}",
                $"Priority: {Item.PriorityDisplayName}"
            };
            if (Item.StoryPoints.HasValue)
            {
                parts.Add($"{Item.StoryPoints} points");
            }
            if (Item.IsPendingSync)
            {
                parts.Add("(saving)");
            }
            return string.Join(", ", parts);
        }
    }

    private async Task HandleClick()
    {
        if (OnSelect.HasDelegate)
        {
            await OnSelect.InvokeAsync(Item);
        }
    }

    private async Task HandleKeyDown(KeyboardEventArgs e)
    {
        if (Item.IsPendingSync) return;

        switch (e.Key)
        {
            case "Enter":
            case " ":
                if (OnSelect.HasDelegate)
                {
                    await OnSelect.InvokeAsync(Item);
                }
                break;

            case "e" when e.CtrlKey && IsConnected:
                if (OnEdit.HasDelegate)
                {
                    await OnEdit.InvokeAsync(Item);
                }
                break;

            case "Delete" when IsConnected:
                if (OnDelete.HasDelegate)
                {
                    await OnDelete.InvokeAsync(Item);
                }
                break;
        }
    }
}
```

### 17. ProjectManagement.Components/WorkItems/KanbanCard.razor

```razor
@using ProjectManagement.Core.ViewModels
@using ProjectManagement.Core.Models

<div class="kanban-card @CardCssClass"
     role="listitem"
     tabindex="0"
     draggable="@IsDraggable.ToString().ToLowerInvariant()"
     aria-label="@AriaLabel"
     aria-grabbed="@_isDragging.ToString().ToLowerInvariant()"
     @onclick="HandleClick"
     @onkeydown="HandleKeyDown"
     @ondragstart="HandleDragStart"
     @ondragend="HandleDragEnd">

    <RadzenStack Gap="0.5rem">
        <RadzenStack Orientation="Orientation.Horizontal" AlignItems="AlignItems.Start" Gap="0.25rem">
            <WorkItemTypeIcon Type="@Item.ItemType" />
            <RadzenText TextStyle="TextStyle.Body2" class="kanban-card-title">
                @Item.Title
            </RadzenText>
        </RadzenStack>

        <RadzenStack Orientation="Orientation.Horizontal" Gap="0.25rem" AlignItems="AlignItems.Center" JustifyContent="JustifyContent.SpaceBetween">
            <RadzenStack Orientation="Orientation.Horizontal" Gap="0.25rem" AlignItems="AlignItems.Center">
                <PriorityBadge Priority="@Item.Priority" />
                @if (Item.StoryPoints.HasValue)
                {
                    <RadzenBadge BadgeStyle="BadgeStyle.Info" Text="@Item.StoryPoints.Value.ToString()" />
                }
            </RadzenStack>

            @if (!Item.IsPendingSync)
            {
                <RadzenButton Icon="edit"
                              ButtonStyle="ButtonStyle.Light"
                              Size="ButtonSize.ExtraSmall"
                              Click="@HandleEditClick"
                              Click:stopPropagation="true"
                              Disabled="@(!IsConnected)"
                              title="Edit"
                              aria-label="@($"Edit {Item.Title}")" />
            }
            else
            {
                <RadzenProgressBarCircular ShowValue="false"
                                           Mode="ProgressBarMode.Indeterminate"
                                           Size="ProgressBarCircularSize.ExtraSmall"
                                           title="Saving..." />
            }
        </RadzenStack>
    </RadzenStack>
</div>

@code {
    [Parameter, EditorRequired]
    public WorkItemViewModel Item { get; set; } = null!;

    [Parameter]
    public bool IsConnected { get; set; } = true;

    [Parameter]
    public EventCallback<WorkItemViewModel> OnClick { get; set; }

    [Parameter]
    public EventCallback<WorkItemViewModel> OnEdit { get; set; }

    [Parameter]
    public EventCallback<WorkItemViewModel> OnDragStart { get; set; }

    [Parameter]
    public EventCallback OnDragEnd { get; set; }

    private bool _isDragging;

    private bool IsDraggable => IsConnected && !Item.IsPendingSync;

    private string CardCssClass
    {
        get
        {
            var classes = new List<string>();
            if (Item.IsPendingSync) classes.Add("pending-sync");
            if (_isDragging) classes.Add("dragging");
            return string.Join(" ", classes);
        }
    }

    private string AriaLabel
    {
        get
        {
            var label = $"{Item.ItemType}: {Item.Title}, Priority: {Item.PriorityDisplayName}";
            if (Item.StoryPoints.HasValue)
            {
                label += $", {Item.StoryPoints} points";
            }
            if (Item.IsPendingSync)
            {
                label += " (saving)";
            }
            return label;
        }
    }

    private async Task HandleClick()
    {
        if (OnClick.HasDelegate)
        {
            await OnClick.InvokeAsync(Item);
        }
    }

    private async Task HandleEditClick(MouseEventArgs e)
    {
        if (OnEdit.HasDelegate && IsConnected && !Item.IsPendingSync)
        {
            await OnEdit.InvokeAsync(Item);
        }
    }

    private async Task HandleDragStart(DragEventArgs e)
    {
        if (!IsDraggable) return;

        _isDragging = true;
        if (OnDragStart.HasDelegate)
        {
            await OnDragStart.InvokeAsync(Item);
        }
    }

    private async Task HandleDragEnd(DragEventArgs e)
    {
        _isDragging = false;
        if (OnDragEnd.HasDelegate)
        {
            await OnDragEnd.InvokeAsync();
        }
    }

    private async Task HandleKeyDown(KeyboardEventArgs e)
    {
        if (Item.IsPendingSync) return;

        switch (e.Key)
        {
            case "Enter":
                if (OnClick.HasDelegate)
                {
                    await OnClick.InvokeAsync(Item);
                }
                break;

            case " " when IsDraggable && !_isDragging:
                _isDragging = true;
                if (OnDragStart.HasDelegate)
                {
                    await OnDragStart.InvokeAsync(Item);
                }
                break;

            case "Escape" when _isDragging:
                _isDragging = false;
                if (OnDragEnd.HasDelegate)
                {
                    await OnDragEnd.InvokeAsync();
                }
                break;

            case "e" when e.CtrlKey && IsConnected:
                if (OnEdit.HasDelegate)
                {
                    await OnEdit.InvokeAsync(Item);
                }
                break;
        }
    }
}
```

### 18. ProjectManagement.Components/WorkItems/VersionConflictDialog.razor

```razor
@inject DialogService DialogService

<RadzenStack Gap="1rem" class="p-2">
    <RadzenStack Orientation="Orientation.Horizontal" AlignItems="AlignItems.Center" Gap="0.5rem">
        <RadzenIcon Icon="warning" Style="font-size: 2rem; color: var(--rz-warning);" />
        <RadzenText TextStyle="TextStyle.H6" class="m-0">Conflict Detected</RadzenText>
    </RadzenStack>

    <RadzenText>
        Someone else has edited "@ItemTitle" since you started editing.
    </RadzenText>

    <RadzenAlert AlertStyle="AlertStyle.Info" Shade="Shade.Light" Size="AlertSize.Small">
        Choose how to resolve this conflict:
    </RadzenAlert>

    <RadzenStack Gap="0.5rem">
        <RadzenButton Text="Reload (discard my changes)"
                      ButtonStyle="ButtonStyle.Secondary"
                      Style="width: 100%;"
                      Click="@(() => DialogService.Close(ConflictResolution.Reload))"
                      aria-label="Discard changes and reload" />
        <RadzenButton Text="Overwrite (keep my changes)"
                      ButtonStyle="ButtonStyle.Warning"
                      Style="width: 100%;"
                      Click="@(() => DialogService.Close(ConflictResolution.Overwrite))"
                      aria-label="Overwrite with my changes" />
        <RadzenButton Text="Cancel"
                      ButtonStyle="ButtonStyle.Light"
                      Style="width: 100%;"
                      Click="@(() => DialogService.Close(ConflictResolution.Cancel))"
                      aria-label="Cancel" />
    </RadzenStack>
</RadzenStack>

@code {
    [Parameter, EditorRequired]
    public string ItemTitle { get; set; } = "";

    public enum ConflictResolution
    {
        Cancel,
        Reload,
        Overwrite
    }
}
```

### 19. ProjectManagement.Components/WorkItems/WorkItemDialog.razor

This is a large component. Key corrections from original plan:
- Use `@bind-Value` instead of `Change` events for proper two-way binding
- Use `ValueChanged` callbacks where needed for dirty tracking
- Properly handle nullable types with Radzen components

```razor
@using ProjectManagement.Core.Models
@using ProjectManagement.Core.ViewModels
@using ProjectManagement.Core.Exceptions
@using ProjectManagement.Services.State
@inject AppState AppState
@inject ViewModelFactory ViewModelFactory
@inject DialogService DialogService
@inject NotificationService NotificationService
@implements IDisposable

<div role="dialog" aria-labelledby="work-item-dialog-title" aria-describedby="work-item-dialog-desc">
    <RadzenStack Gap="1rem">
        <span id="work-item-dialog-desc" class="visually-hidden">
            @(_isEdit ? "Edit work item details" : "Create a new work item")
        </span>

        @* Type Selection *@
        <RadzenRow>
            <RadzenColumn Size="12">
                <RadzenFormField Text="Type" Style="width: 100%;">
                    <RadzenDropDown @bind-Value="_itemType"
                                    Data="@TypeOptions"
                                    TextProperty="Text"
                                    ValueProperty="Value"
                                    Disabled="@_isEdit"
                                    Style="width: 100%;"
                                    aria-label="Work item type" />
                </RadzenFormField>
            </RadzenColumn>
        </RadzenRow>

        @* Title *@
        <RadzenRow>
            <RadzenColumn Size="12">
                <RadzenFormField Text="Title" Style="width: 100%;">
                    <RadzenTextBox @bind-Value="_title"
                                   Style="width: 100%;"
                                   MaxLength="200"
                                   Placeholder="Enter title..."
                                   Change="@HandleTitleChange"
                                   aria-label="Title"
                                   aria-describedby="title-error title-count"
                                   aria-invalid="@_errors.ContainsKey("Title").ToString().ToLowerInvariant()" />
                </RadzenFormField>
                <RadzenStack Orientation="Orientation.Horizontal" JustifyContent="JustifyContent.SpaceBetween">
                    @if (_errors.TryGetValue("Title", out var titleError))
                    {
                        <RadzenText id="title-error" TextStyle="TextStyle.Caption" Style="color: var(--rz-danger);">
                            @titleError
                        </RadzenText>
                    }
                    else
                    {
                        <span></span>
                    }
                    <RadzenText id="title-count" TextStyle="TextStyle.Caption" class="text-muted">
                        @(_title?.Length ?? 0)/200
                    </RadzenText>
                </RadzenStack>
            </RadzenColumn>
        </RadzenRow>

        @* Description *@
        <RadzenRow>
            <RadzenColumn Size="12">
                <RadzenFormField Text="Description" Style="width: 100%;">
                    <RadzenTextArea @bind-Value="_description"
                                    Style="width: 100%; min-height: 100px;"
                                    MaxLength="5000"
                                    Placeholder="Enter description..."
                                    Change="@HandleDescriptionChange"
                                    aria-label="Description"
                                    aria-describedby="desc-count" />
                </RadzenFormField>
                <div class="text-end">
                    <RadzenText id="desc-count" TextStyle="TextStyle.Caption" class="text-muted">
                        @(_description?.Length ?? 0)/5000
                    </RadzenText>
                </div>
            </RadzenColumn>
        </RadzenRow>

        @* Status and Priority *@
        <RadzenRow Gap="1rem">
            <RadzenColumn Size="6">
                <RadzenFormField Text="Status" Style="width: 100%;">
                    <RadzenDropDown @bind-Value="_status"
                                    Data="@StatusOptions"
                                    TextProperty="Text"
                                    ValueProperty="Value"
                                    Style="width: 100%;"
                                    Change="@(_ => MarkDirty())"
                                    aria-label="Status" />
                </RadzenFormField>
            </RadzenColumn>
            <RadzenColumn Size="6">
                <RadzenFormField Text="Priority" Style="width: 100%;">
                    <RadzenDropDown @bind-Value="_priority"
                                    Data="@PriorityOptions"
                                    TextProperty="Text"
                                    ValueProperty="Value"
                                    Style="width: 100%;"
                                    Change="@(_ => MarkDirty())"
                                    aria-label="Priority" />
                </RadzenFormField>
            </RadzenColumn>
        </RadzenRow>

        @* Story Points and Sprint *@
        <RadzenRow Gap="1rem">
            <RadzenColumn Size="6">
                <RadzenFormField Text="Story Points" Style="width: 100%;">
                    <RadzenNumeric @bind-Value="_storyPoints"
                                   TValue="int?"
                                   Min="0"
                                   Max="100"
                                   Style="width: 100%;"
                                   Placeholder="Optional"
                                   Change="@(_ => MarkDirty())"
                                   aria-label="Story points" />
                </RadzenFormField>
            </RadzenColumn>
            <RadzenColumn Size="6">
                <RadzenFormField Text="Sprint" Style="width: 100%;">
                    <RadzenDropDown @bind-Value="_sprintId"
                                    TValue="Guid?"
                                    Data="@_sprints"
                                    TextProperty="Name"
                                    ValueProperty="Id"
                                    AllowClear="true"
                                    Placeholder="No sprint"
                                    Style="width: 100%;"
                                    Change="@(_ => MarkDirty())"
                                    aria-label="Sprint" />
                </RadzenFormField>
            </RadzenColumn>
        </RadzenRow>

        @* Actions *@
        <RadzenRow>
            <RadzenColumn Size="12" class="text-end">
                <RadzenStack Orientation="Orientation.Horizontal" Gap="0.5rem" JustifyContent="JustifyContent.End">
                    <RadzenButton Text="Cancel"
                                  ButtonStyle="ButtonStyle.Light"
                                  Click="@HandleCancel"
                                  Disabled="@_saving" />
                    <LoadingButton Text="@(_isEdit ? "Save" : "Create")"
                                   LoadingText="@(_isEdit ? "Saving..." : "Creating...")"
                                   IsBusy="@_saving"
                                   ConnectionState="@(_isConnected ? ConnectionState.Connected : ConnectionState.Disconnected)"
                                   OnClick="@HandleSubmit" />
                </RadzenStack>
            </RadzenColumn>
        </RadzenRow>
    </RadzenStack>
</div>

@code {
    [Parameter]
    public WorkItemViewModel? WorkItem { get; set; }

    [Parameter]
    public Guid ProjectId { get; set; }

    [Parameter]
    public Guid? ParentId { get; set; }

    [Parameter]
    public WorkItemType DefaultItemType { get; set; } = WorkItemType.Story;

    // Form state
    private bool _isEdit => WorkItem is not null;
    private bool _saving;
    private bool _isDirty;
    private bool _isConnected = true;
    private Dictionary<string, string> _errors = new();

    // Form fields
    private WorkItemType _itemType;
    private string _title = "";
    private string? _description;
    private string _status = "backlog";
    private string _priority = "medium";
    private int? _storyPoints;
    private Guid? _sprintId;

    // Original values for dirty tracking
    private string _originalTitle = "";
    private string? _originalDescription;
    private string _originalStatus = "backlog";
    private string _originalPriority = "medium";
    private int? _originalStoryPoints;
    private Guid? _originalSprintId;

    // Sprint options
    private IReadOnlyList<Sprint> _sprints = Array.Empty<Sprint>();

    // Static dropdown options
    private static readonly List<object> TypeOptions = new()
    {
        new { Text = "Epic", Value = WorkItemType.Epic },
        new { Text = "Story", Value = WorkItemType.Story },
        new { Text = "Task", Value = WorkItemType.Task }
    };

    private static readonly List<object> StatusOptions = new()
    {
        new { Text = "Backlog", Value = "backlog" },
        new { Text = "To Do", Value = "todo" },
        new { Text = "In Progress", Value = "in_progress" },
        new { Text = "Review", Value = "review" },
        new { Text = "Done", Value = "done" }
    };

    private static readonly List<object> PriorityOptions = new()
    {
        new { Text = "Critical", Value = "critical" },
        new { Text = "High", Value = "high" },
        new { Text = "Medium", Value = "medium" },
        new { Text = "Low", Value = "low" }
    };

    protected override void OnInitialized()
    {
        _isConnected = AppState.ConnectionState == ConnectionState.Connected;
        AppState.OnConnectionStateChanged += HandleConnectionChanged;

        if (_isEdit && WorkItem is not null)
        {
            // Populate from existing work item
            _itemType = WorkItem.ItemType;
            _title = _originalTitle = WorkItem.Title;
            _description = _originalDescription = WorkItem.Description;
            _status = _originalStatus = WorkItem.Status;
            _priority = _originalPriority = WorkItem.Priority;
            _storyPoints = _originalStoryPoints = WorkItem.StoryPoints;
            _sprintId = _originalSprintId = WorkItem.SprintId;
            ProjectId = WorkItem.ProjectId;
        }
        else
        {
            // Defaults for new work item
            _itemType = DefaultItemType;
        }

        // Load sprints for the project
        _sprints = AppState.Sprints.GetByProject(ProjectId);
    }

    private void HandleConnectionChanged(ConnectionState state)
    {
        _isConnected = state == ConnectionState.Connected;
        InvokeAsync(StateHasChanged);
    }

    private void HandleTitleChange(string value)
    {
        _title = value;
        MarkDirty();
    }

    private void HandleDescriptionChange(string value)
    {
        _description = value;
        MarkDirty();
    }

    private void MarkDirty()
    {
        _isDirty = _title != _originalTitle ||
                   _description != _originalDescription ||
                   _status != _originalStatus ||
                   _priority != _originalPriority ||
                   _storyPoints != _originalStoryPoints ||
                   _sprintId != _originalSprintId;
    }

    private bool Validate()
    {
        _errors.Clear();

        var trimmedTitle = _title?.Trim() ?? "";
        if (string.IsNullOrWhiteSpace(trimmedTitle))
        {
            _errors["Title"] = "Title is required";
        }
        else if (trimmedTitle.Length > 200)
        {
            _errors["Title"] = "Title must be 200 characters or less";
        }

        return _errors.Count == 0;
    }

    private async Task HandleSubmit()
    {
        _title = _title?.Trim() ?? "";

        if (!Validate())
        {
            StateHasChanged();
            return;
        }

        _saving = true;
        StateHasChanged();

        try
        {
            if (_isEdit)
            {
                await UpdateWorkItemAsync();
            }
            else
            {
                await CreateWorkItemAsync();
            }

            DialogService.Close(true);
        }
        catch (VersionConflictException)
        {
            await HandleVersionConflictAsync();
        }
        catch (Exception ex)
        {
            NotificationService.Notify(NotificationSeverity.Error, "Error", ex.Message);
        }
        finally
        {
            _saving = false;
            StateHasChanged();
        }
    }

    private async Task CreateWorkItemAsync()
    {
        var request = new CreateWorkItemRequest
        {
            ProjectId = ProjectId,
            ItemType = _itemType,
            Title = _title,
            Description = _description,
            ParentId = ParentId,
            Status = _status,
            StoryPoints = _storyPoints,
            SprintId = _sprintId
        };

        await AppState.WorkItems.CreateAsync(request);
        NotificationService.Notify(NotificationSeverity.Success, "Created", "Work item created successfully");
    }

    private async Task UpdateWorkItemAsync()
    {
        var request = new UpdateWorkItemRequest
        {
            WorkItemId = WorkItem!.Id,
            ExpectedVersion = WorkItem.Version,
            Title = _title,
            Description = _description,
            Status = _status,
            Priority = _priority,
            StoryPoints = _storyPoints,
            SprintId = _sprintId
        };

        await AppState.WorkItems.UpdateAsync(request);
        NotificationService.Notify(NotificationSeverity.Success, "Saved", "Work item updated successfully");
    }

    private async Task HandleVersionConflictAsync()
    {
        var result = await DialogService.OpenAsync<VersionConflictDialog>(
            "Version Conflict",
            new Dictionary<string, object> { { "ItemTitle", WorkItem!.Title } },
            new DialogOptions { Width = "400px", CloseDialogOnOverlayClick = false });

        if (result is VersionConflictDialog.ConflictResolution resolution)
        {
            switch (resolution)
            {
                case VersionConflictDialog.ConflictResolution.Reload:
                    var reloaded = AppState.WorkItems.GetById(WorkItem.Id);
                    if (reloaded is not null)
                    {
                        // Re-initialize with fresh data
                        WorkItem = ViewModelFactory.Create(reloaded);
                        OnInitialized();
                        _isDirty = false;
                        StateHasChanged();
                    }
                    break;

                case VersionConflictDialog.ConflictResolution.Overwrite:
                    var current = AppState.WorkItems.GetById(WorkItem.Id);
                    var request = new UpdateWorkItemRequest
                    {
                        WorkItemId = WorkItem.Id,
                        ExpectedVersion = current?.Version ?? WorkItem.Version + 1,
                        Title = _title,
                        Description = _description,
                        Status = _status,
                        Priority = _priority,
                        StoryPoints = _storyPoints,
                        SprintId = _sprintId
                    };
                    await AppState.WorkItems.UpdateAsync(request);
                    NotificationService.Notify(NotificationSeverity.Success, "Saved", "Work item updated successfully");
                    DialogService.Close(true);
                    break;

                case VersionConflictDialog.ConflictResolution.Cancel:
                default:
                    // Do nothing, stay in dialog
                    break;
            }
        }
    }

    private async Task HandleCancel()
    {
        if (_isDirty)
        {
            var discard = await DialogService.Confirm(
                "You have unsaved changes. Discard them?",
                "Unsaved Changes",
                new ConfirmOptions
                {
                    OkButtonText = "Discard",
                    CancelButtonText = "Keep Editing"
                });

            if (discard != true)
            {
                return;
            }
        }

        DialogService.Close(false);
    }

    public void Dispose()
    {
        AppState.OnConnectionStateChanged -= HandleConnectionChanged;
    }
}
```

---

## Continued in Part 2...

Due to the length of this document, the remaining files (20-34) will be documented in a continuation. The key patterns established above should be followed consistently:

1. **Use ViewModelFactory** to create ViewModels from store data
2. **Bind with `@bind-Value`** for two-way binding, use `Change` callback with lambda for side effects: `Change="@(_ => MethodName())"`
3. **Always dispose event subscriptions** in `Dispose()` method
4. **Use `InvokeAsync(StateHasChanged)`** when updating from event handlers
5. **Check `IsPendingSync`** to disable actions and show loading indicators
6. **Use `ConnectionState`** to disable network-dependent actions when offline

---

## Required Store Updates (Do First)

Before implementing any Session 30 components, update the stores:

### IWorkItemStore.cs - Add method:
```csharp
bool IsPending(Guid id);
```

### ISprintStore.cs - Add method:
```csharp
bool IsPending(Guid id);
```

### WorkItemStore.cs - Implement:
```csharp
public bool IsPending(Guid id) => _pendingUpdates.ContainsKey(id);
```

### SprintStore.cs - Add field and implement:
```csharp
private readonly ConcurrentDictionary<Guid, bool> _pendingUpdates = new();

public bool IsPending(Guid id) => _pendingUpdates.ContainsKey(id);
```

### Program.cs - Register factory:
```csharp
builder.Services.AddScoped<ViewModelFactory>();
```

---

## Files 20-24: Kanban CSS, List, Column, Board Components

### 20. ProjectManagement.Components/wwwroot/css/kanban.css

```css
/* ===== Kanban Board ===== */
.kanban-board {
    min-height: 500px;
}

.kanban-columns {
    display: flex;
    gap: 1rem;
    overflow-x: auto;
    padding-bottom: 1rem;
    min-height: 400px;
}

.kanban-column {
    flex: 0 0 280px;
    background: var(--rz-base-200);
    border-radius: 8px;
    display: flex;
    flex-direction: column;
    max-height: calc(100vh - 250px);
    transition: box-shadow 0.2s ease, background-color 0.2s ease;
}

.kanban-column.drag-target {
    box-shadow: 0 0 0 2px var(--rz-primary);
    background: var(--rz-primary-lighter);
}

.kanban-column-header {
    padding: 0.75rem 1rem;
    border-bottom: 1px solid var(--rz-border-color);
    display: flex;
    justify-content: space-between;
    align-items: center;
    flex-shrink: 0;
}

.kanban-column-body {
    flex: 1;
    overflow-y: auto;
    padding: 0.5rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
}

.kanban-empty-column {
    padding: 2rem 1rem;
    text-align: center;
    color: var(--rz-text-tertiary-color);
}

/* ===== Kanban Card ===== */
.kanban-card {
    background: var(--rz-surface);
    border: 1px solid var(--rz-border-color);
    border-radius: 6px;
    padding: 0.75rem;
    cursor: grab;
    transition: box-shadow 0.15s ease, transform 0.15s ease;
    user-select: none;
}

.kanban-card:hover {
    box-shadow: var(--rz-shadow-2);
}

.kanban-card:focus-visible {
    outline: 2px solid var(--rz-primary);
    outline-offset: 2px;
}

.kanban-card:active,
.kanban-card[aria-grabbed="true"],
.kanban-card.dragging {
    cursor: grabbing;
    transform: rotate(2deg);
    box-shadow: var(--rz-shadow-3);
    opacity: 0.9;
}

.kanban-card.pending-sync {
    opacity: 0.7;
    pointer-events: none;
    cursor: default;
}

.kanban-card-title {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    word-break: break-word;
}

/* ===== Responsive ===== */
@media (max-width: 768px) {
    .kanban-column {
        flex: 0 0 260px;
    }
}

/* ===== Reduced motion ===== */
@media (prefers-reduced-motion: reduce) {
    .kanban-card {
        transition: none;
    }

    .kanban-card:active,
    .kanban-card.dragging {
        transform: none;
    }
}
```

### 21. ProjectManagement.Components/WorkItems/WorkItemList.razor

```razor
@using ProjectManagement.Core.Models
@using ProjectManagement.Core.ViewModels
@using ProjectManagement.Services.State
@using Microsoft.AspNetCore.Components.Web.Virtualization
@inject AppState AppState
@inject ViewModelFactory ViewModelFactory
@inject DialogService DialogService
@inject NotificationService NotificationService
@implements IDisposable

<RadzenStack Gap="0.5rem">
    @* Filters *@
    <RadzenRow AlignItems="AlignItems.Center" Gap="0.5rem">
        <RadzenColumn Size="12" SizeMD="6">
            <DebouncedTextBox @bind-Value="_searchText"
                              Placeholder="Search work items..."
                              Style="width: 100%;"
                              DebounceMs="300"
                              ValueChanged="@HandleSearchChanged" />
        </RadzenColumn>
        <RadzenColumn Size="12" SizeMD="6">
            <RadzenStack Orientation="Orientation.Horizontal" Gap="0.5rem" JustifyContent="JustifyContent.End">
                <RadzenDropDown @bind-Value="_typeFilter"
                                TValue="WorkItemType?"
                                Data="@TypeOptions"
                                TextProperty="Text"
                                ValueProperty="Value"
                                Placeholder="All Types"
                                AllowClear="true"
                                Change="@(_ => ApplyFilters())"
                                aria-label="Filter by type" />
                <RadzenDropDown @bind-Value="_statusFilter"
                                TValue="string"
                                Data="@StatusOptions"
                                TextProperty="Text"
                                ValueProperty="Value"
                                Placeholder="All Statuses"
                                AllowClear="true"
                                Change="@(_ => ApplyFilters())"
                                aria-label="Filter by status" />
            </RadzenStack>
        </RadzenColumn>
    </RadzenRow>

    @* Screen reader announcement *@
    <div class="visually-hidden" role="status" aria-live="polite" aria-atomic="true">
        @_filteredItems.Count work items found
    </div>

    @* Content *@
    @if (_filteredItems.Count == 0)
    {
        <EmptyState Icon="@(_allItems.Count == 0 ? "assignment" : "search_off")"
                    Title="@(_allItems.Count == 0 ? "No work items" : "No matches")"
                    Description="@(_allItems.Count == 0 ? "Create your first work item to get started." : "Try adjusting your filters.")"
                    ActionText="@(_allItems.Count == 0 ? "Create Work Item" : "")"
                    OnAction="@(_allItems.Count == 0 ? ShowCreateDialog : default)" />
    }
    else
    {
        <div class="work-item-list" role="table" aria-label="Work items">
            @* Header *@
            <div class="work-item-header" role="row">
                <div class="work-item-cell type-cell" role="columnheader">Type</div>
                <div class="work-item-cell title-cell" role="columnheader">Title</div>
                <div class="work-item-cell status-cell" role="columnheader">Status</div>
                <div class="work-item-cell priority-cell" role="columnheader">Priority</div>
                <div class="work-item-cell points-cell" role="columnheader">Points</div>
                <div class="work-item-cell actions-cell" role="columnheader">
                    <span class="visually-hidden">Actions</span>
                </div>
            </div>

            @* Rows with virtualization *@
            <Virtualize Items="@_filteredItems" Context="item" ItemSize="52">
                <ItemContent>
                    <WorkItemRow Item="@item"
                                 IndentLevel="@GetIndentLevel(item)"
                                 IsConnected="@_isConnected"
                                 OnEdit="@HandleEdit"
                                 OnDelete="@HandleDelete"
                                 OnSelect="@HandleSelect" />
                </ItemContent>
                <Placeholder>
                    <div class="work-item-row">
                        <RadzenSkeleton Width="100%" Height="48px" />
                    </div>
                </Placeholder>
            </Virtualize>
        </div>
    }
</RadzenStack>

@code {
    [Parameter, EditorRequired]
    public Guid ProjectId { get; set; }

    [Parameter]
    public EventCallback<WorkItemViewModel> OnWorkItemSelected { get; set; }

    private List<WorkItemViewModel> _allItems = new();
    private List<WorkItemViewModel> _filteredItems = new();
    private bool _isConnected = true;
    private string _searchText = "";
    private WorkItemType? _typeFilter;
    private string? _statusFilter;

    private static readonly List<object> TypeOptions = new()
    {
        new { Text = "Epic", Value = WorkItemType.Epic },
        new { Text = "Story", Value = WorkItemType.Story },
        new { Text = "Task", Value = WorkItemType.Task }
    };

    private static readonly List<object> StatusOptions = new()
    {
        new { Text = "Backlog", Value = "backlog" },
        new { Text = "To Do", Value = "todo" },
        new { Text = "In Progress", Value = "in_progress" },
        new { Text = "Review", Value = "review" },
        new { Text = "Done", Value = "done" }
    };

    protected override void OnInitialized()
    {
        _isConnected = AppState.ConnectionState == ConnectionState.Connected;
        AppState.OnStateChanged += HandleStateChanged;
        AppState.OnConnectionStateChanged += HandleConnectionChanged;
        RefreshData();
    }

    protected override void OnParametersSet()
    {
        RefreshData();
    }

    private void RefreshData()
    {
        var items = AppState.WorkItems.GetByProject(ProjectId)
            .Where(w => w.ItemType != WorkItemType.Project);
        _allItems = ViewModelFactory.CreateMany(items).ToList();
        ApplyFilters();
    }

    private void HandleSearchChanged(string value)
    {
        _searchText = value;
        ApplyFilters();
    }

    private void ApplyFilters()
    {
        var query = _allItems.AsEnumerable();

        if (!string.IsNullOrWhiteSpace(_searchText))
        {
            var search = _searchText.Trim();
            query = query.Where(w =>
                w.Title.Contains(search, StringComparison.OrdinalIgnoreCase) ||
                (w.Description?.Contains(search, StringComparison.OrdinalIgnoreCase) ?? false));
        }

        if (_typeFilter.HasValue)
        {
            query = query.Where(w => w.ItemType == _typeFilter.Value);
        }

        if (!string.IsNullOrWhiteSpace(_statusFilter))
        {
            query = query.Where(w => w.Status == _statusFilter);
        }

        _filteredItems = query.ToList();
        StateHasChanged();
    }

    private int GetIndentLevel(WorkItemViewModel item)
    {
        var level = 0;
        var currentParentId = item.ParentId;
        const int maxDepth = 5;

        while (currentParentId.HasValue && level < maxDepth)
        {
            var parent = AppState.WorkItems.GetById(currentParentId.Value);
            if (parent is null) break;
            currentParentId = parent.ParentId;
            level++;
        }

        return level;
    }

    private async Task HandleEdit(WorkItemViewModel item)
    {
        await DialogService.OpenAsync<WorkItemDialog>(
            "Edit Work Item",
            new Dictionary<string, object>
            {
                { "WorkItem", item },
                { "ProjectId", item.ProjectId }
            },
            new DialogOptions { Width = "600px" });
    }

    private async Task HandleDelete(WorkItemViewModel item)
    {
        var confirmed = await DialogService.Confirm(
            $"Are you sure you want to delete '{item.Title}'?",
            "Delete Work Item",
            new ConfirmOptions
            {
                OkButtonText = "Delete",
                CancelButtonText = "Cancel"
            });

        if (confirmed == true)
        {
            try
            {
                await AppState.WorkItems.DeleteAsync(item.Id);
                NotificationService.Notify(NotificationSeverity.Success, "Deleted", "Work item deleted");
            }
            catch (Exception ex)
            {
                NotificationService.Notify(NotificationSeverity.Error, "Error", ex.Message);
            }
        }
    }

    private async Task HandleSelect(WorkItemViewModel item)
    {
        if (OnWorkItemSelected.HasDelegate)
        {
            await OnWorkItemSelected.InvokeAsync(item);
        }
    }

    private async Task ShowCreateDialog()
    {
        await DialogService.OpenAsync<WorkItemDialog>(
            "Create Work Item",
            new Dictionary<string, object> { { "ProjectId", ProjectId } },
            new DialogOptions { Width = "600px" });
    }

    private void HandleStateChanged()
    {
        RefreshData();
        InvokeAsync(StateHasChanged);
    }

    private void HandleConnectionChanged(ConnectionState state)
    {
        _isConnected = state == ConnectionState.Connected;
        InvokeAsync(StateHasChanged);
    }

    public void Dispose()
    {
        AppState.OnStateChanged -= HandleStateChanged;
        AppState.OnConnectionStateChanged -= HandleConnectionChanged;
    }
}
```

### 22. ProjectManagement.Components/WorkItems/KanbanColumn.razor

```razor
@using ProjectManagement.Core.ViewModels

<div class="kanban-column @(IsDragTarget ? "drag-target" : "")"
     role="listbox"
     aria-label="@Title column, @ItemCount items"
     @ondragover="HandleDragOver"
     @ondragover:preventDefault="true"
     @ondrop="HandleDrop">

    <div class="kanban-column-header">
        <RadzenText TextStyle="TextStyle.Subtitle1" class="m-0">@Title</RadzenText>
        <RadzenBadge BadgeStyle="BadgeStyle.Light" Text="@ItemCount.ToString()" />
    </div>

    <div class="kanban-column-body" role="list">
        @if (ItemCount == 0)
        {
            <div class="kanban-empty-column">
                <RadzenText TextStyle="TextStyle.Caption" class="text-muted">No items</RadzenText>
            </div>
        }
        else
        {
            @foreach (var item in Items)
            {
                <KanbanCard Item="@item"
                            IsConnected="@IsConnected"
                            OnClick="@OnCardClick"
                            OnEdit="@OnCardEdit"
                            OnDragStart="@OnDragStart"
                            OnDragEnd="@OnDragEnd" />
            }
        }
    </div>
</div>

@code {
    [Parameter, EditorRequired]
    public string Status { get; set; } = "";

    [Parameter, EditorRequired]
    public string Title { get; set; } = "";

    [Parameter]
    public IEnumerable<WorkItemViewModel> Items { get; set; } = Enumerable.Empty<WorkItemViewModel>();

    [Parameter]
    public bool IsConnected { get; set; } = true;

    [Parameter]
    public bool IsDragTarget { get; set; }

    [Parameter]
    public EventCallback<WorkItemViewModel> OnCardClick { get; set; }

    [Parameter]
    public EventCallback<WorkItemViewModel> OnCardEdit { get; set; }

    [Parameter]
    public EventCallback<WorkItemViewModel> OnDragStart { get; set; }

    [Parameter]
    public EventCallback OnDragEnd { get; set; }

    [Parameter]
    public EventCallback OnDragEnter { get; set; }

    [Parameter]
    public EventCallback<string> OnDrop { get; set; }

    private int ItemCount => Items.Count();

    private async Task HandleDragOver(DragEventArgs e)
    {
        if (OnDragEnter.HasDelegate)
        {
            await OnDragEnter.InvokeAsync();
        }
    }

    private async Task HandleDrop(DragEventArgs e)
    {
        if (OnDrop.HasDelegate)
        {
            await OnDrop.InvokeAsync(Status);
        }
    }
}
```

### 23. ProjectManagement.Components/WorkItems/KanbanBoard.razor

```razor
@using ProjectManagement.Core.Models
@using ProjectManagement.Core.ViewModels
@using ProjectManagement.Services.State
@inject AppState AppState
@inject ViewModelFactory ViewModelFactory
@inject DialogService DialogService
@inject NotificationService NotificationService
@implements IDisposable

<div class="kanban-board"
     role="application"
     aria-label="Kanban board"
     aria-describedby="kanban-instructions"
     @onkeydown="HandleBoardKeyDown">

    @* Screen reader instructions *@
    <div id="kanban-instructions" class="visually-hidden">
        Use arrow keys to navigate between columns. Press Space to pick up or drop a card. Press Escape to cancel.
    </div>

    @* Live announcement region *@
    <div class="visually-hidden" role="status" aria-live="polite" aria-atomic="true">
        @_announcement
    </div>

    @* Filters *@
    <RadzenRow class="mb-3">
        <RadzenColumn Size="12">
            <RadzenStack Orientation="Orientation.Horizontal" Gap="0.5rem" AlignItems="AlignItems.Center">
                <RadzenDropDown @bind-Value="_typeFilter"
                                TValue="WorkItemType?"
                                Data="@TypeOptions"
                                TextProperty="Text"
                                ValueProperty="Value"
                                Placeholder="All Types"
                                AllowClear="true"
                                Change="@(_ => ApplyFilters())"
                                aria-label="Filter by type" />
                <RadzenCheckBox @bind-Value="_hideDone" TValue="bool" />
                <RadzenText TextStyle="TextStyle.Body2">Hide Done</RadzenText>
            </RadzenStack>
        </RadzenColumn>
    </RadzenRow>

    @* Columns *@
    <div class="kanban-columns" role="listbox" aria-orientation="horizontal">
        @foreach (var column in Columns)
        {
            <KanbanColumn Status="@column.Status"
                          Title="@column.Title"
                          Items="@GetColumnItems(column.Status)"
                          IsConnected="@_isConnected"
                          IsDragTarget="@(_draggedItem is not null && _dragTargetColumn == column.Status)"
                          OnCardClick="@HandleCardClick"
                          OnCardEdit="@HandleCardEdit"
                          OnDragStart="@HandleDragStart"
                          OnDragEnd="@HandleDragEnd"
                          OnDragEnter="@(() => HandleDragEnter(column.Status))"
                          OnDrop="@HandleDrop" />
        }
    </div>
</div>

@code {
    [Parameter, EditorRequired]
    public Guid ProjectId { get; set; }

    [Parameter]
    public EventCallback<WorkItemViewModel> OnWorkItemSelected { get; set; }

    // State
    private List<WorkItemViewModel> _allItems = new();
    private List<WorkItemViewModel> _filteredItems = new();
    private bool _isConnected = true;
    private WorkItemType? _typeFilter;
    private bool _hideDone;

    // Drag state
    private WorkItemViewModel? _draggedItem;
    private string? _dragTargetColumn;
    private string _announcement = "";

    // Column definitions
    private static readonly List<(string Status, string Title)> Columns = new()
    {
        ("backlog", "Backlog"),
        ("todo", "To Do"),
        ("in_progress", "In Progress"),
        ("review", "Review"),
        ("done", "Done")
    };

    private static readonly List<object> TypeOptions = new()
    {
        new { Text = "Epic", Value = WorkItemType.Epic },
        new { Text = "Story", Value = WorkItemType.Story },
        new { Text = "Task", Value = WorkItemType.Task }
    };

    protected override void OnInitialized()
    {
        _isConnected = AppState.ConnectionState == ConnectionState.Connected;
        AppState.OnStateChanged += HandleStateChanged;
        AppState.OnConnectionStateChanged += HandleConnectionChanged;
        RefreshData();
    }

    protected override void OnParametersSet()
    {
        RefreshData();
    }

    private void RefreshData()
    {
        var items = AppState.WorkItems.GetByProject(ProjectId)
            .Where(w => w.ItemType != WorkItemType.Project);
        _allItems = ViewModelFactory.CreateMany(items).ToList();
        ApplyFilters();
    }

    private void ApplyFilters()
    {
        var query = _allItems.AsEnumerable();

        if (_typeFilter.HasValue)
        {
            query = query.Where(w => w.ItemType == _typeFilter.Value);
        }

        if (_hideDone)
        {
            query = query.Where(w => w.Status != "done");
        }

        _filteredItems = query.ToList();
        StateHasChanged();
    }

    private IEnumerable<WorkItemViewModel> GetColumnItems(string status)
    {
        return _filteredItems
            .Where(w => w.Status == status)
            .OrderBy(w => w.Position);
    }

    private void HandleDragStart(WorkItemViewModel item)
    {
        if (!_isConnected || item.IsPendingSync) return;

        _draggedItem = item;
        _announcement = $"Picked up {item.Title}. Use arrow keys to move between columns.";
        StateHasChanged();
    }

    private void HandleDragEnter(string status)
    {
        if (_draggedItem is null) return;

        _dragTargetColumn = status;
        var columnTitle = Columns.First(c => c.Status == status).Title;
        _announcement = $"Over {columnTitle} column.";
        StateHasChanged();
    }

    private void HandleDragEnd()
    {
        _draggedItem = null;
        _dragTargetColumn = null;
        _announcement = "";
        StateHasChanged();
    }

    private async Task HandleDrop(string newStatus)
    {
        if (_draggedItem is null || !_isConnected)
        {
            HandleDragEnd();
            return;
        }

        var item = _draggedItem;
        var oldStatus = item.Status;

        if (oldStatus == newStatus)
        {
            HandleDragEnd();
            return;
        }

        var columnTitle = Columns.First(c => c.Status == newStatus).Title;
        _announcement = $"Dropped {item.Title} in {columnTitle}.";

        try
        {
            var request = new UpdateWorkItemRequest
            {
                WorkItemId = item.Id,
                ExpectedVersion = item.Version,
                Status = newStatus
            };

            await AppState.WorkItems.UpdateAsync(request);
            NotificationService.Notify(NotificationSeverity.Success, "Moved", $"Moved to {columnTitle}");
        }
        catch (Exception ex)
        {
            NotificationService.Notify(NotificationSeverity.Error, "Error", ex.Message);
            _announcement = $"Failed to move {item.Title}.";
        }
        finally
        {
            HandleDragEnd();
        }
    }

    private async Task HandleCardClick(WorkItemViewModel item)
    {
        if (OnWorkItemSelected.HasDelegate)
        {
            await OnWorkItemSelected.InvokeAsync(item);
        }
    }

    private async Task HandleCardEdit(WorkItemViewModel item)
    {
        await DialogService.OpenAsync<WorkItemDialog>(
            "Edit Work Item",
            new Dictionary<string, object>
            {
                { "WorkItem", item },
                { "ProjectId", item.ProjectId }
            },
            new DialogOptions { Width = "600px" });
    }

    private void HandleBoardKeyDown(KeyboardEventArgs e)
    {
        if (_draggedItem is null) return;

        var currentStatus = _dragTargetColumn ?? _draggedItem.Status;
        var currentIndex = Columns.FindIndex(c => c.Status == currentStatus);

        switch (e.Key)
        {
            case "ArrowLeft" when currentIndex > 0:
                HandleDragEnter(Columns[currentIndex - 1].Status);
                break;

            case "ArrowRight" when currentIndex < Columns.Count - 1:
                HandleDragEnter(Columns[currentIndex + 1].Status);
                break;

            case " " when _dragTargetColumn is not null:
                _ = HandleDrop(_dragTargetColumn);
                break;

            case "Escape":
                _announcement = "Drag cancelled.";
                HandleDragEnd();
                break;
        }
    }

    private void HandleStateChanged()
    {
        RefreshData();
        InvokeAsync(StateHasChanged);
    }

    private void HandleConnectionChanged(ConnectionState state)
    {
        _isConnected = state == ConnectionState.Connected;
        InvokeAsync(StateHasChanged);
    }

    public void Dispose()
    {
        AppState.OnStateChanged -= HandleStateChanged;
        AppState.OnConnectionStateChanged -= HandleConnectionChanged;
    }
}
```

### 24. ProjectManagement.Components/wwwroot/css/layout.css

```css
/* ===== CSS Custom Properties ===== */
:root {
    --nav-width: 250px;
    --nav-collapsed-width: 60px;
    --header-height: 50px;
    --content-padding: 1.5rem;
    --transition-speed: 0.2s;
}

/* ===== Page Header ===== */
.page-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 1.5rem;
    flex-wrap: wrap;
    gap: 1rem;
}

.page-title {
    margin: 0;
    font-size: 1.5rem;
    font-weight: 600;
}

.page-actions {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
}

/* ===== Breadcrumbs ===== */
.breadcrumbs {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 1rem;
    font-size: 0.875rem;
    color: var(--rz-text-secondary-color);
}

.breadcrumbs a {
    color: var(--rz-primary);
    text-decoration: none;
}

.breadcrumbs a:hover {
    text-decoration: underline;
}

.breadcrumbs .separator {
    color: var(--rz-text-tertiary-color);
}

/* ===== Content Cards ===== */
.content-card {
    background: var(--rz-surface);
    border: 1px solid var(--rz-border-color);
    border-radius: 8px;
    padding: 1.5rem;
}

.content-card-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 1rem;
    padding-bottom: 1rem;
    border-bottom: 1px solid var(--rz-border-color);
}

/* ===== Tabs ===== */
.view-tabs {
    margin-bottom: 1rem;
}

/* ===== Responsive ===== */
@media (max-width: 768px) {
    .page-header {
        flex-direction: column;
        align-items: flex-start;
    }

    .page-actions {
        width: 100%;
        justify-content: flex-start;
    }

    .breadcrumbs {
        flex-wrap: wrap;
    }
}
```

---

## Files 25-29: Layout Updates and Pages

### 25. ProjectManagement.Wasm/Layout/NavMenu.razor (update)

```razor
@using ProjectManagement.Core.Models
@using ProjectManagement.Services.State
@inject AppState AppState
@inject NavigationManager NavigationManager
@implements IDisposable

<nav class="nav-menu" aria-label="Main navigation">
    <div class="nav-header">
        <span class="nav-brand">Project Management</span>
    </div>

    <div class="nav-items">
        <NavLink class="nav-item" href="" Match="NavLinkMatch.All">
            <RadzenIcon Icon="home" />
            <span>Home</span>
        </NavLink>

        @if (_projects.Any())
        {
            <div class="nav-section-header">
                <RadzenText TextStyle="TextStyle.Overline" class="text-muted px-3 py-2">Projects</RadzenText>
            </div>

            @foreach (var project in _projects.Take(5))
            {
                <NavLink class="nav-item" href="@($"project/{project.Id}")">
                    <RadzenIcon Icon="folder" />
                    <span>@project.Title</span>
                </NavLink>
            }
        }
    </div>

    <div class="nav-footer">
        <ConnectionStatus />
    </div>
</nav>

@code {
    private List<WorkItem> _projects = new();

    protected override void OnInitialized()
    {
        AppState.OnStateChanged += HandleStateChanged;
        RefreshProjects();
    }

    private void RefreshProjects()
    {
        // Get all projects (WorkItemType.Project)
        _projects = AppState.WorkItems.GetByProject(Guid.Empty)
            .Where(w => w.ItemType == WorkItemType.Project && w.DeletedAt is null)
            .OrderByDescending(w => w.UpdatedAt)
            .Take(5)
            .ToList();
    }

    private void HandleStateChanged()
    {
        RefreshProjects();
        InvokeAsync(StateHasChanged);
    }

    public void Dispose()
    {
        AppState.OnStateChanged -= HandleStateChanged;
    }
}
```

### 26. ProjectManagement.Wasm/Layout/MainLayout.razor (update)

```razor
@inherits LayoutComponentBase
@using Radzen
@using Radzen.Blazor
@using ProjectManagement.Wasm.Shared
@using ProjectManagement.Components.Shared

<RadzenLayout>
    <RadzenHeader>
        <RadzenStack Orientation="Orientation.Horizontal"
                     AlignItems="AlignItems.Center"
                     JustifyContent="JustifyContent.SpaceBetween"
                     Gap="1rem"
                     class="px-4 w-100">
            <RadzenText TextStyle="TextStyle.H5" Text="Project Management" class="m-0" />
            <ConnectionStatus />
        </RadzenStack>
    </RadzenHeader>

    <RadzenSidebar>
        <NavMenu />
    </RadzenSidebar>

    <RadzenBody>
        <OfflineBanner />
        <div class="rz-p-4">
            <AppErrorBoundary>
                @Body
            </AppErrorBoundary>
        </div>
    </RadzenBody>
</RadzenLayout>

<RadzenDialog />
<RadzenNotification />
<RadzenContextMenu />
<RadzenTooltip />
```

### 27-29: Pages (Home, ProjectDetail, WorkItemDetail)

These pages follow the same patterns established above. Key points:

1. **Inject AppState and ViewModelFactory** - not individual stores
2. **Subscribe to OnStateChanged and OnConnectionStateChanged** in `OnInitialized()`
3. **Unsubscribe in Dispose()**
4. **Use InvokeAsync(StateHasChanged)** when updating from event handlers
5. **Check connection state** before enabling actions
6. **Use ViewModelFactory.CreateMany()** to convert store data to ViewModels

Due to document length, these are abbreviated. The full implementations follow the patterns in WorkItemList.razor and KanbanBoard.razor.

---

## Files 30-34: Tests

### Test Patterns to Follow

All tests should use:
- **xUnit** for test framework
- **Moq** for mocking
- **FluentAssertions** for assertions
- **bUnit** for component testing

### Example: ViewModelFactory Tests

```csharp
using FluentAssertions;
using Moq;
using ProjectManagement.Core.Interfaces;
using ProjectManagement.Core.Models;
using ProjectManagement.Core.ViewModels;
using Xunit;

namespace ProjectManagement.Components.Tests.ViewModels;

public class ViewModelFactoryTests
{
    private readonly Mock<IWorkItemStore> _workItemStore;
    private readonly Mock<ISprintStore> _sprintStore;
    private readonly ViewModelFactory _factory;

    public ViewModelFactoryTests()
    {
        _workItemStore = new Mock<IWorkItemStore>();
        _sprintStore = new Mock<ISprintStore>();
        _factory = new ViewModelFactory(_workItemStore.Object, _sprintStore.Object);
    }

    [Fact]
    public void Create_WorkItem_SetsIsPendingSyncFalse_WhenNotPending()
    {
        // Arrange
        var workItem = new WorkItem
        {
            Id = Guid.NewGuid(),
            Title = "Test Item",
            ItemType = WorkItemType.Story,
            ProjectId = Guid.NewGuid()
        };
        _workItemStore.Setup(s => s.IsPending(workItem.Id)).Returns(false);

        // Act
        var viewModel = _factory.Create(workItem);

        // Assert
        viewModel.IsPendingSync.Should().BeFalse();
        viewModel.Model.Should().BeSameAs(workItem);
        viewModel.Title.Should().Be("Test Item");
    }

    [Fact]
    public void Create_WorkItem_SetsIsPendingSyncTrue_WhenPending()
    {
        // Arrange
        var workItem = new WorkItem
        {
            Id = Guid.NewGuid(),
            Title = "Test Item",
            ItemType = WorkItemType.Story,
            ProjectId = Guid.NewGuid()
        };
        _workItemStore.Setup(s => s.IsPending(workItem.Id)).Returns(true);

        // Act
        var viewModel = _factory.Create(workItem);

        // Assert
        viewModel.IsPendingSync.Should().BeTrue();
    }

    [Fact]
    public void Create_ThrowsArgumentNullException_WhenItemIsNull()
    {
        // Act
        var act = () => _factory.Create((WorkItem)null!);

        // Assert
        act.Should().Throw<ArgumentNullException>();
    }

    [Fact]
    public void CreateMany_ReturnsViewModelsForAllItems()
    {
        // Arrange
        var items = new List<WorkItem>
        {
            new() { Id = Guid.NewGuid(), Title = "Item 1", ItemType = WorkItemType.Story, ProjectId = Guid.NewGuid() },
            new() { Id = Guid.NewGuid(), Title = "Item 2", ItemType = WorkItemType.Task, ProjectId = Guid.NewGuid() }
        };
        _workItemStore.Setup(s => s.IsPending(It.IsAny<Guid>())).Returns(false);

        // Act
        var viewModels = _factory.CreateMany(items);

        // Assert
        viewModels.Should().HaveCount(2);
        viewModels[0].Title.Should().Be("Item 1");
        viewModels[1].Title.Should().Be("Item 2");
    }
}
```

---

## Implementation Checklist

Before starting implementation, verify:

- [ ] Store interfaces updated with `IsPending(Guid id)` method
- [ ] Store implementations updated with `IsPending` implementation
- [ ] `ViewModelFactory` registered in DI as Scoped
- [ ] CSS files linked in `index.html` or `_Host.cshtml`
- [ ] `_Imports.razor` updated with new namespaces

During implementation:

- [ ] Each component compiles before moving to the next
- [ ] Event subscriptions paired with unsubscriptions in Dispose
- [ ] All Radzen `Change` events use lambda syntax for void callbacks
- [ ] Accessibility attributes present (aria-label, role, etc.)
- [ ] Loading states shown during async operations
- [ ] Error handling with user-friendly messages

After implementation:

- [ ] All tests pass
- [ ] Manual testing of drag-and-drop
- [ ] Manual testing of offline behavior
- [ ] Screen reader testing for accessibility

---

## Sources

- [.NET 10 Blazor Breaking Changes](https://www.funkysi1701.com/posts/2025/blazor-and-dotnet10/)
- [ASP.NET Core 10.0 Release Notes](https://learn.microsoft.com/en-us/aspnet/core/release-notes/aspnetcore-10.0?view=aspnetcore-10.0)
- [Radzen Blazor Changelog](https://blazor.radzen.com/changelog)
- [Radzen DataGrid Row Reorder](https://blazor.radzen.com/datagrid-rowreorder)
- [Radzen Forum - Drag and Drop](https://forum.radzen.com/t/solved-reorder-datagrid-drag-and-drop-row-radzen/17256)
- [Blazor State Management Best Practices](https://www.infragistics.com/blogs/blazor-state-management/)
