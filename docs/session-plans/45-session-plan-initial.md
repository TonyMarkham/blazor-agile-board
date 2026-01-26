# Session 45: Kanban Drag-and-Drop Fix + Type-Specific Cards

## Problems
1. HTML5 drag-and-drop events (`ondragover`, `ondrop`) don't fire reliably in Blazor WASM, breaking card drops between columns.
2. All work item types (Epic/Story/Task) render identically - no visual hierarchy feedback.

## Solutions
1. Replace custom HTML5 drag-and-drop with Radzen's `RadzenDropZoneContainer<TItem>` and `RadzenDropZone<TItem>` components.
2. Add type-specific card rendering: Epics/Stories show child progress bars, Tasks remain simple.

---

## Critical Design Constraints

1. **No JavaScript** - All functionality must be pure Blazor/C#. JS interop is forbidden except where absolutely required by WASM runtime. If a feature cannot be implemented without custom JS, the feature is descoped or an alternative approach is found. JS is a direction of absolute last resort.

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

## Critical Architecture Notes (from Radzen source analysis)

**RadzenDropZone DOM structure:**
```
<div class="rz-dropzone">       <!-- RadzenDropZone wrapper -->
  ├── ChildContent              <!-- Renders first (header, empty state) -->
  ├── Items (foreach)           <!-- Items render AFTER ChildContent -->
  └── Footer                    <!-- Optional footer -->
</div>
```

**Key insight:** Items render as siblings AFTER ChildContent, not inside a container. This means:
- Column header must be in ChildContent
- RadzenDropZone itself becomes the "column body"
- Empty state goes in ChildContent (visible when no items)

**Draggability control:** Use `args.Attributes["draggable"] = "false"` (no dedicated `Draggable` property)

---

## Files Summary

### Files to Create (2 files)

| File | Purpose |
|------|---------|
| `frontend/.../Core/ViewModels/ChildProgress.cs` | Progress tracking model for Epic/Story cards |
| `frontend/.../Components/WorkItems/ChildProgressBar.razor` | Reusable swimlane progress bar component |

### Files to Modify (8 files)

| File | Changes |
|------|---------|
| `frontend/.../WorkItems/KanbanBoard.razor` | Add `RadzenDropZoneContainer`, new drop handler |
| `frontend/.../WorkItems/KanbanColumn.razor` | Restructure: header outside DropZone, DropZone = body |
| `frontend/.../WorkItems/KanbanCard.razor` | Remove HTML5 drag, add conditional progress section |
| `frontend/.../wwwroot/css/kanban.css` | Add Radzen styles + progress bar styles |
| `frontend/.../Core/ViewModels/WorkItemViewModel.cs` | Add `ChildProgress` computed property |
| `frontend/.../Services/ViewModelFactory.cs` | Compute child progress from AppState cache |
| `frontend/.../Tests/.../KanbanCardTests.cs` | Delete 6 drag tests, add progress bar tests |
| `frontend/.../Tests/.../KanbanBoardTests.cs` | Update 1 test (parameter rename) |

---

## Implementation Steps (Incremental Builds)

**Dependency order:** Child params must exist before parent passes them. Parent must stop passing before child removes params.

### Step 1: Add CSS for Radzen drag states
Add to `kanban.css`:
```css
/* === Radzen DropZone Integration === */

/* DropZone as column body */
.kanban-column > .rz-dropzone {
    flex: 1;
    overflow-y: auto;
    padding: 0.5rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
}

/* Drop target highlight (Radzen applies .rz-can-drop to the zone) */
.rz-dropzone.rz-can-drop {
    box-shadow: 0 0 0 2px var(--rz-primary);
    background: var(--rz-primary-lighter);
}

/* Dragging card style */
.rz-drag-source .kanban-card,
.kanban-card.rz-drag-source {
    cursor: grabbing;
    transform: rotate(2deg);
    box-shadow: var(--rz-shadow-3);
    opacity: 0.9;
}

/* Keyboard drag overlay - captures mouse clicks to cancel keyboard drag */
.keyboard-drag-overlay {
    position: absolute;
    inset: 0;
    z-index: 100;
    background: transparent;
    cursor: default;
}
```
**Build:** `just build-cs-components`

### Step 2: Add CardTemplate infrastructure

**KanbanColumn.razor** - Add parameter (don't use yet):
```csharp
[Parameter]
public RenderFragment<WorkItemViewModel>? CardTemplate { get; set; }
```

**KanbanBoard.razor** - Add methods and pass CardTemplate:
```csharp
// CardTemplate - RenderFragment with builder pattern
// Note: In @code block, use __builder pattern for type safety
private RenderFragment<WorkItemViewModel> CardTemplate => item => __builder =>
{
    __builder.OpenComponent<KanbanCard>(0);
    __builder.AddAttribute(1, "Item", item);
    __builder.AddAttribute(2, "IsConnected", _isConnected);
    __builder.AddAttribute(3, "OnClick", HandleCardClick);
    __builder.AddAttribute(4, "OnEdit", HandleCardEdit);
    __builder.CloseComponent();
};

private bool ItemBelongsToZone(WorkItemViewModel item, RadzenDropZone<WorkItemViewModel> zone)
    => item.Status == zone.Value?.ToString();

// CANONICAL HandleItemRender - use this version everywhere
private void HandleItemRender(RadzenDropZoneItemRenderEventArgs<WorkItemViewModel> args)
{
    if (args?.Item is null) return;

    args.Attributes ??= new Dictionary<string, object>();

    // Always add aria-label for accessibility
    args.Attributes["aria-label"] = $"Drag {args.Item.Title} to move between columns";

    // Disable drag when not connected or syncing
    if (!_isConnected || args.Item.IsPendingSync)
    {
        args.Attributes["draggable"] = "false";
        args.Attributes["aria-disabled"] = "true";
    }
}

private async Task HandleRadzenDrop(RadzenDropZoneItemEventArgs<WorkItemViewModel> args)
{
    // (full implementation in Step 4)
}
```

Update KanbanColumn calls to pass CardTemplate:
```razor
<KanbanColumn ... CardTemplate="@CardTemplate" ... />
```

**Build:** `just build-cs-components`

### Step 3: Add RadzenDropZoneContainer to KanbanBoard

**Why this step comes before Column restructure:** `RadzenDropZone` requires a `RadzenDropZoneContainer` parent to function. We add the Container first so that when we restructure Column in Step 4, the DropZone has a parent.

Wrap the columns div and add keyboard drag overlay:
```razor
<RadzenDropZoneContainer TItem="WorkItemViewModel"
                         Data="@_filteredItems"
                         ItemSelector="@ItemBelongsToZone"
                         Drop="@HandleRadzenDrop"
                         ItemRender="@HandleItemRender">
    <div class="kanban-columns" role="listbox" aria-orientation="horizontal" style="position: relative;">
        @* Overlay captures mouse clicks during keyboard drag *@
        @if (_draggedItem is not null)
        {
            <div class="keyboard-drag-overlay"
                 @onclick="CancelKeyboardDrag"
                 @onclick:stopPropagation="true">
            </div>
        }
        @foreach (var column in Columns) { ... }
    </div>
</RadzenDropZoneContainer>
```

Add cancel method:
```csharp
private void CancelKeyboardDrag()
{
    _announcement = "Drag cancelled.";
    HandleDragEnd();
}
```

Add full `HandleRadzenDrop` implementation:
```csharp
private async Task HandleRadzenDrop(RadzenDropZoneItemEventArgs<WorkItemViewModel> args)
{
    if (args?.Item is null || args.DropZone?.Value is null || !_isConnected)
        return;

    var item = args.Item;
    var newStatus = args.DropZone.Value.ToString()!;
    var oldStatus = item.Status;

    if (oldStatus == newStatus)
    {
        _announcement = $"{item.Title} returned to {GetColumnTitle(newStatus)}.";
        return;
    }

    _announcement = $"Moving {item.Title} to {GetColumnTitle(newStatus)}...";
    await InvokeAsync(StateHasChanged);

    try
    {
        var request = new UpdateWorkItemRequest
        {
            WorkItemId = item.Id,
            ExpectedVersion = item.Version,
            Status = newStatus
        };
        await AppState.WorkItems.UpdateAsync(request);

        _announcement = $"{item.Title} moved to {GetColumnTitle(newStatus)}.";
        NotificationService.Notify(NotificationSeverity.Success, "Moved", $"Moved to {GetColumnTitle(newStatus)}");
    }
    catch (Exception ex)
    {
        _announcement = $"Failed to move {item.Title}: {ex.Message}";
        NotificationService.Notify(NotificationSeverity.Error, "Error", ex.Message);
    }
}

private string GetColumnTitle(string status) =>
    Columns.FirstOrDefault(c => c.Status == status).Title ?? status;
```

**Note:** At this point, the board still renders cards via the old foreach loop in KanbanColumn. The Container is ready but Column hasn't switched to DropZone yet.

**Build:** `just build-cs-components`

### Step 4: Restructure KanbanColumn with RadzenDropZone

**Now that Container exists**, Column can use RadzenDropZone.

Replace the body (remove foreach loop, add RadzenDropZone):
```razor
<div class="kanban-column @(IsDragTarget ? "drag-target" : "")"
     role="listbox"
     aria-label="@($"{Title} column, {ItemCount} items")">

    <!-- Header stays outside -->
    <div class="kanban-column-header">
        <RadzenText TextStyle="TextStyle.Subtitle1" class="m-0">@Title</RadzenText>
        <RadzenBadge BadgeStyle="@HeaderBadgeStyle" Text="@ItemCount.ToString()" IsPill="true" />
    </div>

    <!-- DropZone replaces the body -->
    <RadzenDropZone TItem="WorkItemViewModel" Value="@Status" Template="@CardTemplate">
        @if (ItemCount == 0)
        {
            <div class="kanban-empty-column">
                <RadzenIcon Icon="inbox" Style="font-size: 1.5rem; opacity: 0.5;" />
                <RadzenText TextStyle="TextStyle.Caption" class="text-muted mt-2">No items</RadzenText>
            </div>
        }
    </RadzenDropZone>
</div>
```

Remove from div: `@ondragover`, `@ondragover:preventDefault`, `@ondrop`
Remove: `HandleDragOver()` and `HandleDrop()` methods (in Column's @code block)
Remove: the `foreach` loop (Radzen renders items via Template now)

**Note:** Keep all parameters for now (OnDragStart, etc.) - they're unused but Board still passes them.

**Build:** `just build-cs-components`

**Test:** Run `just dev` - drag and drop should now work via Radzen!

### Step 5: Remove drag callbacks (coordinated change)

**KanbanBoard.razor** - Stop passing callbacks, rename param:
```razor
<KanbanColumn Status="@column.Status"
              Title="@column.Title"
              Items="@GetColumnItems(column.Status)"
              IsConnected="@_isConnected"
              IsKeyboardDragTarget="@(_draggedItem is not null && _dragTargetColumn == column.Status)"
              CardTemplate="@CardTemplate"
              OnCardClick="@HandleCardClick"
              OnCardEdit="@HandleCardEdit" />
@* REMOVED: OnDragStart, OnDragEnd, OnDragEnter, OnDrop *@
@* RENAMED: IsDragTarget → IsKeyboardDragTarget *@
```

**KanbanColumn.razor** - Remove params, rename:
```csharp
// REMOVE these:
// [Parameter] public EventCallback<WorkItemViewModel> OnDragStart { get; set; }
// [Parameter] public EventCallback OnDragEnd { get; set; }
// [Parameter] public EventCallback OnDragEnter { get; set; }
// [Parameter] public EventCallback<string> OnDrop { get; set; }

// RENAME:
[Parameter]
public bool IsKeyboardDragTarget { get; set; }  // was IsDragTarget
```

Update div class: `@(IsKeyboardDragTarget ? "drag-target" : "")`

**Build:** `just build-cs-components`

### Step 6: Simplify KanbanCard.razor

Remove parameters:
```csharp
// REMOVE:
// [Parameter] public EventCallback<WorkItemViewModel> OnDragStart { get; set; }
// [Parameter] public EventCallback OnDragEnd { get; set; }
```

Remove internal drag code:
- `private bool _isDragging` field
- `HandleDragStart(DragEventArgs)` method
- `HandleDragEnd(DragEventArgs)` method

Remove from div:
- `draggable="..."` attribute
- `@ondragstart="HandleDragStart"`
- `@ondragend="HandleDragEnd"`
- `aria-grabbed="..."`

Simplify:
- `CardCssClass` - remove `_isDragging` check (just return `Item.IsPendingSync ? "pending-sync" : ""`)
- `AriaLabel` - remove "Drag to move" hint

Remove keyboard drag handling (Space to pick up/drop) in `HandleKeyDown`.

**Build:** `just build-cs-components`

### Step 7: Update Tests

**KanbanCardTests.cs** - Remove 6 tests in `#region Drag Events Tests`:

| Test Name | Action |
|-----------|--------|
| `KanbanCard_IsDraggable_WhenConnectedAndNotPending` | **DELETE** - no `draggable` attribute |
| `KanbanCard_IsNotDraggable_WhenDisconnected` | **DELETE** - no `draggable` attribute |
| `KanbanCard_IsNotDraggable_WhenPendingSync` | **DELETE** - no `draggable` attribute |
| `KanbanCard_InvokesOnDragStart_WhenDragStarts` | **DELETE** - no `OnDragStart` callback |
| `KanbanCard_InvokesOnDragEnd_WhenDragEnds` | **DELETE** - no `OnDragEnd` callback |
| `KanbanCard_HasAriaGrabbedFalse_WhenNotDragging` | **DELETE** - no `aria-grabbed` attribute |

Delete entire `#region Drag Events Tests` section (lines 120-224).

**KanbanBoardTests.cs** - Update 1 test:

| Test Name | Action |
|-----------|--------|
| `KanbanColumn_AppliesDragTargetClass_WhenIsDragTarget` | **UPDATE** - rename param `IsDragTarget` → `IsKeyboardDragTarget` |

**Build:** `just build-cs-components && just test-cs-components`

---

## Part 2: Type-Specific Cards (Steps 8-13)

**Dependency:** Steps 1-7 must be complete. Drag-and-drop should work before adding card enhancements.

### Step 8: Create ChildProgress Model

**Create**: `frontend/ProjectManagement.Core/ViewModels/ChildProgress.cs`

```csharp
namespace ProjectManagement.Core.ViewModels;

/// <summary>
/// Tracks child item progress for Epic and Story cards.
/// Computed from AppState cache, not fetched separately.
/// </summary>
public record ChildProgress
{
    /// <summary>
    /// Count of child items by status (e.g., "todo" -> 3, "done" -> 5)
    /// </summary>
    public IReadOnlyDictionary<string, int> ByStatus { get; init; } = new Dictionary<string, int>();

    /// <summary>
    /// Total number of child items
    /// </summary>
    public int Total { get; init; }

    /// <summary>
    /// Number of completed items (status = "done")
    /// </summary>
    public int Completed { get; init; }

    /// <summary>
    /// Progress percentage (0-100)
    /// </summary>
    public int Percentage => Total > 0 ? (Completed * 100) / Total : 0;

    /// <summary>
    /// Display string like "3/12"
    /// </summary>
    public string DisplayText => $"{Completed}/{Total}";

    /// <summary>
    /// True if there are any child items to show progress for
    /// </summary>
    public bool HasChildren => Total > 0;

    /// <summary>
    /// Empty progress (no children)
    /// </summary>
    public static ChildProgress Empty { get; } = new();
}
```

**Build:** `just build-cs-core`

### Step 9: Update WorkItemViewModel

**File**: `frontend/ProjectManagement.Core/ViewModels/WorkItemViewModel.cs`

Add the `ChildProgress` properties:

```csharp
// Add to WorkItemViewModel class

/// <summary>
/// Progress of child tasks (for Story cards)
/// </summary>
public ChildProgress? TaskProgress { get; init; }

/// <summary>
/// Progress of child stories (for Epic cards)
/// </summary>
public ChildProgress? StoryProgress { get; init; }

/// <summary>
/// True if this card should show progress bars (Epic or Story with children)
/// </summary>
public bool ShowProgress => ItemType switch
{
    WorkItemType.Epic => StoryProgress?.HasChildren == true || TaskProgress?.HasChildren == true,
    WorkItemType.Story => TaskProgress?.HasChildren == true,
    _ => false
};
```

**Build:** `just build-cs-core`

### Step 10: Update ViewModelFactory to Compute Progress

**File**: `frontend/ProjectManagement.Services/State/ViewModelFactory.cs`

Update the factory to compute child progress from the AppState cache:

```csharp
// Add using
using System.Linq;

// Update Create method or add overload
public WorkItemViewModel Create(WorkItem workItem, IEnumerable<WorkItem>? allProjectItems = null)
{
    ChildProgress? taskProgress = null;
    ChildProgress? storyProgress = null;

    if (allProjectItems is not null)
    {
        var children = allProjectItems
            .Where(w => w.ParentId == workItem.Id && w.DeletedAt is null)
            .ToList();

        if (workItem.ItemType == WorkItemType.Epic)
        {
            // Epic: track both stories and direct tasks
            var stories = children.Where(c => c.ItemType == WorkItemType.Story).ToList();
            var tasks = children.Where(c => c.ItemType == WorkItemType.Task).ToList();

            // Also count tasks under stories (grandchildren)
            var storyIds = stories.Select(s => s.Id).ToHashSet();
            var grandchildTasks = allProjectItems
                .Where(w => w.ParentId.HasValue &&
                            storyIds.Contains(w.ParentId.Value) &&
                            w.ItemType == WorkItemType.Task &&
                            w.DeletedAt is null)
                .ToList();

            var allTasks = tasks.Concat(grandchildTasks).ToList();

            storyProgress = ComputeProgress(stories);
            taskProgress = ComputeProgress(allTasks);
        }
        else if (workItem.ItemType == WorkItemType.Story)
        {
            // Story: track child tasks only
            var tasks = children.Where(c => c.ItemType == WorkItemType.Task).ToList();
            taskProgress = ComputeProgress(tasks);
        }
    }

    return new WorkItemViewModel(workItem, isPendingSync: false)
    {
        TaskProgress = taskProgress,
        StoryProgress = storyProgress
    };
}

private static ChildProgress ComputeProgress(IReadOnlyList<WorkItem> items)
{
    if (items.Count == 0) return ChildProgress.Empty;

    var byStatus = items
        .GroupBy(w => w.Status)
        .ToDictionary(g => g.Key, g => g.Count());

    var completed = byStatus.GetValueOrDefault("done", 0);

    return new ChildProgress
    {
        ByStatus = byStatus,
        Total = items.Count,
        Completed = completed
    };
}

// Update CreateMany to pass all items
public IEnumerable<WorkItemViewModel> CreateMany(IEnumerable<WorkItem> items)
{
    var itemList = items.ToList();
    return itemList.Select(item => Create(item, itemList));
}
```

**Build:** `just build-cs-services`

### Step 11: Create ChildProgressBar Component

**Create**: `frontend/ProjectManagement.Components/WorkItems/ChildProgressBar.razor`

```razor
@using ProjectManagement.Core.ViewModels

@if (Progress?.HasChildren == true)
{
    <div class="child-progress-bar" title="@TooltipText">
        <div class="progress-track">
            @foreach (var segment in GetSegments())
            {
                <div class="progress-segment @segment.StatusClass"
                     style="width: @segment.WidthPercent%"
                     title="@segment.Tooltip">
                </div>
            }
        </div>
        <span class="progress-label">@Label @Progress.DisplayText</span>
    </div>
}

@code {
    [Parameter, EditorRequired]
    public ChildProgress Progress { get; set; } = ChildProgress.Empty;

    [Parameter]
    public string Label { get; set; } = "";

    private string TooltipText => $"{Label}: {Progress.Completed} of {Progress.Total} complete ({Progress.Percentage}%)";

    // Status display order (left to right on progress bar)
    private static readonly string[] StatusOrder = { "done", "review", "in_progress", "todo", "backlog" };

    private IEnumerable<ProgressSegment> GetSegments()
    {
        if (Progress.Total == 0) yield break;

        foreach (var status in StatusOrder)
        {
            if (Progress.ByStatus.TryGetValue(status, out var count) && count > 0)
            {
                yield return new ProgressSegment
                {
                    StatusClass = $"status-{status.Replace("_", "-")}",
                    WidthPercent = (count * 100.0) / Progress.Total,
                    Tooltip = $"{StatusDisplayName(status)}: {count}"
                };
            }
        }
    }

    private static string StatusDisplayName(string status) => status switch
    {
        "backlog" => "Backlog",
        "todo" => "To Do",
        "in_progress" => "In Progress",
        "review" => "Review",
        "done" => "Done",
        _ => status
    };

    private record ProgressSegment
    {
        public string StatusClass { get; init; } = "";
        public double WidthPercent { get; init; }
        public string Tooltip { get; init; } = "";
    }
}
```

**Build:** `just build-cs-components`

### Step 12: Add Progress Bar CSS

**File**: `frontend/ProjectManagement.Components/wwwroot/css/kanban.css`

Add at the end (after Radzen DropZone styles from Step 1):

```css
/* =============================================================================
   CHILD PROGRESS BAR (for Epic/Story cards)
   ============================================================================= */

.child-progress-bar {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-top: 0.25rem;
}

.progress-track {
    flex: 1;
    height: 6px;
    background: var(--rz-base-300);
    border-radius: 3px;
    overflow: hidden;
    display: flex;
}

.progress-segment {
    height: 100%;
    transition: width 0.3s ease;
}

/* Status colors - match Kanban column semantics */
.progress-segment.status-done {
    background: var(--rz-success);
}

.progress-segment.status-review {
    background: var(--rz-secondary);
}

.progress-segment.status-in-progress {
    background: var(--rz-warning);
}

.progress-segment.status-todo {
    background: var(--rz-info);
}

.progress-segment.status-backlog {
    background: var(--rz-base-400);
}

.progress-label {
    font-size: 0.7rem;
    color: var(--rz-text-secondary-color);
    white-space: nowrap;
    min-width: 3rem;
}

/* Compact progress for cards */
.kanban-card .child-progress-bar {
    margin-top: 0.5rem;
    padding-top: 0.5rem;
    border-top: 1px solid var(--rz-border-color);
}

/* Reduced motion */
@media (prefers-reduced-motion: reduce) {
    .progress-segment {
        transition: none;
    }
}
```

**Build:** `just build-cs-components`

### Step 13: Update KanbanCard with Conditional Progress

**File**: `frontend/ProjectManagement.Components/WorkItems/KanbanCard.razor`

Add the progress section after the footer (before closing `</RadzenStack>`):

```razor
@* Add after the footer RadzenStack, before closing </RadzenStack> *@

@* Progress section for Epic/Story cards *@
@if (Item.ShowProgress)
{
    @if (Item.ItemType == WorkItemType.Epic)
    {
        @if (Item.StoryProgress?.HasChildren == true)
        {
            <ChildProgressBar Progress="@Item.StoryProgress" Label="Stories" />
        }
        @if (Item.TaskProgress?.HasChildren == true)
        {
            <ChildProgressBar Progress="@Item.TaskProgress" Label="Tasks" />
        }
    }
    else if (Item.ItemType == WorkItemType.Story)
    {
        @if (Item.TaskProgress?.HasChildren == true)
        {
            <ChildProgressBar Progress="@Item.TaskProgress" Label="Tasks" />
        }
    }
}
```

**Build:** `just build-cs-components`

### Step 14: Add Progress Bar Tests

**File**: `frontend/ProjectManagement.Components.Tests/WorkItems/KanbanCardTests.cs`

Add new test region after existing tests:

```csharp
#region Progress Bar Tests

[Fact]
public void KanbanCard_ShowsTaskProgress_ForStoryWithChildren()
{
    // Arrange
    var workItem = CreateTestWorkItem() with { ItemType = WorkItemType.Story };
    var progress = new ChildProgress
    {
        ByStatus = new Dictionary<string, int> { ["done"] = 2, ["todo"] = 3 },
        Total = 5,
        Completed = 2
    };
    var viewModel = new WorkItemViewModel(workItem) { TaskProgress = progress };

    // Act
    var cut = Render<KanbanCard>(parameters => parameters
        .Add(p => p.Item, viewModel));

    // Assert
    cut.FindComponents<ChildProgressBar>().Should().HaveCount(1);
    cut.Markup.Should().Contain("2/5");
}

[Fact]
public void KanbanCard_ShowsBothProgressBars_ForEpicWithStoriesAndTasks()
{
    // Arrange
    var workItem = CreateTestWorkItem() with { ItemType = WorkItemType.Epic };
    var storyProgress = new ChildProgress
    {
        ByStatus = new Dictionary<string, int> { ["done"] = 1, ["in_progress"] = 2 },
        Total = 3,
        Completed = 1
    };
    var taskProgress = new ChildProgress
    {
        ByStatus = new Dictionary<string, int> { ["done"] = 5, ["todo"] = 5 },
        Total = 10,
        Completed = 5
    };
    var viewModel = new WorkItemViewModel(workItem)
    {
        StoryProgress = storyProgress,
        TaskProgress = taskProgress
    };

    // Act
    var cut = Render<KanbanCard>(parameters => parameters
        .Add(p => p.Item, viewModel));

    // Assert
    cut.FindComponents<ChildProgressBar>().Should().HaveCount(2);
    cut.Markup.Should().Contain("Stories");
    cut.Markup.Should().Contain("Tasks");
}

[Fact]
public void KanbanCard_HidesProgressBar_ForTaskItems()
{
    // Arrange
    var workItem = CreateTestWorkItem() with { ItemType = WorkItemType.Task };
    var viewModel = new WorkItemViewModel(workItem);

    // Act
    var cut = Render<KanbanCard>(parameters => parameters
        .Add(p => p.Item, viewModel));

    // Assert
    cut.FindComponents<ChildProgressBar>().Should().BeEmpty();
}

[Fact]
public void KanbanCard_HidesProgressBar_ForStoryWithNoChildren()
{
    // Arrange
    var workItem = CreateTestWorkItem() with { ItemType = WorkItemType.Story };
    var viewModel = new WorkItemViewModel(workItem) { TaskProgress = ChildProgress.Empty };

    // Act
    var cut = Render<KanbanCard>(parameters => parameters
        .Add(p => p.Item, viewModel));

    // Assert
    cut.FindComponents<ChildProgressBar>().Should().BeEmpty();
}

#endregion
```

**Build:** `just build-cs-components && just test-cs-components`

---

## Verification Checklist

### Build & Tests
1. `just build-cs-components` - compiles without errors
2. `just test-cs-components` - all tests pass (after test updates)

### Mouse Drag & Drop
3. Drag card from Backlog → To Do → card moves, notification shows
4. Drag card from To Do → In Progress → card moves
5. Drag card to Done → card moves
6. Drop in same column → no backend call, no notification
7. Visual feedback: column highlights on hover during drag

### Keyboard Navigation
8. Tab to card → press Space → card is "picked up" (announcement plays)
9. Arrow keys → target column changes (announcement plays)
10. Space again → card drops to target column
11. Escape → drag cancelled, card returns to original position

### Edge Cases
12. Pending sync items (spinner visible) → cannot drag (mouse or keyboard)
13. Disconnected state → cannot drag, buttons disabled
14. Rapid drag operations → no race conditions

### Accessibility
15. Screen reader announces drag start, column changes, and drop result
16. Live region updates (`aria-live="polite"`) work correctly
17. Focus management preserved after drop

### Type-Specific Cards (Steps 8-14)
18. Task cards show NO progress bar
19. Story cards show task progress bar with swimlane colors
20. Epic cards show BOTH story progress AND task progress bars
21. Progress bar segments colored by status (done=green, review=purple, etc.)
22. Hover on progress bar shows tooltip with counts
23. Creating a child task under a Story → Story's progress bar updates
24. Moving a task to "done" → parent Story/Epic progress updates

---

## Key Design Decisions

### Drag-and-Drop (Steps 1-7)

1. **Header outside, DropZone = body** - Column header renders outside DropZone; the DropZone itself becomes the scrollable body containing cards
2. **KanbanCard via Template** - Rendered by Radzen's Template parameter, preserving all card UI (click, edit, accessibility)
3. **Hybrid keyboard support** - Mouse drag via Radzen, keyboard drag via existing custom code (`_draggedItem` state preserved)
4. **Draggability via Attributes** - Use `args.Attributes["draggable"] = "false"` in ItemRender (no dedicated property)
5. **Empty state via conditional render** - Check `Items.Any()` and only render "No items" when empty (simpler than CSS hacks)

### Type-Specific Cards (Steps 8-14)

6. **Single KanbanCard component (not interfaces)** - Keep one `KanbanCard.razor` with conditional `@if` blocks rather than `IKanbanCard` interface + multiple components
   - **Rationale:** 80% of card code is shared (click, edit, priority, accessibility); only 20% differs (progress bar presence); Radzen DropZone Template expects uniform type; simpler mental model

7. **ChildProgress computed from AppState cache** - `WorkItemViewModel.TaskProgress` and `StoryProgress` are computed by `ViewModelFactory` from existing cached data
   - **Rationale:** No extra backend queries; all project work items already loaded; reactive (updates when cache changes); consistent with local-first architecture

8. **Swimlane progress bar** - Progress bar shows colored segments for each status, not just a single "percent complete"
   - **Rationale:** Visual feedback about WHERE work is stuck (e.g., lots of items in "review" = bottleneck); matches Kanban philosophy of flow visualization

9. **Epic tracks grandchildren** - Epic's task progress includes tasks under its Stories, not just direct children
   - **Rationale:** Reflects true completion state; users want to know "how much of this Epic is done" including all nested work

---

## Keyboard + Radzen State Handling

**Problem:** The existing keyboard navigation uses `_draggedItem` state while Radzen manages its own internal drag state. These must not conflict.

**Solution:**
- **Mouse drag:** Radzen handles entirely. `_draggedItem` stays null. `HandleRadzenDrop` processes drops.
- **Keyboard drag:** User presses Space on a card → `HandleDragStart` sets `_draggedItem` → Arrow keys call `HandleDragEnter` → Space calls `HandleDrop` (existing method, not Radzen) → Escape calls `HandleDragEnd`

**Key insight:** The keyboard flow never touches Radzen's Drop callback because:
1. Keyboard drag is initiated by our `HandleDragStart`, not Radzen
2. Keyboard drop calls our `HandleDrop(status)` directly, not `HandleRadzenDrop`

**Code clarification** - Keep BOTH drop handlers:
```csharp
// For MOUSE drag (Radzen)
private async Task HandleRadzenDrop(RadzenDropZoneItemEventArgs<WorkItemViewModel> args) { ... }

// For KEYBOARD drag (existing)
private async Task HandleDrop(string newStatus) { ... }  // Keep this!
```

The keyboard handler (`HandleBoardKeyDown`) continues to call `HandleDrop(string)` for keyboard drops.

---

## Error Handling

**All error handling is built into the canonical `HandleItemRender` and `HandleRadzenDrop` methods shown in Steps 2 and 4.**

Key defensive patterns:
- `if (args?.Item is null) return;` - Guard against null args
- `args.Attributes ??= new Dictionary<string, object>();` - Initialize if needed
- `if (args?.DropZone?.Value is null || !_isConnected) return;` - Validate drop target

---

## Accessibility

**Radzen's limitation:** RadzenDropZone does NOT provide screen reader announcements during drag.

**Our solution:**
1. `HandleItemRender` adds `aria-label` describing drag action
2. `HandleItemRender` sets `aria-disabled="true"` when item can't be dragged
3. `_announcement` live region updates during mouse drops (in `HandleRadzenDrop`)
4. Existing keyboard flow continues to use `_announcement` for all state changes

**Note:** The canonical `HandleItemRender` in Step 2 includes all accessibility attributes.

---

## Edge Cases

### Empty state after drag-out
**Question:** If all items are dragged out of a column, does "No items" appear?
**Answer:** Yes. The `ItemCount` property is computed from `Items.Count()`, which is passed from Board's `GetColumnItems(column.Status)`. When Radzen updates item status via `HandleRadzenDrop`, the item's `Status` property changes, causing it to appear in a different column on next render.

### Keyboard/mouse state conflict
**Question:** If user starts keyboard drag (Space) then clicks with mouse, what happens?

**Solution:** Add transparent overlay during keyboard drag mode. Any mouse click cancels the drag without triggering underlying elements.

**In KanbanBoard.razor** - add inside the board container:
```razor
@if (_draggedItem is not null)
{
    <div class="keyboard-drag-overlay"
         @onclick="CancelKeyboardDrag"
         @onclick:stopPropagation="true">
    </div>
}

@code {
    private void CancelKeyboardDrag()
    {
        _announcement = "Drag cancelled.";
        HandleDragEnd();
    }
}
```

**In kanban.css** - add:
```css
.keyboard-drag-overlay {
    position: absolute;
    inset: 0;
    z-index: 100;
    background: transparent;
    cursor: default;
}
```

**Why this works:**
- Overlay only renders when keyboard drag is active
- Captures all clicks before they reach cards/buttons
- `@onclick:stopPropagation` prevents click from propagating
- Clean separation - no changes needed to child components

### Focus after drop
**Behavior:** After Radzen drop, focus remains on the dropped card (now in new column). Radzen handles this internally. For keyboard drops, focus should stay on the card - verify during testing.

---

## Build Failure Troubleshooting

If any step fails to build:

1. **Check the error message** - Is it a type error, missing using, or syntax error?
2. **Verify previous step** - Did you complete all changes in the prior step?
3. **Common issues:**
   - Missing `@using Radzen` → Already in `_Imports.razor`, should not occur
   - Type mismatch in `Value="@Status"` → Ensure `Status` is `string`, `Value` accepts `object?`
   - RenderFragment compilation error → In `@code` block, use `__builder` pattern (shown in Step 2)
4. **If stuck:** Revert to last working state with `git checkout -- <file>`

---

## Rollback Plan

If Radzen DropZone doesn't work as expected during implementation:

1. **First choice:** Use Radzen's TreeView with drag-and-drop instead of DropZone (pure Blazor)
2. **Second choice:** Try a different Radzen component or pattern (pure Blazor)
3. **Descope:** If no pure Blazor solution exists, remove drag-and-drop and use explicit "Move to..." buttons instead

**Note:** Per Critical Design Constraints, JS interop solutions (Sortable.js, custom JS handlers) are not acceptable options.

**Git strategy:** Create feature branch `fix/kanban-radzen-dropzone` before starting. If issues, revert to `main`.

---

## Browser Compatibility

**CSS selectors used:**
- `.kanban-column > .rz-dropzone` - Direct child selector (all browsers)
- `.rz-dropzone.rz-can-drop` - Class combination (all browsers)
- No `:has()` selector used (limited Safari/Firefox support)

**Radzen compatibility:** Radzen 8.6.2 supports all modern browsers (Chrome, Firefox, Safari, Edge).

---

## Sources

- [Radzen DropZone Demo](https://blazor.radzen.com/dropzone)
- [RadzenDropZoneItemRenderEventArgs API](https://blazor.radzen.com/docs/api/Radzen.RadzenDropZoneItemRenderEventArgs-1.html)
- [Radzen GitHub - RadzenDropZone.razor](https://github.com/radzenhq/radzen-blazor/blob/master/Radzen.Blazor/RadzenDropZone.razor)
