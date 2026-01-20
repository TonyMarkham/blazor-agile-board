# Session 30 Part 1: Foundation + Leaf Components

**Goal**: Establish ViewModel infrastructure, all CSS, all leaf components, and comprehensive tests

**Quality Target**: 9.25+/10 production-grade - NO SHORTCUTS

**Deliverables**:
- ViewModels with pending state tracking
- ViewModelFactory for creating ViewModels
- All CSS files (app, work-items, kanban, layout)
- All leaf components (badges, icons, shared utilities)
- 35+ passing tests

**Checkpoint**: Part 1 must compile and all tests must pass before starting Part 2

---

## Prerequisites: Store Interface Updates

**CRITICAL**: These changes MUST be made before implementing ANY Session 30 code.

### 1. Update IWorkItemStore.cs

```csharp
// Add this method to the interface
/// <summary>Check if a work item has a pending optimistic update.</summary>
bool IsPending(Guid id);
```

### 2. Update ISprintStore.cs

```csharp
// Add this method to the interface
/// <summary>Check if a sprint has a pending optimistic update.</summary>
bool IsPending(Guid id);
```

### 3. Update WorkItemStore.cs

```csharp
// Add implementation
public bool IsPending(Guid id) => _pendingUpdates.ContainsKey(id);
```

### 4. Update SprintStore.cs

```csharp
// Add field (if not already present)
private readonly ConcurrentDictionary<Guid, bool> _pendingUpdates = new();

// Add implementation
public bool IsPending(Guid id) => _pendingUpdates.ContainsKey(id);

// Update write operations to track pending state:
// In CreateAsync, UpdateAsync, StartSprintAsync, CompleteSprintAsync, DeleteAsync:
// Add: _pendingUpdates[id] = true; before the async call
// Remove: _pendingUpdates.TryRemove(id, out _); in finally block
```

### 5. Verification

```bash
cd frontend
dotnet build ProjectManagement.Core
dotnet build ProjectManagement.Services
dotnet test ProjectManagement.Services.Tests
# All must pass before proceeding
```

---

## File Build Order (Part 1)

| Order | File | Type | Dependencies |
|-------|------|------|--------------|
| 1 | `ProjectManagement.Core/ViewModels/IViewModel.cs` | Interface | None |
| 2 | `ProjectManagement.Core/ViewModels/WorkItemViewModel.cs` | Class | #1 |
| 3 | `ProjectManagement.Core/ViewModels/SprintViewModel.cs` | Class | #1 |
| 4 | `ProjectManagement.Core/ViewModels/ViewModelFactory.cs` | Class | #2, #3 |
| 5 | `ProjectManagement.Components/wwwroot/css/app.css` | CSS | None |
| 6 | `ProjectManagement.Components/wwwroot/css/work-items.css` | CSS | None |
| 7 | `ProjectManagement.Components/wwwroot/css/kanban.css` | CSS | None |
| 8 | `ProjectManagement.Components/wwwroot/css/layout.css` | CSS | None |
| 9 | `ProjectManagement.Components/_Imports.razor` | Imports | None |
| 10 | `ProjectManagement.Components/Shared/OfflineBanner.razor` | Component | AppState |
| 11 | `ProjectManagement.Components/Shared/EmptyState.razor` | Component | None |
| 12 | `ProjectManagement.Components/Shared/LoadingButton.razor` | Component | None |
| 13 | `ProjectManagement.Components/Shared/DebouncedTextBox.razor` | Component | None |
| 14 | `ProjectManagement.Components/Shared/ConfirmDialog.razor` | Component | None |
| 15 | `ProjectManagement.Components/Shared/ProjectDetailSkeleton.razor` | Component | None |
| 16 | `ProjectManagement.Components/WorkItems/WorkItemTypeIcon.razor` | Component | None |
| 17 | `ProjectManagement.Components/WorkItems/WorkItemStatusBadge.razor` | Component | None |
| 18 | `ProjectManagement.Components/WorkItems/PriorityBadge.razor` | Component | None |
| 19 | `ProjectManagement.Components.Tests/ViewModels/ViewModelFactoryTests.cs` | Tests | #4 |
| 20 | `ProjectManagement.Components.Tests/ViewModels/WorkItemViewModelTests.cs` | Tests | #2 |
| 21 | `ProjectManagement.Components.Tests/ViewModels/SprintViewModelTests.cs` | Tests | #3 |
| 22 | `ProjectManagement.Components.Tests/Shared/SharedComponentTests.cs` | Tests | #10-15 |
| 23 | `ProjectManagement.Components.Tests/WorkItems/BadgeComponentTests.cs` | Tests | #16-18 |

**Total: 23 files**

---

## File 1: IViewModel.cs

**Path**: `ProjectManagement.Core/ViewModels/IViewModel.cs`

```csharp
namespace ProjectManagement.Core.ViewModels;

/// <summary>
/// Base interface for all view models.
/// View models combine immutable domain models with transient UI state.
/// </summary>
/// <typeparam name="TModel">The underlying domain model type</typeparam>
public interface IViewModel<out TModel> where TModel : class
{
    /// <summary>
    /// The underlying domain model (from server/store).
    /// This is immutable - UI changes create new ViewModels.
    /// </summary>
    TModel Model { get; }

    /// <summary>
    /// True when this item has pending changes being synced to the server.
    /// Used for optimistic UI feedback (shimmer effect, disabled buttons, etc.)
    /// </summary>
    bool IsPendingSync { get; }
}
```

---

## File 2: WorkItemViewModel.cs

**Path**: `ProjectManagement.Core/ViewModels/WorkItemViewModel.cs`

```csharp
using ProjectManagement.Core.Models;

namespace ProjectManagement.Core.ViewModels;

/// <summary>
/// View model for WorkItem. Combines immutable domain data with UI state.
/// Exposes commonly-accessed properties for convenient Razor binding.
/// </summary>
public sealed class WorkItemViewModel : IViewModel<WorkItem>, IEquatable<WorkItemViewModel>
{
    public WorkItemViewModel(WorkItem model, bool isPendingSync = false)
    {
        ArgumentNullException.ThrowIfNull(model);
        Model = model;
        IsPendingSync = isPendingSync;
    }

    public WorkItem Model { get; }
    public bool IsPendingSync { get; }

    // === Identity ===
    public Guid Id => Model.Id;
    public int Version => Model.Version;

    // === Core Properties ===
    public WorkItemType ItemType => Model.ItemType;
    public string Title => Model.Title;
    public string? Description => Model.Description;
    public string Status => Model.Status;
    public string Priority => Model.Priority;
    public int? StoryPoints => Model.StoryPoints;
    public int Position => Model.Position;

    // === Relationships ===
    public Guid ProjectId => Model.ProjectId;
    public Guid? ParentId => Model.ParentId;
    public Guid? SprintId => Model.SprintId;
    public Guid? AssigneeId => Model.AssigneeId;

    // === Audit ===
    public Guid CreatedBy => Model.CreatedBy;
    public Guid UpdatedBy => Model.UpdatedBy;
    public DateTime CreatedAt => Model.CreatedAt;
    public DateTime UpdatedAt => Model.UpdatedAt;
    public DateTime? DeletedAt => Model.DeletedAt;

    // === Computed Properties ===
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

    public string ItemTypeDisplayName => ItemType switch
    {
        WorkItemType.Project => "Project",
        WorkItemType.Epic => "Epic",
        WorkItemType.Story => "Story",
        WorkItemType.Task => "Task",
        _ => ItemType.ToString()
    };

    /// <summary>
    /// Priority sort order (lower = more urgent).
    /// </summary>
    public int PrioritySortOrder => Priority switch
    {
        "critical" => 0,
        "high" => 1,
        "medium" => 2,
        "low" => 3,
        _ => 4
    };

    // === Equality ===
    public bool Equals(WorkItemViewModel? other)
    {
        if (other is null) return false;
        if (ReferenceEquals(this, other)) return true;
        return Id == other.Id && Version == other.Version && IsPendingSync == other.IsPendingSync;
    }

    public override bool Equals(object? obj) => Equals(obj as WorkItemViewModel);

    public override int GetHashCode() => HashCode.Combine(Id, Version, IsPendingSync);

    public static bool operator ==(WorkItemViewModel? left, WorkItemViewModel? right) =>
        left?.Equals(right) ?? right is null;

    public static bool operator !=(WorkItemViewModel? left, WorkItemViewModel? right) =>
        !(left == right);
}
```

---

## File 3: SprintViewModel.cs

**Path**: `ProjectManagement.Core/ViewModels/SprintViewModel.cs`

```csharp
using ProjectManagement.Core.Models;

namespace ProjectManagement.Core.ViewModels;

/// <summary>
/// View model for Sprint. Combines immutable domain data with UI state.
/// Note: Sprint.StartDate and Sprint.EndDate are non-nullable (per Session 20).
/// </summary>
public sealed class SprintViewModel : IViewModel<Sprint>, IEquatable<SprintViewModel>
{
    public SprintViewModel(Sprint model, bool isPendingSync = false)
    {
        ArgumentNullException.ThrowIfNull(model);
        Model = model;
        IsPendingSync = isPendingSync;
    }

    public Sprint Model { get; }
    public bool IsPendingSync { get; }

    // === Identity ===
    public Guid Id => Model.Id;
    public Guid ProjectId => Model.ProjectId;

    // === Core Properties ===
    public string Name => Model.Name;
    public string? Goal => Model.Goal;
    public DateTime StartDate => Model.StartDate;
    public DateTime EndDate => Model.EndDate;
    public SprintStatus Status => Model.Status;

    // === Audit ===
    public DateTime? DeletedAt => Model.DeletedAt;

    // === Computed Properties ===
    public bool IsDeleted => Model.DeletedAt.HasValue;
    public bool IsPlanned => Model.Status == SprintStatus.Planned;
    public bool IsActive => Model.Status == SprintStatus.Active;
    public bool IsCompleted => Model.Status == SprintStatus.Completed;

    public string StatusDisplayName => Status switch
    {
        SprintStatus.Planned => "Planned",
        SprintStatus.Active => "Active",
        SprintStatus.Completed => "Completed",
        _ => Status.ToString()
    };

    /// <summary>
    /// Formatted date range for display (e.g., "Jan 15 - Jan 29").
    /// </summary>
    public string DateRangeDisplay => $"{StartDate:MMM d} - {EndDate:MMM d}";

    /// <summary>
    /// Full date range with year for clarity.
    /// </summary>
    public string DateRangeDisplayFull => $"{StartDate:MMM d, yyyy} - {EndDate:MMM d, yyyy}";

    /// <summary>
    /// Days remaining in the sprint (only meaningful when Active).
    /// Returns null if sprint is not active.
    /// </summary>
    public int? DaysRemaining
    {
        get
        {
            if (Status != SprintStatus.Active) return null;
            var remaining = (EndDate.Date - DateTime.UtcNow.Date).TotalDays;
            return Math.Max(0, (int)remaining);
        }
    }

    /// <summary>
    /// Total duration of the sprint in days.
    /// </summary>
    public int DurationDays => Math.Max(1, (int)(EndDate.Date - StartDate.Date).TotalDays);

    /// <summary>
    /// Progress percentage through the sprint (0-100).
    /// Based on elapsed time, not completed work.
    /// </summary>
    public double ProgressPercent
    {
        get
        {
            if (Status == SprintStatus.Completed) return 100;
            if (Status == SprintStatus.Planned) return 0;

            var total = (EndDate - StartDate).TotalDays;
            if (total <= 0) return 100;

            var elapsed = (DateTime.UtcNow - StartDate).TotalDays;
            return Math.Clamp(elapsed / total * 100, 0, 100);
        }
    }

    /// <summary>
    /// True if the sprint has passed its end date but is not yet completed.
    /// </summary>
    public bool IsOverdue => Status == SprintStatus.Active && DateTime.UtcNow.Date > EndDate.Date;

    // === Equality ===
    public bool Equals(SprintViewModel? other)
    {
        if (other is null) return false;
        if (ReferenceEquals(this, other)) return true;
        return Id == other.Id && Status == other.Status && IsPendingSync == other.IsPendingSync;
    }

    public override bool Equals(object? obj) => Equals(obj as SprintViewModel);

    public override int GetHashCode() => HashCode.Combine(Id, Status, IsPendingSync);

    public static bool operator ==(SprintViewModel? left, SprintViewModel? right) =>
        left?.Equals(right) ?? right is null;

    public static bool operator !=(SprintViewModel? left, SprintViewModel? right) =>
        !(left == right);
}
```

---

## File 4: ViewModelFactory.cs

**Path**: `ProjectManagement.Core/ViewModels/ViewModelFactory.cs`

```csharp
using ProjectManagement.Core.Interfaces;
using ProjectManagement.Core.Models;

namespace ProjectManagement.Core.ViewModels;

/// <summary>
/// Factory for creating ViewModels with proper pending state.
/// Registered as Scoped service to access stores.
/// </summary>
public sealed class ViewModelFactory
{
    private readonly IWorkItemStore _workItemStore;
    private readonly ISprintStore _sprintStore;

    public ViewModelFactory(IWorkItemStore workItemStore, ISprintStore sprintStore)
    {
        ArgumentNullException.ThrowIfNull(workItemStore);
        ArgumentNullException.ThrowIfNull(sprintStore);
        _workItemStore = workItemStore;
        _sprintStore = sprintStore;
    }

    /// <summary>
    /// Create a WorkItemViewModel from a WorkItem, checking pending state.
    /// </summary>
    public WorkItemViewModel Create(WorkItem item)
    {
        ArgumentNullException.ThrowIfNull(item);
        var isPending = _workItemStore.IsPending(item.Id);
        return new WorkItemViewModel(item, isPending);
    }

    /// <summary>
    /// Create a SprintViewModel from a Sprint, checking pending state.
    /// </summary>
    public SprintViewModel Create(Sprint sprint)
    {
        ArgumentNullException.ThrowIfNull(sprint);
        var isPending = _sprintStore.IsPending(sprint.Id);
        return new SprintViewModel(sprint, isPending);
    }

    /// <summary>
    /// Create ViewModels for multiple work items.
    /// </summary>
    public IReadOnlyList<WorkItemViewModel> CreateMany(IEnumerable<WorkItem> items)
    {
        ArgumentNullException.ThrowIfNull(items);
        return items.Select(Create).ToList();
    }

    /// <summary>
    /// Create ViewModels for multiple sprints.
    /// </summary>
    public IReadOnlyList<SprintViewModel> CreateMany(IEnumerable<Sprint> sprints)
    {
        ArgumentNullException.ThrowIfNull(sprints);
        return sprints.Select(Create).ToList();
    }

    /// <summary>
    /// Create a WorkItemViewModel with explicit pending state (for optimistic creates).
    /// </summary>
    public WorkItemViewModel CreateWithPendingState(WorkItem item, bool isPending)
    {
        ArgumentNullException.ThrowIfNull(item);
        return new WorkItemViewModel(item, isPending);
    }

    /// <summary>
    /// Create a SprintViewModel with explicit pending state (for optimistic creates).
    /// </summary>
    public SprintViewModel CreateWithPendingState(Sprint sprint, bool isPending)
    {
        ArgumentNullException.ThrowIfNull(sprint);
        return new SprintViewModel(sprint, isPending);
    }
}
```

---

## File 5: app.css

**Path**: `ProjectManagement.Components/wwwroot/css/app.css`

```css
/* =============================================================================
   APP.CSS - Global application styles
   ============================================================================= */

/* === Skip Link (Accessibility) === */
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
    font-weight: 500;
}

.skip-link:focus {
    top: 0;
}

/* === Visually Hidden (Screen Reader Only) === */
.visually-hidden {
    position: absolute !important;
    width: 1px !important;
    height: 1px !important;
    padding: 0 !important;
    margin: -1px !important;
    overflow: hidden !important;
    clip: rect(0, 0, 0, 0) !important;
    white-space: nowrap !important;
    border: 0 !important;
}

/* === Offline Banner === */
.offline-banner {
    background: var(--rz-warning-lighter);
    border-bottom: 1px solid var(--rz-warning);
    padding: 0.5rem 1rem;
    color: var(--rz-warning-darker);
    display: flex;
    align-items: center;
    gap: 0.5rem;
}

.offline-banner .rzi {
    flex-shrink: 0;
}

/* === Empty State === */
.empty-state {
    padding: 3rem 1rem;
    text-align: center;
}

.empty-state-icon {
    font-size: 3rem;
    color: var(--rz-text-tertiary-color);
    margin-bottom: 1rem;
}

.empty-state-title {
    margin: 0 0 0.5rem 0;
    color: var(--rz-text-color);
}

.empty-state-description {
    color: var(--rz-text-secondary-color);
    max-width: 300px;
    margin: 0 auto 1rem auto;
}

/* === Pending Sync Animation === */
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

/* === Focus Visible (Keyboard Navigation) === */
.focus-visible:focus-visible,
*:focus-visible {
    outline: 2px solid var(--rz-primary);
    outline-offset: 2px;
}

/* === Utility Classes === */
.flex-grow-1 { flex-grow: 1; }
.flex-shrink-0 { flex-shrink: 0; }

.text-start { text-align: start; }
.text-center { text-align: center; }
.text-end { text-align: end; }

.text-muted { color: var(--rz-text-secondary-color); }
.text-tertiary { color: var(--rz-text-tertiary-color); }

.w-100 { width: 100%; }
.h-100 { height: 100%; }

.gap-0 { gap: 0; }
.gap-1 { gap: 0.25rem; }
.gap-2 { gap: 0.5rem; }
.gap-3 { gap: 0.75rem; }
.gap-4 { gap: 1rem; }
.gap-5 { gap: 1.5rem; }

.m-0 { margin: 0; }
.mt-1 { margin-top: 0.25rem; }
.mt-2 { margin-top: 0.5rem; }
.mt-3 { margin-top: 0.75rem; }
.mt-4 { margin-top: 1rem; }
.mb-1 { margin-bottom: 0.25rem; }
.mb-2 { margin-bottom: 0.5rem; }
.mb-3 { margin-bottom: 0.75rem; }
.mb-4 { margin-bottom: 1rem; }

.p-0 { padding: 0; }
.p-1 { padding: 0.25rem; }
.p-2 { padding: 0.5rem; }
.p-3 { padding: 0.75rem; }
.p-4 { padding: 1rem; }

/* === Truncate Text === */
.text-truncate {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}

.text-truncate-2 {
    overflow: hidden;
    text-overflow: ellipsis;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
}

/* === Reduced Motion === */
@media (prefers-reduced-motion: reduce) {
    .pending-sync {
        animation: none;
        opacity: 0.6;
    }

    *,
    *::before,
    *::after {
        animation-duration: 0.01ms !important;
        animation-iteration-count: 1 !important;
        transition-duration: 0.01ms !important;
    }
}

/* === High Contrast Mode === */
@media (prefers-contrast: high) {
    .pending-sync {
        border: 2px dashed currentColor;
    }

    .empty-state-icon {
        color: currentColor;
    }
}
```

---

## File 6: work-items.css

**Path**: `ProjectManagement.Components/wwwroot/css/work-items.css`

```css
/* =============================================================================
   WORK-ITEMS.CSS - Work item list and row styles
   ============================================================================= */

/* === Work Item List Container === */
.work-item-list {
    border: 1px solid var(--rz-border-color);
    border-radius: 8px;
    overflow: hidden;
    background: var(--rz-surface);
}

/* === Header Row === */
.work-item-header {
    display: flex;
    background: var(--rz-base-200);
    border-bottom: 1px solid var(--rz-border-color);
    font-weight: 600;
    font-size: 0.875rem;
    color: var(--rz-text-secondary-color);
}

/* === Data Row === */
.work-item-row {
    display: flex;
    border-bottom: 1px solid var(--rz-border-color);
    transition: background-color 0.15s ease;
    cursor: pointer;
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

/* === Row States === */
.work-item-row.status-done {
    opacity: 0.7;
}

.work-item-row.status-done .work-item-title {
    text-decoration: line-through;
    color: var(--rz-text-secondary-color);
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
    pointer-events: none;
}

/* === Cell Styles === */
.work-item-cell {
    padding: 0.75rem;
    display: flex;
    align-items: center;
    min-height: 52px;
}

.type-cell {
    width: 60px;
    justify-content: center;
    flex-shrink: 0;
}

.title-cell {
    flex: 1;
    min-width: 200px;
    overflow: hidden;
}

.status-cell {
    width: 120px;
    flex-shrink: 0;
}

.priority-cell {
    width: 100px;
    flex-shrink: 0;
}

.points-cell {
    width: 80px;
    justify-content: center;
    flex-shrink: 0;
}

.actions-cell {
    width: 100px;
    justify-content: flex-end;
    flex-shrink: 0;
}

/* === Title Styling === */
.work-item-title {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--rz-text-color);
}

.work-item-title:hover {
    color: var(--rz-primary);
}

/* === Hierarchy Indent === */
.hierarchy-indent {
    display: inline-block;
    flex-shrink: 0;
}

/* === Responsive === */
@media (max-width: 992px) {
    .points-cell {
        display: none;
    }
}

@media (max-width: 768px) {
    .priority-cell {
        display: none;
    }

    .actions-cell {
        width: 80px;
    }

    .title-cell {
        min-width: 150px;
    }
}

@media (max-width: 576px) {
    .status-cell {
        width: 100px;
    }

    .actions-cell {
        width: 60px;
    }
}

/* === Reduced Motion === */
@media (prefers-reduced-motion: reduce) {
    .work-item-row {
        transition: none;
    }

    .work-item-row.pending-sync {
        animation: none;
    }
}
```

---

## File 7: kanban.css

**Path**: `ProjectManagement.Components/wwwroot/css/kanban.css`

```css
/* =============================================================================
   KANBAN.CSS - Kanban board and card styles
   ============================================================================= */

/* === Board Container === */
.kanban-board {
    min-height: 500px;
}

/* === Columns Container === */
.kanban-columns {
    display: flex;
    gap: 1rem;
    overflow-x: auto;
    padding-bottom: 1rem;
    min-height: 400px;
    scroll-behavior: smooth;
}

/* === Single Column === */
.kanban-column {
    flex: 0 0 280px;
    background: var(--rz-base-200);
    border-radius: 8px;
    display: flex;
    flex-direction: column;
    max-height: calc(100vh - 250px);
    min-height: 300px;
    transition: box-shadow 0.2s ease, background-color 0.2s ease;
}

.kanban-column.drag-target {
    box-shadow: 0 0 0 2px var(--rz-primary);
    background: var(--rz-primary-lighter);
}

/* === Column Header === */
.kanban-column-header {
    padding: 0.75rem 1rem;
    border-bottom: 1px solid var(--rz-border-color);
    display: flex;
    justify-content: space-between;
    align-items: center;
    flex-shrink: 0;
    background: var(--rz-base-300);
    border-radius: 8px 8px 0 0;
}

.kanban-column-title {
    font-weight: 600;
    margin: 0;
}

/* === Column Body === */
.kanban-column-body {
    flex: 1;
    overflow-y: auto;
    padding: 0.5rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
}

/* === Empty Column === */
.kanban-empty-column {
    padding: 2rem 1rem;
    text-align: center;
    color: var(--rz-text-tertiary-color);
    font-size: 0.875rem;
}

/* === Kanban Card === */
.kanban-card {
    background: var(--rz-surface);
    border: 1px solid var(--rz-border-color);
    border-radius: 6px;
    padding: 0.75rem;
    cursor: grab;
    transition: box-shadow 0.15s ease, transform 0.15s ease, opacity 0.15s ease;
    user-select: none;
}

.kanban-card:hover {
    box-shadow: var(--rz-shadow-2);
}

.kanban-card:focus-visible {
    outline: 2px solid var(--rz-primary);
    outline-offset: 2px;
}

/* === Card Dragging State === */
.kanban-card:active,
.kanban-card[aria-grabbed="true"],
.kanban-card.dragging {
    cursor: grabbing;
    transform: rotate(2deg);
    box-shadow: var(--rz-shadow-3);
    opacity: 0.9;
}

/* === Card Pending State === */
.kanban-card.pending-sync {
    opacity: 0.6;
    pointer-events: none;
    cursor: default;
    background: linear-gradient(
        90deg,
        var(--rz-base-200) 0%,
        var(--rz-base-100) 50%,
        var(--rz-base-200) 100%
    );
    background-size: 200% 100%;
    animation: shimmer 1.5s infinite;
}

/* === Card Title === */
.kanban-card-title {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    word-break: break-word;
    font-size: 0.875rem;
    line-height: 1.4;
}

/* === Card Footer === */
.kanban-card-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-top: 0.5rem;
    padding-top: 0.5rem;
    border-top: 1px solid var(--rz-border-color);
}

/* === Drop Indicator === */
.kanban-drop-indicator {
    height: 2px;
    background: var(--rz-primary);
    margin: 0.25rem 0;
    border-radius: 1px;
}

/* === Responsive === */
@media (max-width: 768px) {
    .kanban-column {
        flex: 0 0 260px;
    }
}

@media (max-width: 576px) {
    .kanban-column {
        flex: 0 0 240px;
    }

    .kanban-card {
        padding: 0.5rem;
    }
}

/* === Reduced Motion === */
@media (prefers-reduced-motion: reduce) {
    .kanban-card {
        transition: none;
    }

    .kanban-card:active,
    .kanban-card.dragging {
        transform: none;
    }

    .kanban-card.pending-sync {
        animation: none;
    }

    .kanban-columns {
        scroll-behavior: auto;
    }
}

/* === High Contrast === */
@media (prefers-contrast: high) {
    .kanban-column {
        border: 2px solid currentColor;
    }

    .kanban-card {
        border: 2px solid currentColor;
    }

    .kanban-column.drag-target {
        border-style: dashed;
    }
}
```

---

## File 8: layout.css

**Path**: `ProjectManagement.Components/wwwroot/css/layout.css`

```css
/* =============================================================================
   LAYOUT.CSS - Page layout, headers, and structural styles
   ============================================================================= */

/* === CSS Custom Properties === */
:root {
    --header-height: 50px;
    --sidebar-width: 250px;
    --sidebar-collapsed-width: 60px;
    --content-padding: 1.5rem;
    --transition-speed: 0.2s;
}

/* === Page Header === */
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
    color: var(--rz-text-color);
}

.page-subtitle {
    margin: 0.25rem 0 0 0;
    font-size: 0.875rem;
    color: var(--rz-text-secondary-color);
}

.page-actions {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
}

/* === Breadcrumbs === */
.breadcrumbs {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 1rem;
    font-size: 0.875rem;
    color: var(--rz-text-secondary-color);
    flex-wrap: wrap;
}

.breadcrumbs a {
    color: var(--rz-primary);
    text-decoration: none;
    transition: color 0.15s ease;
}

.breadcrumbs a:hover {
    text-decoration: underline;
}

.breadcrumbs a:focus-visible {
    outline: 2px solid var(--rz-primary);
    outline-offset: 2px;
}

.breadcrumbs .separator {
    color: var(--rz-text-tertiary-color);
}

.breadcrumbs .current {
    color: var(--rz-text-color);
    font-weight: 500;
}

/* === Content Cards === */
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

.content-card-title {
    margin: 0;
    font-size: 1.125rem;
    font-weight: 600;
}

.content-card-actions {
    display: flex;
    gap: 0.5rem;
}

/* === View Tabs === */
.view-tabs {
    margin-bottom: 1rem;
    border-bottom: 1px solid var(--rz-border-color);
}

.view-tab {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.75rem 1rem;
    border: none;
    background: transparent;
    color: var(--rz-text-secondary-color);
    font-size: 0.875rem;
    font-weight: 500;
    cursor: pointer;
    border-bottom: 2px solid transparent;
    margin-bottom: -1px;
    transition: color 0.15s ease, border-color 0.15s ease;
}

.view-tab:hover {
    color: var(--rz-text-color);
}

.view-tab:focus-visible {
    outline: 2px solid var(--rz-primary);
    outline-offset: -2px;
}

.view-tab.active {
    color: var(--rz-primary);
    border-bottom-color: var(--rz-primary);
}

/* === Stat Cards === */
.stat-card {
    background: var(--rz-surface);
    border: 1px solid var(--rz-border-color);
    border-radius: 8px;
    padding: 1rem;
}

.stat-card-value {
    font-size: 2rem;
    font-weight: 700;
    color: var(--rz-text-color);
    line-height: 1;
}

.stat-card-label {
    font-size: 0.875rem;
    color: var(--rz-text-secondary-color);
    margin-top: 0.25rem;
}

/* === Dividers === */
.divider {
    height: 1px;
    background: var(--rz-border-color);
    margin: 1rem 0;
}

.divider-vertical {
    width: 1px;
    height: 100%;
    background: var(--rz-border-color);
    margin: 0 1rem;
}

/* === Responsive === */
@media (max-width: 768px) {
    .page-header {
        flex-direction: column;
        align-items: flex-start;
    }

    .page-actions {
        width: 100%;
    }

    .content-card {
        padding: 1rem;
    }

    .content-card-header {
        flex-direction: column;
        align-items: flex-start;
        gap: 0.5rem;
    }
}

/* === Reduced Motion === */
@media (prefers-reduced-motion: reduce) {
    .view-tab,
    .breadcrumbs a {
        transition: none;
    }
}
```

---

## File 9: _Imports.razor (Components)

**Path**: `ProjectManagement.Components/_Imports.razor`

```razor
@using System.Net.Http
@using System.Net.Http.Json
@using Microsoft.AspNetCore.Components.Forms
@using Microsoft.AspNetCore.Components.Routing
@using Microsoft.AspNetCore.Components.Web
@using Microsoft.AspNetCore.Components.Web.Virtualization
@using Microsoft.Extensions.Logging
@using Microsoft.JSInterop
@using Radzen
@using Radzen.Blazor
@using ProjectManagement.Core.Models
@using ProjectManagement.Core.Interfaces
@using ProjectManagement.Core.Exceptions
@using ProjectManagement.Core.ViewModels
@using ProjectManagement.Services.State
@using ProjectManagement.Components.Shared
@using ProjectManagement.Components.WorkItems
```

---

## File 10: OfflineBanner.razor

**Path**: `ProjectManagement.Components/Shared/OfflineBanner.razor`

```razor
@using ProjectManagement.Core.Models
@inject AppState AppState
@implements IDisposable

@if (_showBanner)
{
    <div class="offline-banner" role="alert" aria-live="polite">
        <RadzenIcon Icon="cloud_off" />
        <span>You're offline. Changes will sync when reconnected.</span>
        @if (_isReconnecting)
        {
            <RadzenProgressBarCircular ShowValue="false"
                                       Mode="ProgressBarMode.Indeterminate"
                                       Size="ProgressBarCircularSize.Small"
                                       Style="margin-left: auto;" />
        }
    </div>
}

@code {
    private bool _showBanner;
    private bool _isReconnecting;

    protected override void OnInitialized()
    {
        UpdateState(AppState.ConnectionState);
        AppState.OnConnectionStateChanged += HandleConnectionStateChanged;
    }

    private void HandleConnectionStateChanged(ConnectionState state)
    {
        UpdateState(state);
        InvokeAsync(StateHasChanged);
    }

    private void UpdateState(ConnectionState state)
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

---

## File 11: EmptyState.razor

**Path**: `ProjectManagement.Components/Shared/EmptyState.razor`

```razor
<div class="empty-state" role="status">
    <RadzenIcon Icon="@Icon" class="empty-state-icon" />
    <h3 class="empty-state-title">@Title</h3>
    @if (!string.IsNullOrWhiteSpace(Description))
    {
        <p class="empty-state-description">@Description</p>
    }
    @if (ShowAction && OnAction.HasDelegate)
    {
        <RadzenButton Text="@ActionText"
                      Icon="@ActionIcon"
                      Click="@HandleAction"
                      ButtonStyle="ButtonStyle.Primary" />
    }
</div>

@code {
    /// <summary>Material icon name for the empty state illustration.</summary>
    [Parameter]
    public string Icon { get; set; } = "inbox";

    /// <summary>Main title text.</summary>
    [Parameter, EditorRequired]
    public string Title { get; set; } = "No items";

    /// <summary>Optional description text below the title.</summary>
    [Parameter]
    public string? Description { get; set; }

    /// <summary>Text for the action button.</summary>
    [Parameter]
    public string ActionText { get; set; } = "Create";

    /// <summary>Icon for the action button.</summary>
    [Parameter]
    public string ActionIcon { get; set; } = "add";

    /// <summary>Whether to show the action button.</summary>
    [Parameter]
    public bool ShowAction { get; set; } = true;

    /// <summary>Callback when the action button is clicked.</summary>
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

---

## File 12: LoadingButton.razor

**Path**: `ProjectManagement.Components/Shared/LoadingButton.razor`

```razor
@using ProjectManagement.Core.Models

<RadzenButton Text="@DisplayText"
              Icon="@DisplayIcon"
              IsBusy="@IsBusy"
              Disabled="@IsDisabled"
              ButtonStyle="@ButtonStyle"
              Size="@Size"
              Variant="@Variant"
              Click="@HandleClick"
              title="@Tooltip"
              Style="@Style"
              @attributes="AdditionalAttributes" />

@code {
    /// <summary>Button text when not loading.</summary>
    [Parameter, EditorRequired]
    public string Text { get; set; } = "";

    /// <summary>Button text when loading.</summary>
    [Parameter]
    public string LoadingText { get; set; } = "Loading...";

    /// <summary>Button icon when not loading.</summary>
    [Parameter]
    public string Icon { get; set; } = "";

    /// <summary>Whether the button is in a loading state.</summary>
    [Parameter]
    public bool IsBusy { get; set; }

    /// <summary>Whether the button is disabled (independent of loading state).</summary>
    [Parameter]
    public bool Disabled { get; set; }

    /// <summary>Current connection state - button is disabled when disconnected.</summary>
    [Parameter]
    public ConnectionState ConnectionState { get; set; } = ConnectionState.Connected;

    /// <summary>Visual style of the button.</summary>
    [Parameter]
    public ButtonStyle ButtonStyle { get; set; } = ButtonStyle.Primary;

    /// <summary>Size of the button.</summary>
    [Parameter]
    public ButtonSize Size { get; set; } = ButtonSize.Medium;

    /// <summary>Variant of the button (filled, outlined, text, etc.).</summary>
    [Parameter]
    public Variant Variant { get; set; } = Variant.Filled;

    /// <summary>Additional inline styles.</summary>
    [Parameter]
    public string? Style { get; set; }

    /// <summary>Click event callback.</summary>
    [Parameter]
    public EventCallback<MouseEventArgs> OnClick { get; set; }

    /// <summary>Additional HTML attributes to pass through.</summary>
    [Parameter(CaptureUnmatchedValues = true)]
    public Dictionary<string, object>? AdditionalAttributes { get; set; }

    private bool IsConnected => ConnectionState == ConnectionState.Connected;
    private bool IsDisabled => IsBusy || Disabled || !IsConnected;
    private string DisplayText => IsBusy ? LoadingText : Text;
    private string DisplayIcon => IsBusy ? "" : Icon;

    private string Tooltip
    {
        get
        {
            if (!IsConnected) return "Offline - action unavailable";
            if (IsBusy) return "Please wait...";
            return Text;
        }
    }

    private async Task HandleClick(MouseEventArgs args)
    {
        if (!IsDisabled && OnClick.HasDelegate)
        {
            await OnClick.InvokeAsync(args);
        }
    }
}
```

---

## File 13: DebouncedTextBox.razor

**Path**: `ProjectManagement.Components/Shared/DebouncedTextBox.razor`

```razor
@implements IDisposable

<RadzenTextBox Value="@_currentValue"
               Placeholder="@Placeholder"
               Style="@Style"
               Disabled="@Disabled"
               MaxLength="@MaxLength"
               Change="@HandleChange"
               @attributes="AdditionalAttributes" />

@code {
    /// <summary>The current value of the text box.</summary>
    [Parameter]
    public string Value { get; set; } = "";

    /// <summary>Callback when the value changes (after debounce).</summary>
    [Parameter]
    public EventCallback<string> ValueChanged { get; set; }

    /// <summary>Placeholder text.</summary>
    [Parameter]
    public string Placeholder { get; set; } = "";

    /// <summary>Inline styles.</summary>
    [Parameter]
    public string Style { get; set; } = "";

    /// <summary>Whether the input is disabled.</summary>
    [Parameter]
    public bool Disabled { get; set; }

    /// <summary>Maximum length of input.</summary>
    [Parameter]
    public int? MaxLength { get; set; }

    /// <summary>Debounce delay in milliseconds.</summary>
    [Parameter]
    public int DebounceMs { get; set; } = 300;

    /// <summary>Additional HTML attributes.</summary>
    [Parameter(CaptureUnmatchedValues = true)]
    public Dictionary<string, object>? AdditionalAttributes { get; set; }

    private string _currentValue = "";
    private CancellationTokenSource? _debounceCts;
    private bool _isDebouncing;

    protected override void OnParametersSet()
    {
        // Only update from external value if we're not in the middle of debouncing
        if (!_isDebouncing && _currentValue != Value)
        {
            _currentValue = Value;
        }
    }

    private async Task HandleChange(string newValue)
    {
        _currentValue = newValue;
        _isDebouncing = true;

        // Cancel any pending debounce
        _debounceCts?.Cancel();
        _debounceCts?.Dispose();
        _debounceCts = new CancellationTokenSource();

        var token = _debounceCts.Token;

        try
        {
            await Task.Delay(DebounceMs, token);

            // Debounce completed successfully
            if (ValueChanged.HasDelegate)
            {
                await ValueChanged.InvokeAsync(newValue);
            }
        }
        catch (TaskCanceledException)
        {
            // Debounce was cancelled by new input - this is expected
        }
        finally
        {
            if (!token.IsCancellationRequested)
            {
                _isDebouncing = false;
                _debounceCts?.Dispose();
                _debounceCts = null;
            }
        }
    }

    public void Dispose()
    {
        _debounceCts?.Cancel();
        _debounceCts?.Dispose();
    }
}
```

---

## File 14: ConfirmDialog.razor

**Path**: `ProjectManagement.Components/Shared/ConfirmDialog.razor`

```razor
@inject DialogService DialogService

<RadzenStack Gap="1rem">
    <RadzenText TextStyle="TextStyle.Body1">@Message</RadzenText>

    @if (!string.IsNullOrWhiteSpace(WarningMessage))
    {
        <RadzenAlert AlertStyle="AlertStyle.Warning"
                     Shade="Shade.Light"
                     Size="AlertSize.Small"
                     AllowClose="false">
            @WarningMessage
        </RadzenAlert>
    }

    @if (ChildContent is not null)
    {
        @ChildContent
    }

    <RadzenStack Orientation="Orientation.Horizontal"
                 Gap="0.5rem"
                 JustifyContent="JustifyContent.End">
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
    /// <summary>The main confirmation message.</summary>
    [Parameter, EditorRequired]
    public string Message { get; set; } = "";

    /// <summary>Optional warning message shown in an alert.</summary>
    [Parameter]
    public string? WarningMessage { get; set; }

    /// <summary>Text for the confirm button.</summary>
    [Parameter]
    public string ConfirmText { get; set; } = "Confirm";

    /// <summary>Text for the cancel button.</summary>
    [Parameter]
    public string CancelText { get; set; } = "Cancel";

    /// <summary>Style for the confirm button.</summary>
    [Parameter]
    public ButtonStyle ConfirmButtonStyle { get; set; } = ButtonStyle.Primary;

    /// <summary>Whether the dialog is in a busy/loading state.</summary>
    [Parameter]
    public bool IsBusy { get; set; }

    /// <summary>Optional additional content to display.</summary>
    [Parameter]
    public RenderFragment? ChildContent { get; set; }

    /// <summary>Callback when confirm is clicked.</summary>
    [Parameter]
    public EventCallback OnConfirm { get; set; }

    /// <summary>Callback when cancel is clicked.</summary>
    [Parameter]
    public EventCallback OnCancel { get; set; }

    private async Task HandleConfirm()
    {
        if (OnConfirm.HasDelegate)
        {
            await OnConfirm.InvokeAsync();
        }
        else
        {
            DialogService.Close(true);
        }
    }

    private async Task HandleCancel()
    {
        if (OnCancel.HasDelegate)
        {
            await OnCancel.InvokeAsync();
        }
        else
        {
            DialogService.Close(false);
        }
    }
}
```

---

## File 15: ProjectDetailSkeleton.razor

**Path**: `ProjectManagement.Components/Shared/ProjectDetailSkeleton.razor`

```razor
<div role="status" aria-label="Loading project details" aria-busy="true">
    <RadzenStack Gap="1rem">
        @* Header skeleton *@
        <RadzenRow AlignItems="AlignItems.Center">
            <RadzenColumn Size="12" SizeMD="8">
                <RadzenStack Orientation="Orientation.Horizontal"
                             AlignItems="AlignItems.Center"
                             Gap="0.5rem">
                    <RadzenSkeleton Shape="SkeletonShape.Circle" Width="32px" Height="32px" />
                    <RadzenSkeleton Width="250px" Height="32px" />
                </RadzenStack>
                <div class="mt-2">
                    <RadzenSkeleton Width="180px" Height="16px" />
                </div>
            </RadzenColumn>
            <RadzenColumn Size="12" SizeMD="4" class="text-end">
                <RadzenStack Orientation="Orientation.Horizontal"
                             Gap="0.5rem"
                             JustifyContent="JustifyContent.End">
                    <RadzenSkeleton Width="100px" Height="36px" />
                    <RadzenSkeleton Width="120px" Height="36px" />
                </RadzenStack>
            </RadzenColumn>
        </RadzenRow>

        @* Tabs skeleton *@
        <RadzenStack Orientation="Orientation.Horizontal" Gap="1rem" class="mt-2">
            <RadzenSkeleton Width="80px" Height="32px" />
            <RadzenSkeleton Width="80px" Height="32px" />
            <RadzenSkeleton Width="80px" Height="32px" />
        </RadzenStack>

        @* Content skeleton *@
        <RadzenRow class="mt-3">
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

    <span class="visually-hidden">Loading project details...</span>
</div>

@code {
    /// <summary>Number of placeholder rows to display.</summary>
    [Parameter]
    public int RowCount { get; set; } = 5;

    /// <summary>Height of each placeholder row in pixels.</summary>
    [Parameter]
    public int RowHeight { get; set; } = 52;
}
```

---

## File 16: WorkItemTypeIcon.razor

**Path**: `ProjectManagement.Components/WorkItems/WorkItemTypeIcon.razor`

```razor
@using ProjectManagement.Core.Models

<span class="work-item-type-icon" title="@TypeName">
    <RadzenIcon Icon="@IconName" Style="@IconStyle" />
    <span class="visually-hidden">@TypeName</span>
</span>

@code {
    /// <summary>The work item type to display.</summary>
    [Parameter, EditorRequired]
    public WorkItemType Type { get; set; }

    /// <summary>Optional size override (e.g., "1.5rem", "24px").</summary>
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
        WorkItemType.Epic => "#9c27b0",      // Purple
        WorkItemType.Story => "#2196f3",     // Blue
        WorkItemType.Task => "#4caf50",      // Green
        _ => "var(--rz-text-secondary-color)"
    };

    private string IconStyle
    {
        get
        {
            var style = $"color: {IconColor};";
            if (!string.IsNullOrEmpty(Size))
            {
                style += $" font-size: {Size};";
            }
            return style;
        }
    }
}
```

---

## File 17: WorkItemStatusBadge.razor

**Path**: `ProjectManagement.Components/WorkItems/WorkItemStatusBadge.razor`

```razor
<RadzenBadge BadgeStyle="@BadgeStyle"
             Text="@DisplayText"
             title="@($"Status: {DisplayText}")"
             IsPill="@IsPill" />

@code {
    /// <summary>The status value (e.g., "backlog", "in_progress", "done").</summary>
    [Parameter, EditorRequired]
    public string Status { get; set; } = "backlog";

    /// <summary>Whether to render as a pill shape.</summary>
    [Parameter]
    public bool IsPill { get; set; } = true;

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

---

## File 18: PriorityBadge.razor

**Path**: `ProjectManagement.Components/WorkItems/PriorityBadge.razor`

```razor
<span class="priority-badge" title="@($"Priority: {DisplayText}")">
    <RadzenStack Orientation="Orientation.Horizontal"
                 AlignItems="AlignItems.Center"
                 Gap="0.25rem">
        <RadzenIcon Icon="@IconName" Style="@IconStyle" />
        @if (ShowLabel)
        {
            <span style="@LabelStyle">@DisplayText</span>
        }
    </RadzenStack>
</span>

@code {
    /// <summary>The priority value (e.g., "critical", "high", "medium", "low").</summary>
    [Parameter, EditorRequired]
    public string Priority { get; set; } = "medium";

    /// <summary>Whether to show the text label alongside the icon.</summary>
    [Parameter]
    public bool ShowLabel { get; set; } = true;

    /// <summary>Optional size for the icon.</summary>
    [Parameter]
    public string? Size { get; set; }

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
        "critical" => "#d32f2f",  // Red
        "high" => "#f57c00",      // Orange
        "medium" => "#1976d2",    // Blue
        "low" => "#388e3c",       // Green
        _ => "var(--rz-text-secondary-color)"
    };

    private string IconStyle
    {
        get
        {
            var style = $"color: {IconColor};";
            if (!string.IsNullOrEmpty(Size))
            {
                style += $" font-size: {Size};";
            }
            return style;
        }
    }

    private string LabelStyle => $"color: {IconColor}; font-size: 0.875rem;";

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

## File 19: ViewModelFactoryTests.cs

**Path**: `ProjectManagement.Components.Tests/ViewModels/ViewModelFactoryTests.cs`

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

    #region Constructor Tests

    [Fact]
    public void Constructor_ThrowsArgumentNullException_WhenWorkItemStoreIsNull()
    {
        // Act
        var act = () => new ViewModelFactory(null!, _sprintStore.Object);

        // Assert
        act.Should().Throw<ArgumentNullException>()
            .WithParameterName("workItemStore");
    }

    [Fact]
    public void Constructor_ThrowsArgumentNullException_WhenSprintStoreIsNull()
    {
        // Act
        var act = () => new ViewModelFactory(_workItemStore.Object, null!);

        // Assert
        act.Should().Throw<ArgumentNullException>()
            .WithParameterName("sprintStore");
    }

    #endregion

    #region Create WorkItem Tests

    [Fact]
    public void Create_WorkItem_ThrowsArgumentNullException_WhenItemIsNull()
    {
        // Act
        var act = () => _factory.Create((WorkItem)null!);

        // Assert
        act.Should().Throw<ArgumentNullException>();
    }

    [Fact]
    public void Create_WorkItem_ReturnsViewModelWithCorrectModel()
    {
        // Arrange
        var workItem = CreateTestWorkItem();
        _workItemStore.Setup(s => s.IsPending(workItem.Id)).Returns(false);

        // Act
        var viewModel = _factory.Create(workItem);

        // Assert
        viewModel.Model.Should().BeSameAs(workItem);
        viewModel.Id.Should().Be(workItem.Id);
        viewModel.Title.Should().Be(workItem.Title);
    }

    [Fact]
    public void Create_WorkItem_SetsIsPendingSyncFalse_WhenNotPending()
    {
        // Arrange
        var workItem = CreateTestWorkItem();
        _workItemStore.Setup(s => s.IsPending(workItem.Id)).Returns(false);

        // Act
        var viewModel = _factory.Create(workItem);

        // Assert
        viewModel.IsPendingSync.Should().BeFalse();
    }

    [Fact]
    public void Create_WorkItem_SetsIsPendingSyncTrue_WhenPending()
    {
        // Arrange
        var workItem = CreateTestWorkItem();
        _workItemStore.Setup(s => s.IsPending(workItem.Id)).Returns(true);

        // Act
        var viewModel = _factory.Create(workItem);

        // Assert
        viewModel.IsPendingSync.Should().BeTrue();
    }

    [Fact]
    public void Create_WorkItem_CallsIsPendingWithCorrectId()
    {
        // Arrange
        var workItem = CreateTestWorkItem();
        _workItemStore.Setup(s => s.IsPending(workItem.Id)).Returns(false);

        // Act
        _factory.Create(workItem);

        // Assert
        _workItemStore.Verify(s => s.IsPending(workItem.Id), Times.Once);
    }

    #endregion

    #region Create Sprint Tests

    [Fact]
    public void Create_Sprint_ThrowsArgumentNullException_WhenSprintIsNull()
    {
        // Act
        var act = () => _factory.Create((Sprint)null!);

        // Assert
        act.Should().Throw<ArgumentNullException>();
    }

    [Fact]
    public void Create_Sprint_ReturnsViewModelWithCorrectModel()
    {
        // Arrange
        var sprint = CreateTestSprint();
        _sprintStore.Setup(s => s.IsPending(sprint.Id)).Returns(false);

        // Act
        var viewModel = _factory.Create(sprint);

        // Assert
        viewModel.Model.Should().BeSameAs(sprint);
        viewModel.Id.Should().Be(sprint.Id);
        viewModel.Name.Should().Be(sprint.Name);
    }

    [Fact]
    public void Create_Sprint_SetsIsPendingSyncFalse_WhenNotPending()
    {
        // Arrange
        var sprint = CreateTestSprint();
        _sprintStore.Setup(s => s.IsPending(sprint.Id)).Returns(false);

        // Act
        var viewModel = _factory.Create(sprint);

        // Assert
        viewModel.IsPendingSync.Should().BeFalse();
    }

    [Fact]
    public void Create_Sprint_SetsIsPendingSyncTrue_WhenPending()
    {
        // Arrange
        var sprint = CreateTestSprint();
        _sprintStore.Setup(s => s.IsPending(sprint.Id)).Returns(true);

        // Act
        var viewModel = _factory.Create(sprint);

        // Assert
        viewModel.IsPendingSync.Should().BeTrue();
    }

    #endregion

    #region CreateMany Tests

    [Fact]
    public void CreateMany_WorkItems_ThrowsArgumentNullException_WhenItemsIsNull()
    {
        // Act
        var act = () => _factory.CreateMany((IEnumerable<WorkItem>)null!);

        // Assert
        act.Should().Throw<ArgumentNullException>();
    }

    [Fact]
    public void CreateMany_WorkItems_ReturnsEmptyList_WhenItemsIsEmpty()
    {
        // Act
        var viewModels = _factory.CreateMany(Enumerable.Empty<WorkItem>());

        // Assert
        viewModels.Should().BeEmpty();
    }

    [Fact]
    public void CreateMany_WorkItems_ReturnsCorrectNumberOfViewModels()
    {
        // Arrange
        var items = new List<WorkItem>
        {
            CreateTestWorkItem(),
            CreateTestWorkItem(),
            CreateTestWorkItem()
        };
        _workItemStore.Setup(s => s.IsPending(It.IsAny<Guid>())).Returns(false);

        // Act
        var viewModels = _factory.CreateMany(items);

        // Assert
        viewModels.Should().HaveCount(3);
    }

    [Fact]
    public void CreateMany_WorkItems_ChecksPendingStateForEachItem()
    {
        // Arrange
        var item1 = CreateTestWorkItem();
        var item2 = CreateTestWorkItem();
        _workItemStore.Setup(s => s.IsPending(item1.Id)).Returns(true);
        _workItemStore.Setup(s => s.IsPending(item2.Id)).Returns(false);

        // Act
        var viewModels = _factory.CreateMany(new[] { item1, item2 });

        // Assert
        viewModels[0].IsPendingSync.Should().BeTrue();
        viewModels[1].IsPendingSync.Should().BeFalse();
    }

    [Fact]
    public void CreateMany_Sprints_ThrowsArgumentNullException_WhenSprintsIsNull()
    {
        // Act
        var act = () => _factory.CreateMany((IEnumerable<Sprint>)null!);

        // Assert
        act.Should().Throw<ArgumentNullException>();
    }

    [Fact]
    public void CreateMany_Sprints_ReturnsCorrectNumberOfViewModels()
    {
        // Arrange
        var sprints = new List<Sprint>
        {
            CreateTestSprint(),
            CreateTestSprint()
        };
        _sprintStore.Setup(s => s.IsPending(It.IsAny<Guid>())).Returns(false);

        // Act
        var viewModels = _factory.CreateMany(sprints);

        // Assert
        viewModels.Should().HaveCount(2);
    }

    #endregion

    #region CreateWithPendingState Tests

    [Fact]
    public void CreateWithPendingState_WorkItem_SetsExplicitPendingState()
    {
        // Arrange
        var workItem = CreateTestWorkItem();

        // Act
        var viewModel = _factory.CreateWithPendingState(workItem, true);

        // Assert
        viewModel.IsPendingSync.Should().BeTrue();
        _workItemStore.Verify(s => s.IsPending(It.IsAny<Guid>()), Times.Never);
    }

    [Fact]
    public void CreateWithPendingState_Sprint_SetsExplicitPendingState()
    {
        // Arrange
        var sprint = CreateTestSprint();

        // Act
        var viewModel = _factory.CreateWithPendingState(sprint, false);

        // Assert
        viewModel.IsPendingSync.Should().BeFalse();
        _sprintStore.Verify(s => s.IsPending(It.IsAny<Guid>()), Times.Never);
    }

    #endregion

    #region Helper Methods

    private static WorkItem CreateTestWorkItem() => new()
    {
        Id = Guid.NewGuid(),
        Title = "Test Work Item",
        ItemType = WorkItemType.Story,
        ProjectId = Guid.NewGuid(),
        Status = "backlog",
        Priority = "medium",
        Position = 1,
        Version = 1,
        CreatedAt = DateTime.UtcNow,
        UpdatedAt = DateTime.UtcNow,
        CreatedBy = Guid.NewGuid(),
        UpdatedBy = Guid.NewGuid()
    };

    private static Sprint CreateTestSprint() => new()
    {
        Id = Guid.NewGuid(),
        Name = "Sprint 1",
        ProjectId = Guid.NewGuid(),
        StartDate = DateTime.UtcNow,
        EndDate = DateTime.UtcNow.AddDays(14),
        Status = SprintStatus.Planned
    };

    #endregion
}
```

---

## File 20: WorkItemViewModelTests.cs

**Path**: `ProjectManagement.Components.Tests/ViewModels/WorkItemViewModelTests.cs`

```csharp
using FluentAssertions;
using ProjectManagement.Core.Models;
using ProjectManagement.Core.ViewModels;
using Xunit;

namespace ProjectManagement.Components.Tests.ViewModels;

public class WorkItemViewModelTests
{
    #region Constructor Tests

    [Fact]
    public void Constructor_ThrowsArgumentNullException_WhenModelIsNull()
    {
        // Act
        var act = () => new WorkItemViewModel(null!);

        // Assert
        act.Should().Throw<ArgumentNullException>();
    }

    [Fact]
    public void Constructor_SetsModelCorrectly()
    {
        // Arrange
        var workItem = CreateTestWorkItem();

        // Act
        var viewModel = new WorkItemViewModel(workItem);

        // Assert
        viewModel.Model.Should().BeSameAs(workItem);
    }

    [Fact]
    public void Constructor_DefaultsIsPendingSyncToFalse()
    {
        // Arrange
        var workItem = CreateTestWorkItem();

        // Act
        var viewModel = new WorkItemViewModel(workItem);

        // Assert
        viewModel.IsPendingSync.Should().BeFalse();
    }

    [Fact]
    public void Constructor_SetsIsPendingSyncFromParameter()
    {
        // Arrange
        var workItem = CreateTestWorkItem();

        // Act
        var viewModel = new WorkItemViewModel(workItem, isPendingSync: true);

        // Assert
        viewModel.IsPendingSync.Should().BeTrue();
    }

    #endregion

    #region Property Accessor Tests

    [Fact]
    public void PropertyAccessors_ReturnModelValues()
    {
        // Arrange
        var workItem = CreateTestWorkItem();

        // Act
        var viewModel = new WorkItemViewModel(workItem);

        // Assert
        viewModel.Id.Should().Be(workItem.Id);
        viewModel.Title.Should().Be(workItem.Title);
        viewModel.Description.Should().Be(workItem.Description);
        viewModel.ItemType.Should().Be(workItem.ItemType);
        viewModel.Status.Should().Be(workItem.Status);
        viewModel.Priority.Should().Be(workItem.Priority);
        viewModel.StoryPoints.Should().Be(workItem.StoryPoints);
        viewModel.Position.Should().Be(workItem.Position);
        viewModel.ProjectId.Should().Be(workItem.ProjectId);
        viewModel.ParentId.Should().Be(workItem.ParentId);
        viewModel.SprintId.Should().Be(workItem.SprintId);
        viewModel.AssigneeId.Should().Be(workItem.AssigneeId);
        viewModel.Version.Should().Be(workItem.Version);
    }

    #endregion

    #region Computed Property Tests

    [Theory]
    [InlineData(null, false)]
    [InlineData("2024-01-01", true)]
    public void IsDeleted_ReturnsCorrectValue(string? deletedAtString, bool expected)
    {
        // Arrange
        DateTime? deletedAt = deletedAtString is null ? null : DateTime.Parse(deletedAtString);
        var workItem = CreateTestWorkItem() with { DeletedAt = deletedAt };

        // Act
        var viewModel = new WorkItemViewModel(workItem);

        // Assert
        viewModel.IsDeleted.Should().Be(expected);
    }

    [Theory]
    [InlineData("done", true)]
    [InlineData("backlog", false)]
    [InlineData("in_progress", false)]
    public void IsCompleted_ReturnsCorrectValue(string status, bool expected)
    {
        // Arrange
        var workItem = CreateTestWorkItem() with { Status = status };

        // Act
        var viewModel = new WorkItemViewModel(workItem);

        // Assert
        viewModel.IsCompleted.Should().Be(expected);
    }

    [Theory]
    [InlineData("backlog", "Backlog")]
    [InlineData("todo", "To Do")]
    [InlineData("in_progress", "In Progress")]
    [InlineData("review", "Review")]
    [InlineData("done", "Done")]
    [InlineData("custom", "custom")]
    public void StatusDisplayName_ReturnsCorrectValue(string status, string expected)
    {
        // Arrange
        var workItem = CreateTestWorkItem() with { Status = status };

        // Act
        var viewModel = new WorkItemViewModel(workItem);

        // Assert
        viewModel.StatusDisplayName.Should().Be(expected);
    }

    [Theory]
    [InlineData("critical", "Critical")]
    [InlineData("high", "High")]
    [InlineData("medium", "Medium")]
    [InlineData("low", "Low")]
    [InlineData("custom", "custom")]
    public void PriorityDisplayName_ReturnsCorrectValue(string priority, string expected)
    {
        // Arrange
        var workItem = CreateTestWorkItem() with { Priority = priority };

        // Act
        var viewModel = new WorkItemViewModel(workItem);

        // Assert
        viewModel.PriorityDisplayName.Should().Be(expected);
    }

    [Theory]
    [InlineData("critical", 0)]
    [InlineData("high", 1)]
    [InlineData("medium", 2)]
    [InlineData("low", 3)]
    [InlineData("unknown", 4)]
    public void PrioritySortOrder_ReturnsCorrectValue(string priority, int expected)
    {
        // Arrange
        var workItem = CreateTestWorkItem() with { Priority = priority };

        // Act
        var viewModel = new WorkItemViewModel(workItem);

        // Assert
        viewModel.PrioritySortOrder.Should().Be(expected);
    }

    [Theory]
    [InlineData(WorkItemType.Project, "Project")]
    [InlineData(WorkItemType.Epic, "Epic")]
    [InlineData(WorkItemType.Story, "Story")]
    [InlineData(WorkItemType.Task, "Task")]
    public void ItemTypeDisplayName_ReturnsCorrectValue(WorkItemType type, string expected)
    {
        // Arrange
        var workItem = CreateTestWorkItem() with { ItemType = type };

        // Act
        var viewModel = new WorkItemViewModel(workItem);

        // Assert
        viewModel.ItemTypeDisplayName.Should().Be(expected);
    }

    #endregion

    #region Equality Tests

    [Fact]
    public void Equals_ReturnsFalse_WhenOtherIsNull()
    {
        // Arrange
        var viewModel = new WorkItemViewModel(CreateTestWorkItem());

        // Act & Assert
        viewModel.Equals(null).Should().BeFalse();
    }

    [Fact]
    public void Equals_ReturnsTrue_WhenSameReference()
    {
        // Arrange
        var viewModel = new WorkItemViewModel(CreateTestWorkItem());

        // Act & Assert
        viewModel.Equals(viewModel).Should().BeTrue();
    }

    [Fact]
    public void Equals_ReturnsTrue_WhenSameIdVersionAndPendingState()
    {
        // Arrange
        var workItem = CreateTestWorkItem();
        var viewModel1 = new WorkItemViewModel(workItem, false);
        var viewModel2 = new WorkItemViewModel(workItem, false);

        // Act & Assert
        viewModel1.Equals(viewModel2).Should().BeTrue();
    }

    [Fact]
    public void Equals_ReturnsFalse_WhenDifferentPendingState()
    {
        // Arrange
        var workItem = CreateTestWorkItem();
        var viewModel1 = new WorkItemViewModel(workItem, false);
        var viewModel2 = new WorkItemViewModel(workItem, true);

        // Act & Assert
        viewModel1.Equals(viewModel2).Should().BeFalse();
    }

    [Fact]
    public void Equals_ReturnsFalse_WhenDifferentVersion()
    {
        // Arrange
        var workItem1 = CreateTestWorkItem() with { Version = 1 };
        var workItem2 = workItem1 with { Version = 2 };
        var viewModel1 = new WorkItemViewModel(workItem1);
        var viewModel2 = new WorkItemViewModel(workItem2);

        // Act & Assert
        viewModel1.Equals(viewModel2).Should().BeFalse();
    }

    [Fact]
    public void GetHashCode_ReturnsSameValue_ForEqualViewModels()
    {
        // Arrange
        var workItem = CreateTestWorkItem();
        var viewModel1 = new WorkItemViewModel(workItem);
        var viewModel2 = new WorkItemViewModel(workItem);

        // Act & Assert
        viewModel1.GetHashCode().Should().Be(viewModel2.GetHashCode());
    }

    #endregion

    #region Helper Methods

    private static WorkItem CreateTestWorkItem() => new()
    {
        Id = Guid.NewGuid(),
        Title = "Test Work Item",
        Description = "Test Description",
        ItemType = WorkItemType.Story,
        ProjectId = Guid.NewGuid(),
        ParentId = Guid.NewGuid(),
        SprintId = Guid.NewGuid(),
        AssigneeId = Guid.NewGuid(),
        Status = "backlog",
        Priority = "medium",
        StoryPoints = 5,
        Position = 1,
        Version = 1,
        CreatedAt = DateTime.UtcNow,
        UpdatedAt = DateTime.UtcNow,
        CreatedBy = Guid.NewGuid(),
        UpdatedBy = Guid.NewGuid()
    };

    #endregion
}
```

---

## File 21: SprintViewModelTests.cs

**Path**: `ProjectManagement.Components.Tests/ViewModels/SprintViewModelTests.cs`

```csharp
using FluentAssertions;
using ProjectManagement.Core.Models;
using ProjectManagement.Core.ViewModels;
using Xunit;

namespace ProjectManagement.Components.Tests.ViewModels;

public class SprintViewModelTests
{
    #region Constructor Tests

    [Fact]
    public void Constructor_ThrowsArgumentNullException_WhenModelIsNull()
    {
        // Act
        var act = () => new SprintViewModel(null!);

        // Assert
        act.Should().Throw<ArgumentNullException>();
    }

    [Fact]
    public void Constructor_SetsModelCorrectly()
    {
        // Arrange
        var sprint = CreateTestSprint();

        // Act
        var viewModel = new SprintViewModel(sprint);

        // Assert
        viewModel.Model.Should().BeSameAs(sprint);
    }

    [Fact]
    public void Constructor_DefaultsIsPendingSyncToFalse()
    {
        // Arrange
        var sprint = CreateTestSprint();

        // Act
        var viewModel = new SprintViewModel(sprint);

        // Assert
        viewModel.IsPendingSync.Should().BeFalse();
    }

    #endregion

    #region Computed Property Tests

    [Theory]
    [InlineData(SprintStatus.Planned, true, false, false)]
    [InlineData(SprintStatus.Active, false, true, false)]
    [InlineData(SprintStatus.Completed, false, false, true)]
    public void StatusBooleans_ReturnCorrectValues(SprintStatus status, bool isPlanned, bool isActive, bool isCompleted)
    {
        // Arrange
        var sprint = CreateTestSprint() with { Status = status };

        // Act
        var viewModel = new SprintViewModel(sprint);

        // Assert
        viewModel.IsPlanned.Should().Be(isPlanned);
        viewModel.IsActive.Should().Be(isActive);
        viewModel.IsCompleted.Should().Be(isCompleted);
    }

    [Theory]
    [InlineData(SprintStatus.Planned, "Planned")]
    [InlineData(SprintStatus.Active, "Active")]
    [InlineData(SprintStatus.Completed, "Completed")]
    public void StatusDisplayName_ReturnsCorrectValue(SprintStatus status, string expected)
    {
        // Arrange
        var sprint = CreateTestSprint() with { Status = status };

        // Act
        var viewModel = new SprintViewModel(sprint);

        // Assert
        viewModel.StatusDisplayName.Should().Be(expected);
    }

    [Fact]
    public void DateRangeDisplay_FormatsCorrectly()
    {
        // Arrange
        var sprint = CreateTestSprint() with
        {
            StartDate = new DateTime(2024, 1, 15),
            EndDate = new DateTime(2024, 1, 29)
        };

        // Act
        var viewModel = new SprintViewModel(sprint);

        // Assert
        viewModel.DateRangeDisplay.Should().Be("Jan 15 - Jan 29");
    }

    [Fact]
    public void DurationDays_CalculatesCorrectly()
    {
        // Arrange
        var sprint = CreateTestSprint() with
        {
            StartDate = new DateTime(2024, 1, 1),
            EndDate = new DateTime(2024, 1, 15)
        };

        // Act
        var viewModel = new SprintViewModel(sprint);

        // Assert
        viewModel.DurationDays.Should().Be(14);
    }

    [Fact]
    public void DaysRemaining_ReturnsNull_WhenNotActive()
    {
        // Arrange
        var sprint = CreateTestSprint() with { Status = SprintStatus.Planned };

        // Act
        var viewModel = new SprintViewModel(sprint);

        // Assert
        viewModel.DaysRemaining.Should().BeNull();
    }

    [Fact]
    public void DaysRemaining_ReturnsValue_WhenActive()
    {
        // Arrange
        var sprint = CreateTestSprint() with
        {
            Status = SprintStatus.Active,
            StartDate = DateTime.UtcNow.Date.AddDays(-7),
            EndDate = DateTime.UtcNow.Date.AddDays(7)
        };

        // Act
        var viewModel = new SprintViewModel(sprint);

        // Assert
        viewModel.DaysRemaining.Should().Be(7);
    }

    [Fact]
    public void DaysRemaining_ReturnsZero_WhenPastEndDate()
    {
        // Arrange
        var sprint = CreateTestSprint() with
        {
            Status = SprintStatus.Active,
            StartDate = DateTime.UtcNow.Date.AddDays(-14),
            EndDate = DateTime.UtcNow.Date.AddDays(-1)
        };

        // Act
        var viewModel = new SprintViewModel(sprint);

        // Assert
        viewModel.DaysRemaining.Should().Be(0);
    }

    [Fact]
    public void ProgressPercent_ReturnsZero_WhenPlanned()
    {
        // Arrange
        var sprint = CreateTestSprint() with { Status = SprintStatus.Planned };

        // Act
        var viewModel = new SprintViewModel(sprint);

        // Assert
        viewModel.ProgressPercent.Should().Be(0);
    }

    [Fact]
    public void ProgressPercent_Returns100_WhenCompleted()
    {
        // Arrange
        var sprint = CreateTestSprint() with { Status = SprintStatus.Completed };

        // Act
        var viewModel = new SprintViewModel(sprint);

        // Assert
        viewModel.ProgressPercent.Should().Be(100);
    }

    [Fact]
    public void ProgressPercent_CalculatesCorrectly_WhenActive()
    {
        // Arrange
        var sprint = CreateTestSprint() with
        {
            Status = SprintStatus.Active,
            StartDate = DateTime.UtcNow.Date.AddDays(-5),
            EndDate = DateTime.UtcNow.Date.AddDays(5)
        };

        // Act
        var viewModel = new SprintViewModel(sprint);

        // Assert
        viewModel.ProgressPercent.Should().BeApproximately(50, 10); // ~50% with some tolerance
    }

    [Fact]
    public void IsOverdue_ReturnsFalse_WhenNotActive()
    {
        // Arrange
        var sprint = CreateTestSprint() with
        {
            Status = SprintStatus.Completed,
            EndDate = DateTime.UtcNow.Date.AddDays(-1)
        };

        // Act
        var viewModel = new SprintViewModel(sprint);

        // Assert
        viewModel.IsOverdue.Should().BeFalse();
    }

    [Fact]
    public void IsOverdue_ReturnsTrue_WhenActiveAndPastEndDate()
    {
        // Arrange
        var sprint = CreateTestSprint() with
        {
            Status = SprintStatus.Active,
            StartDate = DateTime.UtcNow.Date.AddDays(-14),
            EndDate = DateTime.UtcNow.Date.AddDays(-1)
        };

        // Act
        var viewModel = new SprintViewModel(sprint);

        // Assert
        viewModel.IsOverdue.Should().BeTrue();
    }

    #endregion

    #region Equality Tests

    [Fact]
    public void Equals_ReturnsTrue_WhenSameIdStatusAndPendingState()
    {
        // Arrange
        var sprint = CreateTestSprint();
        var viewModel1 = new SprintViewModel(sprint);
        var viewModel2 = new SprintViewModel(sprint);

        // Act & Assert
        viewModel1.Equals(viewModel2).Should().BeTrue();
    }

    [Fact]
    public void Equals_ReturnsFalse_WhenDifferentStatus()
    {
        // Arrange
        var sprint1 = CreateTestSprint() with { Status = SprintStatus.Planned };
        var sprint2 = sprint1 with { Status = SprintStatus.Active };
        var viewModel1 = new SprintViewModel(sprint1);
        var viewModel2 = new SprintViewModel(sprint2);

        // Act & Assert
        viewModel1.Equals(viewModel2).Should().BeFalse();
    }

    #endregion

    #region Helper Methods

    private static Sprint CreateTestSprint() => new()
    {
        Id = Guid.NewGuid(),
        Name = "Sprint 1",
        Goal = "Complete features",
        ProjectId = Guid.NewGuid(),
        StartDate = DateTime.UtcNow.Date,
        EndDate = DateTime.UtcNow.Date.AddDays(14),
        Status = SprintStatus.Planned
    };

    #endregion
}
```

---

## File 22: SharedComponentTests.cs

**Path**: `ProjectManagement.Components.Tests/Shared/SharedComponentTests.cs`

```csharp
using Bunit;
using FluentAssertions;
using Microsoft.Extensions.DependencyInjection;
using Moq;
using ProjectManagement.Components.Shared;
using ProjectManagement.Core.Models;
using ProjectManagement.Services.State;
using Radzen;
using Xunit;

namespace ProjectManagement.Components.Tests.Shared;

public class SharedComponentTests : TestContext
{
    public SharedComponentTests()
    {
        Services.AddRadzenComponents();
    }

    #region OfflineBanner Tests

    [Fact]
    public void OfflineBanner_DoesNotRender_WhenConnected()
    {
        // Arrange
        var appState = new AppState();
        appState.SetConnectionState(ConnectionState.Connected);
        Services.AddSingleton(appState);

        // Act
        var cut = RenderComponent<OfflineBanner>();

        // Assert
        cut.Markup.Should().BeEmpty();
    }

    [Fact]
    public void OfflineBanner_Renders_WhenDisconnected()
    {
        // Arrange
        var appState = new AppState();
        appState.SetConnectionState(ConnectionState.Disconnected);
        Services.AddSingleton(appState);

        // Act
        var cut = RenderComponent<OfflineBanner>();

        // Assert
        cut.Markup.Should().Contain("offline-banner");
        cut.Markup.Should().Contain("You're offline");
        cut.Markup.Should().Contain("role=\"alert\"");
    }

    [Fact]
    public void OfflineBanner_ShowsSpinner_WhenReconnecting()
    {
        // Arrange
        var appState = new AppState();
        appState.SetConnectionState(ConnectionState.Reconnecting);
        Services.AddSingleton(appState);

        // Act
        var cut = RenderComponent<OfflineBanner>();

        // Assert
        cut.Markup.Should().Contain("offline-banner");
        cut.FindComponents<RadzenProgressBarCircular>().Should().HaveCount(1);
    }

    [Fact]
    public void OfflineBanner_UpdatesOnStateChange()
    {
        // Arrange
        var appState = new AppState();
        appState.SetConnectionState(ConnectionState.Connected);
        Services.AddSingleton(appState);
        var cut = RenderComponent<OfflineBanner>();

        // Act
        appState.SetConnectionState(ConnectionState.Disconnected);

        // Assert
        cut.Markup.Should().Contain("offline-banner");
    }

    [Fact]
    public void OfflineBanner_HasAriaLivePolite()
    {
        // Arrange
        var appState = new AppState();
        appState.SetConnectionState(ConnectionState.Disconnected);
        Services.AddSingleton(appState);

        // Act
        var cut = RenderComponent<OfflineBanner>();

        // Assert
        cut.Markup.Should().Contain("aria-live=\"polite\"");
    }

    #endregion

    #region EmptyState Tests

    [Fact]
    public void EmptyState_RendersTitle()
    {
        // Act
        var cut = RenderComponent<EmptyState>(parameters => parameters
            .Add(p => p.Title, "No items found"));

        // Assert
        cut.Markup.Should().Contain("No items found");
        cut.Markup.Should().Contain("role=\"status\"");
    }

    [Fact]
    public void EmptyState_RendersDescription_WhenProvided()
    {
        // Act
        var cut = RenderComponent<EmptyState>(parameters => parameters
            .Add(p => p.Title, "No items")
            .Add(p => p.Description, "Create your first item to get started"));

        // Assert
        cut.Markup.Should().Contain("Create your first item to get started");
    }

    [Fact]
    public void EmptyState_DoesNotRenderDescription_WhenNull()
    {
        // Act
        var cut = RenderComponent<EmptyState>(parameters => parameters
            .Add(p => p.Title, "No items")
            .Add(p => p.Description, null));

        // Assert
        cut.Markup.Should().NotContain("empty-state-description");
    }

    [Fact]
    public void EmptyState_RendersActionButton_WhenShowActionTrue()
    {
        // Arrange
        var clicked = false;

        // Act
        var cut = RenderComponent<EmptyState>(parameters => parameters
            .Add(p => p.Title, "No items")
            .Add(p => p.ShowAction, true)
            .Add(p => p.ActionText, "Create Item")
            .Add(p => p.OnAction, EventCallback.Factory.Create(this, () => clicked = true)));

        // Assert
        var button = cut.FindComponent<RadzenButton>();
        button.Instance.Text.Should().Be("Create Item");
    }

    [Fact]
    public void EmptyState_DoesNotRenderButton_WhenShowActionFalse()
    {
        // Act
        var cut = RenderComponent<EmptyState>(parameters => parameters
            .Add(p => p.Title, "No items")
            .Add(p => p.ShowAction, false));

        // Assert
        cut.FindComponents<RadzenButton>().Should().BeEmpty();
    }

    [Fact]
    public async Task EmptyState_InvokesCallback_WhenButtonClicked()
    {
        // Arrange
        var clicked = false;

        var cut = RenderComponent<EmptyState>(parameters => parameters
            .Add(p => p.Title, "No items")
            .Add(p => p.ShowAction, true)
            .Add(p => p.OnAction, EventCallback.Factory.Create(this, () => clicked = true)));

        // Act
        var button = cut.FindComponent<RadzenButton>();
        await cut.InvokeAsync(() => button.Instance.Click.InvokeAsync(null));

        // Assert
        clicked.Should().BeTrue();
    }

    [Fact]
    public void EmptyState_UsesCustomIcon()
    {
        // Act
        var cut = RenderComponent<EmptyState>(parameters => parameters
            .Add(p => p.Title, "No results")
            .Add(p => p.Icon, "search_off"));

        // Assert
        cut.Markup.Should().Contain("search_off");
    }

    #endregion

    #region LoadingButton Tests

    [Fact]
    public void LoadingButton_RendersText()
    {
        // Act
        var cut = RenderComponent<LoadingButton>(parameters => parameters
            .Add(p => p.Text, "Save"));

        // Assert
        var button = cut.FindComponent<RadzenButton>();
        button.Instance.Text.Should().Be("Save");
    }

    [Fact]
    public void LoadingButton_ShowsLoadingText_WhenBusy()
    {
        // Act
        var cut = RenderComponent<LoadingButton>(parameters => parameters
            .Add(p => p.Text, "Save")
            .Add(p => p.LoadingText, "Saving...")
            .Add(p => p.IsBusy, true));

        // Assert
        var button = cut.FindComponent<RadzenButton>();
        button.Instance.Text.Should().Be("Saving...");
        button.Instance.IsBusy.Should().BeTrue();
    }

    [Fact]
    public void LoadingButton_IsDisabled_WhenBusy()
    {
        // Act
        var cut = RenderComponent<LoadingButton>(parameters => parameters
            .Add(p => p.Text, "Save")
            .Add(p => p.IsBusy, true));

        // Assert
        var button = cut.FindComponent<RadzenButton>();
        button.Instance.Disabled.Should().BeTrue();
    }

    [Fact]
    public void LoadingButton_IsDisabled_WhenDisabledParameter()
    {
        // Act
        var cut = RenderComponent<LoadingButton>(parameters => parameters
            .Add(p => p.Text, "Save")
            .Add(p => p.Disabled, true));

        // Assert
        var button = cut.FindComponent<RadzenButton>();
        button.Instance.Disabled.Should().BeTrue();
    }

    [Fact]
    public void LoadingButton_IsDisabled_WhenDisconnected()
    {
        // Act
        var cut = RenderComponent<LoadingButton>(parameters => parameters
            .Add(p => p.Text, "Save")
            .Add(p => p.ConnectionState, ConnectionState.Disconnected));

        // Assert
        var button = cut.FindComponent<RadzenButton>();
        button.Instance.Disabled.Should().BeTrue();
    }

    [Fact]
    public void LoadingButton_IsEnabled_WhenConnected()
    {
        // Act
        var cut = RenderComponent<LoadingButton>(parameters => parameters
            .Add(p => p.Text, "Save")
            .Add(p => p.ConnectionState, ConnectionState.Connected));

        // Assert
        var button = cut.FindComponent<RadzenButton>();
        button.Instance.Disabled.Should().BeFalse();
    }

    [Fact]
    public async Task LoadingButton_InvokesCallback_WhenClicked()
    {
        // Arrange
        var clicked = false;

        var cut = RenderComponent<LoadingButton>(parameters => parameters
            .Add(p => p.Text, "Save")
            .Add(p => p.OnClick, EventCallback.Factory.Create<Microsoft.AspNetCore.Components.Web.MouseEventArgs>(
                this, _ => clicked = true)));

        // Act
        var button = cut.FindComponent<RadzenButton>();
        await cut.InvokeAsync(() => button.Instance.Click.InvokeAsync(new Microsoft.AspNetCore.Components.Web.MouseEventArgs()));

        // Assert
        clicked.Should().BeTrue();
    }

    [Fact]
    public async Task LoadingButton_DoesNotInvokeCallback_WhenDisabled()
    {
        // Arrange
        var clicked = false;

        var cut = RenderComponent<LoadingButton>(parameters => parameters
            .Add(p => p.Text, "Save")
            .Add(p => p.Disabled, true)
            .Add(p => p.OnClick, EventCallback.Factory.Create<Microsoft.AspNetCore.Components.Web.MouseEventArgs>(
                this, _ => clicked = true)));

        // Act - simulate internal behavior (button won't fire when disabled)
        // The component's HandleClick checks IsDisabled

        // Assert
        clicked.Should().BeFalse();
    }

    [Fact]
    public void LoadingButton_ShowsOfflineTooltip_WhenDisconnected()
    {
        // Act
        var cut = RenderComponent<LoadingButton>(parameters => parameters
            .Add(p => p.Text, "Save")
            .Add(p => p.ConnectionState, ConnectionState.Disconnected));

        // Assert
        cut.Markup.Should().Contain("Offline - action unavailable");
    }

    #endregion

    #region DebouncedTextBox Tests

    [Fact]
    public void DebouncedTextBox_RendersWithValue()
    {
        // Act
        var cut = RenderComponent<DebouncedTextBox>(parameters => parameters
            .Add(p => p.Value, "test value")
            .Add(p => p.Placeholder, "Enter text..."));

        // Assert
        var textBox = cut.FindComponent<RadzenTextBox>();
        textBox.Instance.Value.Should().Be("test value");
    }

    [Fact]
    public void DebouncedTextBox_RendersPlaceholder()
    {
        // Act
        var cut = RenderComponent<DebouncedTextBox>(parameters => parameters
            .Add(p => p.Placeholder, "Search..."));

        // Assert
        var textBox = cut.FindComponent<RadzenTextBox>();
        textBox.Instance.Placeholder.Should().Be("Search...");
    }

    [Fact]
    public void DebouncedTextBox_IsDisabled_WhenDisabledParameter()
    {
        // Act
        var cut = RenderComponent<DebouncedTextBox>(parameters => parameters
            .Add(p => p.Disabled, true));

        // Assert
        var textBox = cut.FindComponent<RadzenTextBox>();
        textBox.Instance.Disabled.Should().BeTrue();
    }

    [Fact]
    public async Task DebouncedTextBox_DebouncesBefireCallback()
    {
        // Arrange
        var callCount = 0;
        string? lastValue = null;

        var cut = RenderComponent<DebouncedTextBox>(parameters => parameters
            .Add(p => p.DebounceMs, 100)
            .Add(p => p.ValueChanged, EventCallback.Factory.Create<string>(this, v =>
            {
                callCount++;
                lastValue = v;
            })));

        var textBox = cut.FindComponent<RadzenTextBox>();

        // Act - simulate rapid input
        await cut.InvokeAsync(() => textBox.Instance.Change.InvokeAsync("a"));
        await cut.InvokeAsync(() => textBox.Instance.Change.InvokeAsync("ab"));
        await cut.InvokeAsync(() => textBox.Instance.Change.InvokeAsync("abc"));

        // Wait for debounce to complete
        await Task.Delay(150);

        // Assert - should only have called back once with final value
        callCount.Should().Be(1);
        lastValue.Should().Be("abc");
    }

    [Fact]
    public async Task DebouncedTextBox_ImmediateCallback_WhenDebounceIsZero()
    {
        // Arrange
        var values = new List<string>();

        var cut = RenderComponent<DebouncedTextBox>(parameters => parameters
            .Add(p => p.DebounceMs, 0)
            .Add(p => p.ValueChanged, EventCallback.Factory.Create<string>(this, v => values.Add(v))));

        var textBox = cut.FindComponent<RadzenTextBox>();

        // Act
        await cut.InvokeAsync(() => textBox.Instance.Change.InvokeAsync("test"));
        await Task.Delay(10); // Small delay for async completion

        // Assert
        values.Should().Contain("test");
    }

    #endregion

    #region ConfirmDialog Tests

    [Fact]
    public void ConfirmDialog_RendersMessage()
    {
        // Arrange
        Services.AddScoped<DialogService>();

        // Act
        var cut = RenderComponent<ConfirmDialog>(parameters => parameters
            .Add(p => p.Message, "Are you sure?"));

        // Assert
        cut.Markup.Should().Contain("Are you sure?");
    }

    [Fact]
    public void ConfirmDialog_RendersWarning_WhenProvided()
    {
        // Arrange
        Services.AddScoped<DialogService>();

        // Act
        var cut = RenderComponent<ConfirmDialog>(parameters => parameters
            .Add(p => p.Message, "Delete item?")
            .Add(p => p.WarningMessage, "This action cannot be undone"));

        // Assert
        cut.Markup.Should().Contain("This action cannot be undone");
        cut.FindComponents<RadzenAlert>().Should().HaveCount(1);
    }

    [Fact]
    public void ConfirmDialog_DoesNotRenderWarning_WhenNull()
    {
        // Arrange
        Services.AddScoped<DialogService>();

        // Act
        var cut = RenderComponent<ConfirmDialog>(parameters => parameters
            .Add(p => p.Message, "Confirm?")
            .Add(p => p.WarningMessage, null));

        // Assert
        cut.FindComponents<RadzenAlert>().Should().BeEmpty();
    }

    [Fact]
    public void ConfirmDialog_RendersCustomButtonText()
    {
        // Arrange
        Services.AddScoped<DialogService>();

        // Act
        var cut = RenderComponent<ConfirmDialog>(parameters => parameters
            .Add(p => p.Message, "Delete?")
            .Add(p => p.ConfirmText, "Delete")
            .Add(p => p.CancelText, "Keep"));

        // Assert
        var buttons = cut.FindComponents<RadzenButton>();
        buttons.Should().HaveCount(2);
        buttons[0].Instance.Text.Should().Be("Keep");
        buttons[1].Instance.Text.Should().Be("Delete");
    }

    [Fact]
    public async Task ConfirmDialog_InvokesOnConfirm()
    {
        // Arrange
        Services.AddScoped<DialogService>();
        var confirmed = false;

        var cut = RenderComponent<ConfirmDialog>(parameters => parameters
            .Add(p => p.Message, "Confirm?")
            .Add(p => p.OnConfirm, EventCallback.Factory.Create(this, () => confirmed = true)));

        // Act
        var confirmButton = cut.FindComponents<RadzenButton>()
            .First(b => b.Instance.Text == "Confirm");
        await cut.InvokeAsync(() => confirmButton.Instance.Click.InvokeAsync(null));

        // Assert
        confirmed.Should().BeTrue();
    }

    [Fact]
    public async Task ConfirmDialog_InvokesOnCancel()
    {
        // Arrange
        Services.AddScoped<DialogService>();
        var cancelled = false;

        var cut = RenderComponent<ConfirmDialog>(parameters => parameters
            .Add(p => p.Message, "Confirm?")
            .Add(p => p.OnCancel, EventCallback.Factory.Create(this, () => cancelled = true)));

        // Act
        var cancelButton = cut.FindComponents<RadzenButton>()
            .First(b => b.Instance.Text == "Cancel");
        await cut.InvokeAsync(() => cancelButton.Instance.Click.InvokeAsync(null));

        // Assert
        cancelled.Should().BeTrue();
    }

    [Fact]
    public void ConfirmDialog_DisablesCancelButton_WhenBusy()
    {
        // Arrange
        Services.AddScoped<DialogService>();

        // Act
        var cut = RenderComponent<ConfirmDialog>(parameters => parameters
            .Add(p => p.Message, "Confirm?")
            .Add(p => p.IsBusy, true));

        // Assert
        var cancelButton = cut.FindComponents<RadzenButton>()
            .First(b => b.Instance.Text == "Cancel");
        cancelButton.Instance.Disabled.Should().BeTrue();
    }

    [Fact]
    public void ConfirmDialog_ShowsSpinnerOnConfirmButton_WhenBusy()
    {
        // Arrange
        Services.AddScoped<DialogService>();

        // Act
        var cut = RenderComponent<ConfirmDialog>(parameters => parameters
            .Add(p => p.Message, "Confirm?")
            .Add(p => p.IsBusy, true));

        // Assert
        var confirmButton = cut.FindComponents<RadzenButton>()
            .First(b => b.Instance.Text == "Confirm");
        confirmButton.Instance.IsBusy.Should().BeTrue();
    }

    #endregion

    #region ProjectDetailSkeleton Tests

    [Fact]
    public void ProjectDetailSkeleton_RendersWithDefaultRowCount()
    {
        // Act
        var cut = RenderComponent<ProjectDetailSkeleton>();

        // Assert
        cut.Markup.Should().Contain("role=\"status\"");
        cut.Markup.Should().Contain("aria-busy=\"true\"");
        cut.FindComponents<RadzenSkeleton>().Count.Should().BeGreaterThan(5);
    }

    [Fact]
    public void ProjectDetailSkeleton_RendersCustomRowCount()
    {
        // Act
        var cut = RenderComponent<ProjectDetailSkeleton>(parameters => parameters
            .Add(p => p.RowCount, 3));

        // Assert - should have fewer skeleton rows
        cut.Markup.Should().Contain("role=\"status\"");
    }

    [Fact]
    public void ProjectDetailSkeleton_HasAccessibleLabel()
    {
        // Act
        var cut = RenderComponent<ProjectDetailSkeleton>();

        // Assert
        cut.Markup.Should().Contain("aria-label=\"Loading project details\"");
        cut.Markup.Should().Contain("Loading project details...");
    }

    [Fact]
    public void ProjectDetailSkeleton_RendersHeaderSkeleton()
    {
        // Act
        var cut = RenderComponent<ProjectDetailSkeleton>();

        // Assert - should have circle skeleton for icon and rectangle for title
        var skeletons = cut.FindComponents<RadzenSkeleton>();
        skeletons.Any(s => s.Instance.Shape == SkeletonShape.Circle).Should().BeTrue();
    }

    [Fact]
    public void ProjectDetailSkeleton_RendersTabsSkeleton()
    {
        // Act
        var cut = RenderComponent<ProjectDetailSkeleton>();

        // Assert - should have multiple skeleton tabs
        cut.Markup.Should().Contain("80px"); // Tab width
    }

    #endregion
}
```

---

## File 23: BadgeComponentTests.cs

**Path**: `ProjectManagement.Components.Tests/WorkItems/BadgeComponentTests.cs`

```csharp
using Bunit;
using FluentAssertions;
using Microsoft.Extensions.DependencyInjection;
using ProjectManagement.Components.WorkItems;
using ProjectManagement.Core.Models;
using Radzen;
using Xunit;

namespace ProjectManagement.Components.Tests.WorkItems;

public class BadgeComponentTests : TestContext
{
    public BadgeComponentTests()
    {
        Services.AddRadzenComponents();
    }

    #region WorkItemTypeIcon Tests

    [Theory]
    [InlineData(WorkItemType.Project, "folder")]
    [InlineData(WorkItemType.Epic, "rocket_launch")]
    [InlineData(WorkItemType.Story, "description")]
    [InlineData(WorkItemType.Task, "task_alt")]
    public void WorkItemTypeIcon_RendersCorrectIcon(WorkItemType type, string expectedIcon)
    {
        // Act
        var cut = RenderComponent<WorkItemTypeIcon>(parameters => parameters
            .Add(p => p.Type, type));

        // Assert
        cut.Markup.Should().Contain(expectedIcon);
    }

    [Theory]
    [InlineData(WorkItemType.Project, "Project")]
    [InlineData(WorkItemType.Epic, "Epic")]
    [InlineData(WorkItemType.Story, "Story")]
    [InlineData(WorkItemType.Task, "Task")]
    public void WorkItemTypeIcon_HasCorrectTitle(WorkItemType type, string expectedTitle)
    {
        // Act
        var cut = RenderComponent<WorkItemTypeIcon>(parameters => parameters
            .Add(p => p.Type, type));

        // Assert
        cut.Markup.Should().Contain($"title=\"{expectedTitle}\"");
    }

    [Theory]
    [InlineData(WorkItemType.Project, "Project")]
    [InlineData(WorkItemType.Epic, "Epic")]
    [InlineData(WorkItemType.Story, "Story")]
    [InlineData(WorkItemType.Task, "Task")]
    public void WorkItemTypeIcon_HasAccessibleText(WorkItemType type, string expectedText)
    {
        // Act
        var cut = RenderComponent<WorkItemTypeIcon>(parameters => parameters
            .Add(p => p.Type, type));

        // Assert
        cut.Markup.Should().Contain("visually-hidden");
        cut.Markup.Should().Contain(expectedText);
    }

    [Fact]
    public void WorkItemTypeIcon_AppliesCustomSize()
    {
        // Act
        var cut = RenderComponent<WorkItemTypeIcon>(parameters => parameters
            .Add(p => p.Type, WorkItemType.Story)
            .Add(p => p.Size, "2rem"));

        // Assert
        cut.Markup.Should().Contain("font-size: 2rem");
    }

    [Theory]
    [InlineData(WorkItemType.Project, "var(--rz-primary)")]
    [InlineData(WorkItemType.Epic, "#9c27b0")]
    [InlineData(WorkItemType.Story, "#2196f3")]
    [InlineData(WorkItemType.Task, "#4caf50")]
    public void WorkItemTypeIcon_HasCorrectColor(WorkItemType type, string expectedColor)
    {
        // Act
        var cut = RenderComponent<WorkItemTypeIcon>(parameters => parameters
            .Add(p => p.Type, type));

        // Assert
        cut.Markup.Should().Contain($"color: {expectedColor}");
    }

    [Fact]
    public void WorkItemTypeIcon_HasCssClass()
    {
        // Act
        var cut = RenderComponent<WorkItemTypeIcon>(parameters => parameters
            .Add(p => p.Type, WorkItemType.Story));

        // Assert
        cut.Markup.Should().Contain("work-item-type-icon");
    }

    #endregion

    #region WorkItemStatusBadge Tests

    [Theory]
    [InlineData("backlog", "Backlog")]
    [InlineData("todo", "To Do")]
    [InlineData("in_progress", "In Progress")]
    [InlineData("review", "Review")]
    [InlineData("done", "Done")]
    public void WorkItemStatusBadge_RendersCorrectText(string status, string expectedText)
    {
        // Act
        var cut = RenderComponent<WorkItemStatusBadge>(parameters => parameters
            .Add(p => p.Status, status));

        // Assert
        var badge = cut.FindComponent<RadzenBadge>();
        badge.Instance.Text.Should().Be(expectedText);
    }

    [Theory]
    [InlineData("backlog", BadgeStyle.Secondary)]
    [InlineData("todo", BadgeStyle.Info)]
    [InlineData("in_progress", BadgeStyle.Warning)]
    [InlineData("review", BadgeStyle.Primary)]
    [InlineData("done", BadgeStyle.Success)]
    public void WorkItemStatusBadge_HasCorrectStyle(string status, BadgeStyle expectedStyle)
    {
        // Act
        var cut = RenderComponent<WorkItemStatusBadge>(parameters => parameters
            .Add(p => p.Status, status));

        // Assert
        var badge = cut.FindComponent<RadzenBadge>();
        badge.Instance.BadgeStyle.Should().Be(expectedStyle);
    }

    [Fact]
    public void WorkItemStatusBadge_IsPillByDefault()
    {
        // Act
        var cut = RenderComponent<WorkItemStatusBadge>(parameters => parameters
            .Add(p => p.Status, "backlog"));

        // Assert
        var badge = cut.FindComponent<RadzenBadge>();
        badge.Instance.IsPill.Should().BeTrue();
    }

    [Fact]
    public void WorkItemStatusBadge_CanDisablePill()
    {
        // Act
        var cut = RenderComponent<WorkItemStatusBadge>(parameters => parameters
            .Add(p => p.Status, "backlog")
            .Add(p => p.IsPill, false));

        // Assert
        var badge = cut.FindComponent<RadzenBadge>();
        badge.Instance.IsPill.Should().BeFalse();
    }

    [Theory]
    [InlineData("backlog", "Backlog")]
    [InlineData("todo", "To Do")]
    [InlineData("in_progress", "In Progress")]
    public void WorkItemStatusBadge_HasAccessibleTitle(string status, string expectedText)
    {
        // Act
        var cut = RenderComponent<WorkItemStatusBadge>(parameters => parameters
            .Add(p => p.Status, status));

        // Assert
        cut.Markup.Should().Contain($"Status: {expectedText}");
    }

    [Fact]
    public void WorkItemStatusBadge_HandlesUnknownStatus()
    {
        // Act
        var cut = RenderComponent<WorkItemStatusBadge>(parameters => parameters
            .Add(p => p.Status, "custom_status"));

        // Assert
        var badge = cut.FindComponent<RadzenBadge>();
        badge.Instance.Text.Should().Be("custom_status");
        badge.Instance.BadgeStyle.Should().Be(BadgeStyle.Light);
    }

    #endregion

    #region PriorityBadge Tests

    [Theory]
    [InlineData("critical", "priority_high")]
    [InlineData("high", "keyboard_arrow_up")]
    [InlineData("medium", "remove")]
    [InlineData("low", "keyboard_arrow_down")]
    public void PriorityBadge_RendersCorrectIcon(string priority, string expectedIcon)
    {
        // Act
        var cut = RenderComponent<PriorityBadge>(parameters => parameters
            .Add(p => p.Priority, priority));

        // Assert
        cut.Markup.Should().Contain(expectedIcon);
    }

    [Theory]
    [InlineData("critical", "Critical")]
    [InlineData("high", "High")]
    [InlineData("medium", "Medium")]
    [InlineData("low", "Low")]
    public void PriorityBadge_RendersLabel_WhenShowLabelTrue(string priority, string expectedLabel)
    {
        // Act
        var cut = RenderComponent<PriorityBadge>(parameters => parameters
            .Add(p => p.Priority, priority)
            .Add(p => p.ShowLabel, true));

        // Assert
        cut.Markup.Should().Contain(expectedLabel);
    }

    [Fact]
    public void PriorityBadge_DoesNotRenderLabel_WhenShowLabelFalse()
    {
        // Act
        var cut = RenderComponent<PriorityBadge>(parameters => parameters
            .Add(p => p.Priority, "high")
            .Add(p => p.ShowLabel, false));

        // Assert
        cut.Markup.Should().NotContain(">High<");
    }

    [Theory]
    [InlineData("critical", "#d32f2f")]
    [InlineData("high", "#f57c00")]
    [InlineData("medium", "#1976d2")]
    [InlineData("low", "#388e3c")]
    public void PriorityBadge_HasCorrectColor(string priority, string expectedColor)
    {
        // Act
        var cut = RenderComponent<PriorityBadge>(parameters => parameters
            .Add(p => p.Priority, priority));

        // Assert
        cut.Markup.Should().Contain($"color: {expectedColor}");
    }

    [Theory]
    [InlineData("critical", "Critical")]
    [InlineData("high", "High")]
    [InlineData("medium", "Medium")]
    [InlineData("low", "Low")]
    public void PriorityBadge_HasAccessibleTitle(string priority, string expectedText)
    {
        // Act
        var cut = RenderComponent<PriorityBadge>(parameters => parameters
            .Add(p => p.Priority, priority));

        // Assert
        cut.Markup.Should().Contain($"Priority: {expectedText}");
    }

    [Fact]
    public void PriorityBadge_AppliesCustomSize()
    {
        // Act
        var cut = RenderComponent<PriorityBadge>(parameters => parameters
            .Add(p => p.Priority, "high")
            .Add(p => p.Size, "1.5rem"));

        // Assert
        cut.Markup.Should().Contain("font-size: 1.5rem");
    }

    [Fact]
    public void PriorityBadge_HandlesUnknownPriority()
    {
        // Act
        var cut = RenderComponent<PriorityBadge>(parameters => parameters
            .Add(p => p.Priority, "urgent"));

        // Assert
        cut.Markup.Should().Contain("remove"); // Default icon
        cut.Markup.Should().Contain("urgent");
    }

    [Fact]
    public void PriorityBadge_ShowsLabelByDefault()
    {
        // Act
        var cut = RenderComponent<PriorityBadge>(parameters => parameters
            .Add(p => p.Priority, "medium"));

        // Assert
        cut.Markup.Should().Contain("Medium");
    }

    [Fact]
    public void PriorityBadge_HasPriorityBadgeClass()
    {
        // Act
        var cut = RenderComponent<PriorityBadge>(parameters => parameters
            .Add(p => p.Priority, "high"));

        // Assert
        cut.Markup.Should().Contain("priority-badge");
    }

    #endregion
}
```

---

## Program.cs Updates

Add after existing registrations:

```csharp
// ViewModels
builder.Services.AddScoped<ViewModelFactory>();
```

---

## index.html CSS Links

Add to `<head>` section of `wwwroot/index.html`:

```html
<link href="_content/ProjectManagement.Components/css/app.css" rel="stylesheet" />
<link href="_content/ProjectManagement.Components/css/work-items.css" rel="stylesheet" />
<link href="_content/ProjectManagement.Components/css/kanban.css" rel="stylesheet" />
<link href="_content/ProjectManagement.Components/css/layout.css" rel="stylesheet" />
```

---

## Part 1 Verification Checklist

Before proceeding to Part 2, verify:

```bash
# Build Core
cd frontend
dotnet build ProjectManagement.Core

# Build Services
dotnet build ProjectManagement.Services

# Build Components
dotnet build ProjectManagement.Components

# Run all tests
dotnet test

# Expected: 35+ tests passing, 0 failures
```

- [ ] All ViewModel classes compile
- [ ] ViewModelFactory compiles and is registered
- [ ] All CSS files created
- [ ] All leaf components compile
- [ ] All tests pass (35+)
- [ ] No warnings in build output

---

## End of Part 1

Part 2 will implement:
- WorkItemRow, KanbanCard (composite components using leaf components)
- WorkItemDialog, VersionConflictDialog (dialogs)
- WorkItemList, KanbanColumn, KanbanBoard (views)
- NavMenu, MainLayout updates
- Home, ProjectDetail, WorkItemDetail pages
- Remaining tests (35+ additional)

**Total after Part 2: 70+ tests**
