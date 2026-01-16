# Session 30.5: Frontend UI (Components + WASM Host)

## Overview

Create the Razor Component Library with Kanban board, work item cards, and dialogs. Set up the standalone WASM host application.

**Estimated Files**: 10
**Dependencies**: Session 30.4 complete (Core + Services projects exist)

---

## Phase 1: Components Project Setup

### 1.1 Project File

**File**: `frontend/ProjectManagement.Components/ProjectManagement.Components.csproj`

```xml
<Project Sdk="Microsoft.NET.Sdk.Razor">

  <PropertyGroup>
    <TargetFramework>net8.0</TargetFramework>
    <ImplicitUsings>enable</ImplicitUsings>
    <Nullable>enable</Nullable>
  </PropertyGroup>

  <ItemGroup>
    <ProjectReference Include="..\ProjectManagement.Core\ProjectManagement.Core.csproj" />
    <ProjectReference Include="..\ProjectManagement.Services\ProjectManagement.Services.csproj" />
  </ItemGroup>

  <ItemGroup>
    <PackageReference Include="Microsoft.AspNetCore.Components.Web" Version="8.0.0" />
    <PackageReference Include="Radzen.Blazor" Version="4.25.0" />
  </ItemGroup>

</Project>
```

---

### 1.2 Global Imports

**File**: `frontend/ProjectManagement.Components/_Imports.razor`

```razor
@using System.Net.Http
@using Microsoft.AspNetCore.Components.Forms
@using Microsoft.AspNetCore.Components.Routing
@using Microsoft.AspNetCore.Components.Web
@using Microsoft.AspNetCore.Components.Web.Virtualization
@using Microsoft.JSInterop

@using Radzen
@using Radzen.Blazor

@using ProjectManagement.Core.Models
@using ProjectManagement.Core.Enums
@using ProjectManagement.Services.State
@using ProjectManagement.Services.Commands
@using ProjectManagement.Components.Components
@using ProjectManagement.Components.Dialogs
```

---

## Phase 2: Page Components

### 2.1 Project Dashboard

**File**: `frontend/ProjectManagement.Components/Pages/ProjectDashboard.razor`

```razor
@page "/project/{ProjectId:guid}"
@inject ProjectStateManager StateManager
@inject DialogService DialogService
@inject NotificationService NotificationService
@implements IAsyncDisposable

<PageTitle>Project Dashboard</PageTitle>

<RadzenCard>
    <RadzenRow Gap="1rem" AlignItems="AlignItems.Center">
        <RadzenColumn Size="8">
            <RadzenText TextStyle="TextStyle.H4">@_projectTitle</RadzenText>
        </RadzenColumn>
        <RadzenColumn Size="4" Style="text-align: right;">
            <RadzenButton Text="Create Item" Icon="add" Click="@ShowCreateDialog" />
        </RadzenColumn>
    </RadzenRow>
</RadzenCard>

<RadzenTabs @bind-SelectedIndex="_selectedTab" Style="margin-top: 1rem;">
    <Tabs>
        <RadzenTabsItem Text="Kanban Board">
            <KanbanBoard
                WorkItems="@_workItems"
                OnStatusChange="@HandleStatusChange"
                OnCardClick="@HandleCardClick" />
        </RadzenTabsItem>
        <RadzenTabsItem Text="List View">
            <WorkItemList
                WorkItems="@_workItems"
                OnItemClick="@HandleCardClick" />
        </RadzenTabsItem>
    </Tabs>
</RadzenTabs>

@code {
    [Parameter]
    public Guid ProjectId { get; set; }

    private string _projectTitle = "Loading...";
    private IReadOnlyCollection<WorkItem> _workItems = Array.Empty<WorkItem>();
    private int _selectedTab = 0;
    private bool _initialized = false;

    protected override async Task OnInitializedAsync()
    {
        await StateManager.InitializeAsync(ProjectId);
        StateManager.OnStateChanged += HandleStateChanged;
        LoadWorkItems();
        _initialized = true;
    }

    private void HandleStateChanged()
    {
        LoadWorkItems();
        InvokeAsync(StateHasChanged);
    }

    private void LoadWorkItems()
    {
        _workItems = StateManager.State.GetByProject(ProjectId);

        var project = _workItems.FirstOrDefault(i => i.ItemType == WorkItemType.Project);
        _projectTitle = project?.Title ?? "Project";
    }

    private async Task ShowCreateDialog()
    {
        var result = await DialogService.OpenAsync<CreateWorkItemDialog>(
            "Create Work Item",
            new Dictionary<string, object>
            {
                { "ProjectId", ProjectId },
                { "StateManager", StateManager },
            },
            new DialogOptions
            {
                Width = "500px",
                Height = "400px",
            });

        if (result is WorkItem created)
        {
            NotificationService.Notify(new NotificationMessage
            {
                Severity = NotificationSeverity.Success,
                Summary = "Created",
                Detail = $"Work item '{created.Title}' created successfully.",
                Duration = 3000,
            });
        }
    }

    private async Task HandleStatusChange((Guid WorkItemId, WorkItemStatus NewStatus) args)
    {
        var item = StateManager.State.GetById(args.WorkItemId);
        if (item == null) return;

        try
        {
            var commands = new WorkItemCommands(/* inject client */);
            await commands.UpdateAsync(
                args.WorkItemId,
                item.Version,
                status: args.NewStatus);
        }
        catch (WebSocketRequestException ex) when (ex.ErrorCode == "CONFLICT")
        {
            NotificationService.Notify(new NotificationMessage
            {
                Severity = NotificationSeverity.Warning,
                Summary = "Conflict",
                Detail = "Item was modified by another user. Please refresh.",
                Duration = 5000,
            });
        }
        catch (Exception ex)
        {
            NotificationService.Notify(new NotificationMessage
            {
                Severity = NotificationSeverity.Error,
                Summary = "Error",
                Detail = ex.Message,
                Duration = 5000,
            });
        }
    }

    private void HandleCardClick(WorkItem item)
    {
        // Navigate to item detail or open edit dialog
        // Implementation depends on UX requirements
    }

    public async ValueTask DisposeAsync()
    {
        StateManager.OnStateChanged -= HandleStateChanged;
        await StateManager.DisposeAsync();
    }
}
```

---

## Phase 3: Reusable Components

### 3.1 Kanban Board

**File**: `frontend/ProjectManagement.Components/Components/KanbanBoard.razor`

```razor
<div class="kanban-container">
    @foreach (var status in _statuses)
    {
        <div class="kanban-column" @ondrop="() => HandleDrop(status)" @ondragover:preventDefault>
            <div class="kanban-column-header">
                <RadzenText TextStyle="TextStyle.H6">@GetStatusName(status)</RadzenText>
                <RadzenBadge Text="@GetItemCount(status).ToString()" BadgeStyle="BadgeStyle.Secondary" />
            </div>
            <div class="kanban-column-body">
                @foreach (var item in GetItemsForStatus(status))
                {
                    <WorkItemCard
                        Item="@item"
                        OnClick="@(() => OnCardClick.InvokeAsync(item))"
                        OnDragStart="@(() => HandleDragStart(item))" />
                }
            </div>
        </div>
    }
</div>

<style>
    .kanban-container {
        display: flex;
        gap: 1rem;
        overflow-x: auto;
        padding: 1rem 0;
    }

    .kanban-column {
        min-width: 280px;
        max-width: 320px;
        background: var(--rz-base-100);
        border-radius: 8px;
        display: flex;
        flex-direction: column;
    }

    .kanban-column-header {
        padding: 1rem;
        display: flex;
        justify-content: space-between;
        align-items: center;
        border-bottom: 1px solid var(--rz-base-300);
    }

    .kanban-column-body {
        padding: 0.5rem;
        flex: 1;
        overflow-y: auto;
        display: flex;
        flex-direction: column;
        gap: 0.5rem;
    }
</style>

@code {
    [Parameter]
    public IReadOnlyCollection<WorkItem> WorkItems { get; set; } = Array.Empty<WorkItem>();

    [Parameter]
    public EventCallback<(Guid WorkItemId, WorkItemStatus NewStatus)> OnStatusChange { get; set; }

    [Parameter]
    public EventCallback<WorkItem> OnCardClick { get; set; }

    private readonly WorkItemStatus[] _statuses = new[]
    {
        WorkItemStatus.New,
        WorkItemStatus.Active,
        WorkItemStatus.Resolved,
        WorkItemStatus.Closed,
    };

    private WorkItem? _draggedItem;

    private IEnumerable<WorkItem> GetItemsForStatus(WorkItemStatus status)
    {
        return WorkItems
            .Where(i => i.Status == status && i.ItemType != WorkItemType.Project)
            .OrderBy(i => i.Position);
    }

    private int GetItemCount(WorkItemStatus status)
    {
        return WorkItems.Count(i => i.Status == status && i.ItemType != WorkItemType.Project);
    }

    private string GetStatusName(WorkItemStatus status) => status switch
    {
        WorkItemStatus.New => "New",
        WorkItemStatus.Active => "Active",
        WorkItemStatus.Resolved => "Resolved",
        WorkItemStatus.Closed => "Closed",
        _ => status.ToString(),
    };

    private void HandleDragStart(WorkItem item)
    {
        _draggedItem = item;
    }

    private async Task HandleDrop(WorkItemStatus newStatus)
    {
        if (_draggedItem == null) return;

        if (_draggedItem.Status != newStatus)
        {
            await OnStatusChange.InvokeAsync((_draggedItem.Id, newStatus));
        }

        _draggedItem = null;
    }
}
```

---

### 3.2 Work Item Card

**File**: `frontend/ProjectManagement.Components/Components/WorkItemCard.razor`

```razor
<div class="work-item-card @GetTypeClass()"
     draggable="true"
     @ondragstart="OnDragStart"
     @onclick="() => OnClick.InvokeAsync()">

    <div class="card-header">
        <span class="item-type">@GetTypeBadge()</span>
        <span class="item-priority @GetPriorityClass()">@GetPriorityIcon()</span>
    </div>

    <div class="card-title">@Item.Title</div>

    <div class="card-footer">
        @if (Item.StoryPoints.HasValue)
        {
            <RadzenBadge Text="@($"{Item.StoryPoints} pts")" BadgeStyle="BadgeStyle.Info" />
        }
        @if (Item.AssigneeId.HasValue)
        {
            <span class="assignee-avatar" title="Assigned">@GetInitials()</span>
        }
    </div>
</div>

<style>
    .work-item-card {
        background: white;
        border-radius: 6px;
        padding: 0.75rem;
        cursor: pointer;
        box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
        border-left: 4px solid var(--rz-primary);
        transition: box-shadow 0.2s;
    }

    .work-item-card:hover {
        box-shadow: 0 4px 8px rgba(0, 0, 0, 0.15);
    }

    .work-item-card.epic { border-left-color: #9c27b0; }
    .work-item-card.story { border-left-color: #2196f3; }
    .work-item-card.task { border-left-color: #4caf50; }

    .card-header {
        display: flex;
        justify-content: space-between;
        margin-bottom: 0.5rem;
        font-size: 0.75rem;
    }

    .item-type {
        text-transform: uppercase;
        color: var(--rz-text-secondary-color);
        font-weight: 500;
    }

    .item-priority.critical { color: #f44336; }
    .item-priority.high { color: #ff9800; }
    .item-priority.medium { color: #2196f3; }
    .item-priority.low { color: #9e9e9e; }

    .card-title {
        font-weight: 500;
        margin-bottom: 0.5rem;
        line-height: 1.3;
    }

    .card-footer {
        display: flex;
        justify-content: space-between;
        align-items: center;
        font-size: 0.75rem;
    }

    .assignee-avatar {
        width: 24px;
        height: 24px;
        border-radius: 50%;
        background: var(--rz-primary);
        color: white;
        display: flex;
        align-items: center;
        justify-content: center;
        font-size: 0.7rem;
    }
</style>

@code {
    [Parameter]
    public WorkItem Item { get; set; } = null!;

    [Parameter]
    public EventCallback OnClick { get; set; }

    [Parameter]
    public EventCallback OnDragStart { get; set; }

    private string GetTypeClass() => Item.ItemType switch
    {
        WorkItemType.Epic => "epic",
        WorkItemType.Story => "story",
        WorkItemType.Task => "task",
        _ => "",
    };

    private string GetTypeBadge() => Item.ItemType switch
    {
        WorkItemType.Epic => "EPIC",
        WorkItemType.Story => "STORY",
        WorkItemType.Task => "TASK",
        _ => Item.ItemType.ToString().ToUpper(),
    };

    private string GetPriorityClass() => Item.Priority switch
    {
        WorkItemPriority.Critical => "critical",
        WorkItemPriority.High => "high",
        WorkItemPriority.Medium => "medium",
        WorkItemPriority.Low => "low",
        _ => "",
    };

    private string GetPriorityIcon() => Item.Priority switch
    {
        WorkItemPriority.Critical => "!!!",
        WorkItemPriority.High => "!!",
        WorkItemPriority.Medium => "!",
        WorkItemPriority.Low => "-",
        _ => "",
    };

    private string GetInitials()
    {
        // In real app, look up user name from ID
        return "??";
    }
}
```

---

### 3.3 Work Item List

**File**: `frontend/ProjectManagement.Components/Components/WorkItemList.razor`

```razor
<RadzenDataGrid Data="@FilteredItems"
                TItem="WorkItem"
                AllowFiltering="true"
                AllowSorting="true"
                AllowPaging="true"
                PageSize="20"
                PagerHorizontalAlign="HorizontalAlign.Center"
                RowClick="@(args => OnItemClick.InvokeAsync(args.Data))">
    <Columns>
        <RadzenDataGridColumn TItem="WorkItem" Property="ItemType" Title="Type" Width="100px">
            <Template Context="item">
                <RadzenBadge Text="@item.ItemType.ToString()" BadgeStyle="@GetTypeBadgeStyle(item.ItemType)" />
            </Template>
        </RadzenDataGridColumn>

        <RadzenDataGridColumn TItem="WorkItem" Property="Title" Title="Title" />

        <RadzenDataGridColumn TItem="WorkItem" Property="Status" Title="Status" Width="120px">
            <Template Context="item">
                <RadzenBadge Text="@item.Status.ToString()" BadgeStyle="@GetStatusBadgeStyle(item.Status)" />
            </Template>
        </RadzenDataGridColumn>

        <RadzenDataGridColumn TItem="WorkItem" Property="Priority" Title="Priority" Width="100px">
            <Template Context="item">
                <span class="@GetPriorityClass(item.Priority)">@item.Priority</span>
            </Template>
        </RadzenDataGridColumn>

        <RadzenDataGridColumn TItem="WorkItem" Property="StoryPoints" Title="Points" Width="80px" TextAlign="TextAlign.Center" />

        <RadzenDataGridColumn TItem="WorkItem" Property="UpdatedAt" Title="Updated" Width="150px">
            <Template Context="item">
                @item.UpdatedAt.ToString("MMM dd, HH:mm")
            </Template>
        </RadzenDataGridColumn>
    </Columns>
</RadzenDataGrid>

@code {
    [Parameter]
    public IReadOnlyCollection<WorkItem> WorkItems { get; set; } = Array.Empty<WorkItem>();

    [Parameter]
    public EventCallback<WorkItem> OnItemClick { get; set; }

    private IEnumerable<WorkItem> FilteredItems =>
        WorkItems.Where(i => i.ItemType != WorkItemType.Project);

    private BadgeStyle GetTypeBadgeStyle(WorkItemType type) => type switch
    {
        WorkItemType.Epic => BadgeStyle.Secondary,
        WorkItemType.Story => BadgeStyle.Primary,
        WorkItemType.Task => BadgeStyle.Success,
        _ => BadgeStyle.Light,
    };

    private BadgeStyle GetStatusBadgeStyle(WorkItemStatus status) => status switch
    {
        WorkItemStatus.New => BadgeStyle.Info,
        WorkItemStatus.Active => BadgeStyle.Primary,
        WorkItemStatus.Resolved => BadgeStyle.Success,
        WorkItemStatus.Closed => BadgeStyle.Light,
        _ => BadgeStyle.Light,
    };

    private string GetPriorityClass(WorkItemPriority priority) => priority switch
    {
        WorkItemPriority.Critical => "priority-critical",
        WorkItemPriority.High => "priority-high",
        _ => "",
    };
}
```

---

## Phase 4: Dialogs

### 4.1 Create Work Item Dialog

**File**: `frontend/ProjectManagement.Components/Dialogs/CreateWorkItemDialog.razor`

```razor
@inject DialogService DialogService

<RadzenTemplateForm TItem="CreateWorkItemModel" Data="@_model" Submit="@HandleSubmit">
    <RadzenStack Gap="1rem">
        <RadzenFormField Text="Type" Variant="Variant.Outlined">
            <RadzenDropDown @bind-Value="_model.ItemType"
                            Data="@_allowedTypes"
                            TextProperty="Text"
                            ValueProperty="Value"
                            Style="width: 100%;" />
        </RadzenFormField>

        <RadzenFormField Text="Title" Variant="Variant.Outlined">
            <RadzenTextBox @bind-Value="_model.Title"
                           Placeholder="Enter title..."
                           Style="width: 100%;"
                           MaxLength="500" />
        </RadzenFormField>

        <RadzenFormField Text="Description" Variant="Variant.Outlined">
            <RadzenTextArea @bind-Value="_model.Description"
                            Placeholder="Enter description..."
                            Style="width: 100%; min-height: 100px;" />
        </RadzenFormField>

        @if (_model.ItemType != WorkItemType.Epic)
        {
            <RadzenFormField Text="Parent" Variant="Variant.Outlined">
                <RadzenDropDown @bind-Value="_model.ParentId"
                                Data="@_availableParents"
                                TextProperty="Title"
                                ValueProperty="Id"
                                AllowClear="true"
                                Placeholder="Select parent..."
                                Style="width: 100%;" />
            </RadzenFormField>
        }

        <RadzenStack Orientation="Orientation.Horizontal" JustifyContent="JustifyContent.End" Gap="0.5rem">
            <RadzenButton Text="Cancel" ButtonStyle="ButtonStyle.Light" Click="@Cancel" />
            <RadzenButton Text="Create" ButtonType="ButtonType.Submit" ButtonStyle="ButtonStyle.Primary" />
        </RadzenStack>
    </RadzenStack>
</RadzenTemplateForm>

@code {
    [Parameter]
    public Guid ProjectId { get; set; }

    [Parameter]
    public ProjectStateManager StateManager { get; set; } = null!;

    private CreateWorkItemModel _model = new();
    private List<TypeOption> _allowedTypes = new();
    private List<WorkItem> _availableParents = new();

    protected override void OnInitialized()
    {
        _model.ProjectId = ProjectId;

        _allowedTypes = new List<TypeOption>
        {
            new("Epic", WorkItemType.Epic),
            new("Story", WorkItemType.Story),
            new("Task", WorkItemType.Task),
        };

        _model.ItemType = WorkItemType.Story;
        UpdateAvailableParents();
    }

    private void UpdateAvailableParents()
    {
        var allItems = StateManager.State.GetByProject(ProjectId);

        _availableParents = _model.ItemType switch
        {
            WorkItemType.Epic => allItems.Where(i => i.ItemType == WorkItemType.Project).ToList(),
            WorkItemType.Story => allItems.Where(i => i.ItemType == WorkItemType.Epic).ToList(),
            WorkItemType.Task => allItems.Where(i => i.ItemType == WorkItemType.Story).ToList(),
            _ => new List<WorkItem>(),
        };
    }

    private async Task HandleSubmit()
    {
        if (string.IsNullOrWhiteSpace(_model.Title))
        {
            return;
        }

        try
        {
            var commands = new WorkItemCommands(/* inject client */);
            var created = await commands.CreateAsync(
                _model.ItemType,
                _model.Title,
                _model.Description,
                _model.ProjectId,
                _model.ParentId);

            DialogService.Close(created);
        }
        catch (Exception ex)
        {
            // Show error notification
        }
    }

    private void Cancel()
    {
        DialogService.Close(null);
    }

    private class CreateWorkItemModel
    {
        public WorkItemType ItemType { get; set; }
        public string Title { get; set; } = string.Empty;
        public string? Description { get; set; }
        public Guid ProjectId { get; set; }
        public Guid? ParentId { get; set; }
    }

    private record TypeOption(string Text, WorkItemType Value);
}
```

---

## Phase 5: WASM Host

### 5.1 Project File

**File**: `frontend/ProjectManagement.Wasm/ProjectManagement.Wasm.csproj`

```xml
<Project Sdk="Microsoft.NET.Sdk.BlazorWebAssembly">

  <PropertyGroup>
    <TargetFramework>net8.0</TargetFramework>
    <ImplicitUsings>enable</ImplicitUsings>
    <Nullable>enable</Nullable>
  </PropertyGroup>

  <ItemGroup>
    <ProjectReference Include="..\ProjectManagement.Components\ProjectManagement.Components.csproj" />
  </ItemGroup>

  <ItemGroup>
    <PackageReference Include="Microsoft.AspNetCore.Components.WebAssembly" Version="8.0.0" />
    <PackageReference Include="Microsoft.AspNetCore.Components.WebAssembly.DevServer" Version="8.0.0" PrivateAssets="all" />
    <PackageReference Include="Radzen.Blazor" Version="4.25.0" />
  </ItemGroup>

</Project>
```

---

### 5.2 Index HTML

**File**: `frontend/ProjectManagement.Wasm/wwwroot/index.html`

```html
<!DOCTYPE html>
<html lang="en">

<head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Project Management</title>
    <base href="/" />
    <link rel="stylesheet" href="_content/Radzen.Blazor/css/material-base.css" />
    <link href="css/app.css" rel="stylesheet" />
    <link href="ProjectManagement.Wasm.styles.css" rel="stylesheet" />
</head>

<body>
    <div id="app">
        <div style="display: flex; justify-content: center; align-items: center; height: 100vh;">
            <div>Loading...</div>
        </div>
    </div>

    <div id="blazor-error-ui">
        An unhandled error has occurred.
        <a href="" class="reload">Reload</a>
        <a class="dismiss">Dismiss</a>
    </div>

    <script src="_content/Radzen.Blazor/Radzen.Blazor.js"></script>
    <script src="_framework/blazor.webassembly.js"></script>
</body>

</html>
```

---

### 5.3 Program Entry Point

**File**: `frontend/ProjectManagement.Wasm/Program.cs`

```csharp
using Microsoft.AspNetCore.Components.Web;
using Microsoft.AspNetCore.Components.WebAssembly.Hosting;
using Microsoft.Extensions.Logging;
using Radzen;
using ProjectManagement.Services.State;
using ProjectManagement.Services.WebSocket;

namespace ProjectManagement.Wasm;

public class Program
{
    public static async Task Main(string[] args)
    {
        var builder = WebAssemblyHostBuilder.CreateDefault(args);
        builder.RootComponents.Add<App>("#app");
        builder.RootComponents.Add<HeadOutlet>("head::after");

        // Configuration
        var backendUrl = builder.Configuration["BackendUrl"] ?? "wss://localhost:5001/ws";
        var jwtToken = builder.Configuration["JwtToken"] ?? "";

        // Logging
        builder.Logging.SetMinimumLevel(LogLevel.Information);

        // Radzen services
        builder.Services.AddScoped<DialogService>();
        builder.Services.AddScoped<NotificationService>();
        builder.Services.AddScoped<TooltipService>();
        builder.Services.AddScoped<ContextMenuService>();

        // Application services
        builder.Services.AddScoped(sp =>
        {
            var logger = sp.GetRequiredService<ILogger<ProjectStateManager>>();
            var clientLogger = sp.GetRequiredService<ILogger<ProjectManagementWebSocketClient>>();
            return new ProjectStateManager(logger, clientLogger, new Uri(backendUrl), jwtToken);
        });

        await builder.Build().RunAsync();
    }
}
```

---

### 5.4 App Component

**File**: `frontend/ProjectManagement.Wasm/App.razor`

```razor
<RadzenComponents />

<Router AppAssembly="@typeof(App).Assembly"
        AdditionalAssemblies="new[] { typeof(ProjectManagement.Components._Imports).Assembly }">
    <Found Context="routeData">
        <RouteView RouteData="@routeData" DefaultLayout="@typeof(MainLayout)" />
        <FocusOnNavigate RouteData="@routeData" Selector="h1" />
    </Found>
    <NotFound>
        <PageTitle>Not found</PageTitle>
        <LayoutView Layout="@typeof(MainLayout)">
            <p role="alert">Sorry, there's nothing at this address.</p>
        </LayoutView>
    </NotFound>
</Router>
```

---

### 5.5 Main Layout

**File**: `frontend/ProjectManagement.Wasm/MainLayout.razor`

```razor
@inherits LayoutComponentBase

<RadzenLayout>
    <RadzenHeader>
        <RadzenStack Orientation="Orientation.Horizontal" AlignItems="AlignItems.Center" Gap="0.5rem" Style="padding: 0 1rem;">
            <RadzenText TextStyle="TextStyle.H5" Style="margin: 0;">Project Management</RadzenText>
        </RadzenStack>
    </RadzenHeader>

    <RadzenBody>
        <div class="container-fluid" style="padding: 1rem;">
            @Body
        </div>
    </RadzenBody>
</RadzenLayout>

<RadzenNotification />
<RadzenDialog />
<RadzenTooltip />
<RadzenContextMenu />
```

---

## File Summary

| Action | Path |
|--------|------|
| Create | `frontend/ProjectManagement.Components/ProjectManagement.Components.csproj` |
| Create | `frontend/ProjectManagement.Components/_Imports.razor` |
| Create | `frontend/ProjectManagement.Components/Pages/ProjectDashboard.razor` |
| Create | `frontend/ProjectManagement.Components/Components/KanbanBoard.razor` |
| Create | `frontend/ProjectManagement.Components/Components/WorkItemCard.razor` |
| Create | `frontend/ProjectManagement.Components/Components/WorkItemList.razor` |
| Create | `frontend/ProjectManagement.Components/Dialogs/CreateWorkItemDialog.razor` |
| Create | `frontend/ProjectManagement.Wasm/ProjectManagement.Wasm.csproj` |
| Create | `frontend/ProjectManagement.Wasm/wwwroot/index.html` |
| Create | `frontend/ProjectManagement.Wasm/Program.cs` |
| Create | `frontend/ProjectManagement.Wasm/App.razor` |
| Create | `frontend/ProjectManagement.Wasm/MainLayout.razor` |

---

## Verification

```bash
cd frontend

# Build entire solution
dotnet build ProjectManagement.sln

# Run the WASM app
dotnet run --project ProjectManagement.Wasm

# Open browser to https://localhost:5000 (or configured port)
```

---

## End-to-End Test Checklist (Full Session 30)

After completing all Session 30 sub-sessions:

### Backend Tests
1. [ ] All migrations run successfully
2. [ ] Repository unit tests pass
3. [ ] Handler integration tests pass
4. [ ] WebSocket connection with valid JWT works
5. [ ] Unauthorized requests rejected

### Frontend Tests
1. [ ] Solution builds without errors
2. [ ] WASM app starts and loads
3. [ ] Can connect to WebSocket server
4. [ ] Dashboard displays work items

### Integration Tests
1. [ ] Subscribe to project receives initial data
2. [ ] Create work item updates Kanban board
3. [ ] Drag-and-drop status change persists
4. [ ] Second browser receives broadcast updates
5. [ ] Conflict detection shows warning
6. [ ] Reconnection syncs missed changes
7. [ ] Delete blocked when children exist
