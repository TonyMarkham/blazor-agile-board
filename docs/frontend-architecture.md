# Frontend Architecture - Blazor WebAssembly + Radzen

This document outlines the structure and organization of the Blazor WebAssembly frontend.

## Project Structure

```
/frontend
├── ProjectManagement.sln
│
├── /ProjectManagement.Core          # Shared models, DTOs, interfaces
│   ├── ProjectManagement.Core.csproj
│   └── /
│       ├── /Models                   # Domain models matching backend
│       ├── /Dto                      # Data transfer objects for API
│       ├── /Interfaces               # Service abstractions
│       ├── /Proto                    # Generated protobuf C# code
│       └── /Enums                    # Shared enumerations
│
├── /ProjectManagement.Services      # Business logic and data access
│   ├── ProjectManagement.Services.csproj
│   └── /
│       ├── /Api                      # REST API clients
│       ├── /WebSocket                # WebSocket client and handling
│       ├── /State                    # Application state management
│       └── /Mapping                  # DTO ↔ Model mapping
│
├── /ProjectManagement.Components    # Reusable Razor Class Library (RCL)
│   ├── ProjectManagement.Components.csproj
│   └── /
│       ├── _Imports.razor
│       ├── /Pages                    # Routable page components
│       ├── /Components               # Reusable UI components
│       ├── /Layouts                  # Layout components
│       └── /wwwroot                  # Static assets, CSS
│
└── /ProjectManagement.Wasm          # Standalone WASM host
    ├── ProjectManagement.Wasm.csproj
    └── /
        ├── Program.cs                # Entry point, DI setup
        ├── App.razor                 # Root component
        └── /wwwroot
            └── index.html            # HTML shell
```

---

## Project Descriptions

### ProjectManagement.Core

**Purpose**: Shared types, interfaces, DTOs - no UI, no dependencies on Blazor

**Key Types:**

```csharp
// Models/WorkItem.cs
public class WorkItem
{
    public Guid Id { get; set; }
    public WorkItemType ItemType { get; set; }
    public string Title { get; set; } = string.Empty;
    public string? Description { get; set; }
    public string Status { get; set; } = "backlog";

    public Guid? ParentId { get; set; }
    public Guid ProjectId { get; set; }
    public int Position { get; set; }

    public Guid? AssigneeId { get; set; }
    public Guid? SprintId { get; set; }

    public DateTimeOffset CreatedAt { get; set; }
    public DateTimeOffset UpdatedAt { get; set; }
    public Guid CreatedBy { get; set; }
    public Guid UpdatedBy { get; set; }
}

public enum WorkItemType
{
    Project,
    Epic,
    Story,
    Task
}

// Dto/CreateWorkItemDto.cs
public record CreateWorkItemDto(
    WorkItemType ItemType,
    string Title,
    string? Description,
    Guid? ParentId,
    Guid? AssigneeId
);

// Interfaces/IProjectManagementService.cs
public interface IProjectManagementService
{
    Task<IEnumerable<WorkItem>> GetProjectWorkItemsAsync(Guid projectId);
    Task<WorkItem> GetWorkItemAsync(Guid id);
    Task<WorkItem> CreateWorkItemAsync(CreateWorkItemDto dto);
    Task<WorkItem> UpdateWorkItemAsync(Guid id, UpdateWorkItemDto dto);
    Task DeleteWorkItemAsync(Guid id);
}
```

**Proto Generated Code:**
```bash
# Generate C# protobuf code
protoc --csharp_out=Proto --proto_path=../../../proto ../../../proto/messages.proto
```

**Dependencies:**
- Google.Protobuf
- System.ComponentModel.Annotations (for validation)

---

### ProjectManagement.Services

**Purpose**: API clients, WebSocket handling, state management

**API Clients:**

```csharp
// Api/WorkItemApiClient.cs
public class WorkItemApiClient
{
    private readonly HttpClient _httpClient;
    private readonly string _baseUrl = "/api/v1/work-items";

    public WorkItemApiClient(HttpClient httpClient)
    {
        _httpClient = httpClient;
    }

    public async Task<IEnumerable<WorkItemDto>> GetByProjectAsync(Guid projectId)
    {
        var response = await _httpClient.GetAsync($"{_baseUrl}?projectId={projectId}");
        response.EnsureSuccessStatusCode();
        return await response.Content.ReadFromJsonAsync<IEnumerable<WorkItemDto>>()
            ?? Enumerable.Empty<WorkItemDto>();
    }

    public async Task<WorkItemDto> CreateAsync(CreateWorkItemDto dto)
    {
        var response = await _httpClient.PostAsJsonAsync(_baseUrl, dto);
        response.EnsureSuccessStatusCode();
        return await response.Content.ReadFromJsonAsync<WorkItemDto>()
            ?? throw new Exception("Failed to create work item");
    }

    // ... other CRUD methods
}
```

**WebSocket Client:**

```csharp
// WebSocket/ProjectManagementWebSocketClient.cs
public class ProjectManagementWebSocketClient : IAsyncDisposable
{
    private ClientWebSocket? _webSocket;
    private readonly string _wsUrl;
    private readonly string _jwtToken;
    private CancellationTokenSource _cts = new();

    private readonly Channel<WebSocketMessage> _outgoing;
    private readonly Channel<WebSocketMessage> _incoming;

    public IAsyncEnumerable<WebSocketMessage> Messages => _incoming.Reader.ReadAllAsync();

    public ProjectManagementWebSocketClient(string wsUrl, string jwtToken)
    {
        _wsUrl = wsUrl;
        _jwtToken = jwtToken;
        _outgoing = Channel.CreateUnbounded<WebSocketMessage>();
        _incoming = Channel.CreateUnbounded<WebSocketMessage>();
    }

    public async Task ConnectAsync()
    {
        _webSocket = new ClientWebSocket();
        _webSocket.Options.SetRequestHeader("Authorization", $"Bearer {_jwtToken}");

        await _webSocket.ConnectAsync(new Uri(_wsUrl), _cts.Token);

        // Start send/receive tasks
        _ = Task.Run(SendLoop);
        _ = Task.Run(ReceiveLoop);
    }

    private async Task SendLoop()
    {
        await foreach (var message in _outgoing.Reader.ReadAllAsync(_cts.Token))
        {
            using var ms = new MemoryStream();
            message.WriteTo(ms);
            var bytes = ms.ToArray();

            await _webSocket!.SendAsync(
                new ArraySegment<byte>(bytes),
                WebSocketMessageType.Binary,
                endOfMessage: true,
                _cts.Token
            );
        }
    }

    private async Task ReceiveLoop()
    {
        var buffer = new byte[8192];

        while (!_cts.Token.IsCancellationRequested)
        {
            var result = await _webSocket!.ReceiveAsync(
                new ArraySegment<byte>(buffer),
                _cts.Token
            );

            if (result.MessageType == WebSocketMessageType.Close)
            {
                await HandleDisconnectAsync();
                break;
            }

            // Decode protobuf message
            var message = WebSocketMessage.Parser.ParseFrom(buffer, 0, result.Count);
            await _incoming.Writer.WriteAsync(message, _cts.Token);
        }
    }

    public async Task SubscribeAsync(IEnumerable<Guid> projectIds, IEnumerable<Guid> sprintIds)
    {
        var message = new WebSocketMessage
        {
            MessageId = Guid.NewGuid().ToString(),
            Timestamp = DateTimeOffset.UtcNow.ToUnixTimeSeconds(),
            Subscribe = new Subscribe
            {
                ProjectIds = { projectIds.Select(id => id.ToString()) },
                SprintIds = { sprintIds.Select(id => id.ToString()) }
            }
        };

        await _outgoing.Writer.WriteAsync(message);
    }

    public async ValueTask DisposeAsync()
    {
        _cts.Cancel();
        _outgoing.Writer.Complete();
        _incoming.Writer.Complete();

        if (_webSocket != null)
        {
            await _webSocket.CloseAsync(
                WebSocketCloseStatus.NormalClosure,
                "Disposing",
                CancellationToken.None
            );
            _webSocket.Dispose();
        }
    }
}
```

**State Management:**

```csharp
// State/ProjectState.cs
public class ProjectState
{
    private readonly Dictionary<Guid, WorkItem> _workItems = new();
    private readonly Dictionary<Guid, Sprint> _sprints = new();

    public event Action? OnChange;

    public IEnumerable<WorkItem> WorkItems => _workItems.Values;
    public IEnumerable<Sprint> Sprints => _sprints.Values;

    public void SetWorkItems(IEnumerable<WorkItem> items)
    {
        _workItems.Clear();
        foreach (var item in items)
        {
            _workItems[item.Id] = item;
        }
        OnChange?.Invoke();
    }

    public void UpdateWorkItem(WorkItem item)
    {
        _workItems[item.Id] = item;
        OnChange?.Invoke();
    }

    public void RemoveWorkItem(Guid id)
    {
        _workItems.Remove(id);
        OnChange?.Invoke();
    }

    public WorkItem? GetWorkItem(Guid id)
    {
        return _workItems.GetValueOrDefault(id);
    }

    public IEnumerable<WorkItem> GetChildren(Guid parentId)
    {
        return _workItems.Values
            .Where(w => w.ParentId == parentId)
            .OrderBy(w => w.Position);
    }
}

// State/ProjectStateManager.cs
public class ProjectStateManager : IAsyncDisposable
{
    private readonly ProjectState _state;
    private readonly ProjectManagementWebSocketClient _wsClient;
    private readonly WorkItemApiClient _apiClient;
    private CancellationTokenSource _cts = new();

    public ProjectStateManager(
        ProjectState state,
        ProjectManagementWebSocketClient wsClient,
        WorkItemApiClient apiClient)
    {
        _state = state;
        _wsClient = wsClient;
        _apiClient = apiClient;
    }

    public async Task InitializeAsync(Guid projectId)
    {
        // 1. Load initial data via REST
        var items = await _apiClient.GetByProjectAsync(projectId);
        _state.SetWorkItems(items);

        // 2. Connect WebSocket
        await _wsClient.ConnectAsync();

        // 3. Subscribe to updates
        await _wsClient.SubscribeAsync(new[] { projectId }, Array.Empty<Guid>());

        // 4. Start listening for WebSocket messages
        _ = Task.Run(HandleWebSocketMessages);
    }

    private async Task HandleWebSocketMessages()
    {
        await foreach (var message in _wsClient.Messages.WithCancellation(_cts.Token))
        {
            switch (message.PayloadCase)
            {
                case WebSocketMessage.PayloadOneofCase.WorkItemCreated:
                    HandleWorkItemCreated(message.WorkItemCreated);
                    break;

                case WebSocketMessage.PayloadOneofCase.WorkItemUpdated:
                    HandleWorkItemUpdated(message.WorkItemUpdated);
                    break;

                case WebSocketMessage.PayloadOneofCase.WorkItemDeleted:
                    HandleWorkItemDeleted(message.WorkItemDeleted);
                    break;

                // ... other cases
            }
        }
    }

    private void HandleWorkItemCreated(WorkItemCreated evt)
    {
        var item = MapFromProto(evt.WorkItem);
        _state.UpdateWorkItem(item);
    }

    private void HandleWorkItemUpdated(WorkItemUpdated evt)
    {
        var item = MapFromProto(evt.WorkItem);
        _state.UpdateWorkItem(item);
    }

    public async ValueTask DisposeAsync()
    {
        _cts.Cancel();
        await _wsClient.DisposeAsync();
    }
}
```

**Dependencies:**
- System.Net.Http
- System.Net.WebSockets
- System.Threading.Channels
- Google.Protobuf
- ProjectManagement.Core

---

### ProjectManagement.Components (Razor Class Library)

**Purpose**: Reusable UI components using Radzen

**Page Components:**

```razor
<!-- Pages/ProjectBoard.razor -->
@page "/projects/{ProjectId:guid}/board"
@inject ProjectState State
@inject NavigationManager Nav
@implements IDisposable

<PageTitle>Project Board</PageTitle>

<RadzenStack Orientation="Orientation.Vertical" Gap="1rem">
    <RadzenRow>
        <RadzenColumn Size="12">
            <RadzenText TextStyle="TextStyle.H3">@_projectName</RadzenText>
        </RadzenColumn>
    </RadzenRow>

    <RadzenRow>
        <RadzenColumn Size="12">
            <RadzenTabs>
                <Tabs>
                    <RadzenTabsItem Text="Board">
                        <KanbanBoard WorkItems="@_workItems" OnItemMoved="HandleItemMoved" />
                    </RadzenTabsItem>
                    <RadzenTabsItem Text="Backlog">
                        <BacklogView WorkItems="@_backlogItems" />
                    </RadzenTabsItem>
                    <RadzenTabsItem Text="Timeline">
                        <TimelineView Sprints="@_sprints" />
                    </RadzenTabsItem>
                </Tabs>
            </RadzenTabs>
        </RadzenColumn>
    </RadzenRow>
</RadzenStack>

@code {
    [Parameter] public Guid ProjectId { get; set; }

    private string _projectName = "Loading...";
    private IEnumerable<WorkItem> _workItems = Array.Empty<WorkItem>();
    private IEnumerable<WorkItem> _backlogItems = Array.Empty<WorkItem>();
    private IEnumerable<Sprint> _sprints = Array.Empty<Sprint>();

    protected override void OnInitialized()
    {
        State.OnChange += StateHasChanged;
        LoadData();
    }

    private void LoadData()
    {
        _workItems = State.WorkItems.Where(w => w.ProjectId == ProjectId);
        _backlogItems = _workItems.Where(w => w.SprintId == null);
        // ...
    }

    private async Task HandleItemMoved(WorkItemMovedEventArgs args)
    {
        // Update via API, WebSocket will broadcast change
        await WorkItemService.MoveAsync(args.ItemId, args.NewStatus, args.NewPosition);
    }

    public void Dispose()
    {
        State.OnChange -= StateHasChanged;
    }
}
```

**Component Library:**

```razor
<!-- Components/KanbanBoard.razor -->
@using Radzen.Blazor

<RadzenRow>
    @foreach (var lane in _swimLanes)
    {
        <RadzenColumn Size="3">
            <RadzenCard>
                <RadzenStack Orientation="Orientation.Vertical" Gap="0.5rem">
                    <RadzenText TextStyle="TextStyle.H6">@lane.Name</RadzenText>
                    <RadzenBadge Text="@GetItemCount(lane.StatusValue).ToString()" />

                    @foreach (var item in GetItemsInLane(lane.StatusValue))
                    {
                        <WorkItemCard WorkItem="@item" OnClick="() => ShowDetails(item)" />
                    }
                </RadzenStack>
            </RadzenCard>
        </RadzenColumn>
    }
</RadzenRow>

@code {
    [Parameter] public IEnumerable<WorkItem> WorkItems { get; set; } = Array.Empty<WorkItem>();
    [Parameter] public EventCallback<WorkItemMovedEventArgs> OnItemMoved { get; set; }

    private List<SwimLane> _swimLanes = new()
    {
        new("Backlog", "backlog"),
        new("In Progress", "in_progress"),
        new("In Review", "in_review"),
        new("Done", "done")
    };

    private IEnumerable<WorkItem> GetItemsInLane(string status)
    {
        return WorkItems.Where(w => w.Status == status).OrderBy(w => w.Position);
    }

    private int GetItemCount(string status) => GetItemsInLane(status).Count();

    private async Task ShowDetails(WorkItem item)
    {
        await DialogService.OpenAsync<WorkItemDetailDialog>(
            $"{item.Title}",
            new Dictionary<string, object> { { "WorkItem", item } }
        );
    }
}

record SwimLane(string Name, string StatusValue);
```

```razor
<!-- Components/WorkItemCard.razor -->
<RadzenCard class="work-item-card" @onclick="OnClick">
    <RadzenStack Orientation="Orientation.Vertical" Gap="0.25rem">
        <RadzenRow>
            <RadzenColumn Size="8">
                <RadzenBadge Text="@WorkItem.ItemType.ToString()" BadgeStyle="BadgeStyle.Info" />
            </RadzenColumn>
            <RadzenColumn Size="4" class="text-right">
                @if (WorkItem.AssigneeId.HasValue)
                {
                    <RadzenImage Path="@GetAvatarUrl(WorkItem.AssigneeId.Value)"
                                 Style="width: 24px; height: 24px; border-radius: 50%;" />
                }
            </RadzenColumn>
        </RadzenRow>

        <RadzenText TextStyle="TextStyle.Body1">@WorkItem.Title</RadzenText>

        @if (HasDependencies)
        {
            <RadzenBadge Text="Blocked" BadgeStyle="BadgeStyle.Danger" />
        }
    </RadzenStack>
</RadzenCard>

@code {
    [Parameter] public WorkItem WorkItem { get; set; } = null!;
    [Parameter] public EventCallback OnClick { get; set; }

    private bool HasDependencies => false; // TODO: Check dependencies

    private string GetAvatarUrl(Guid userId)
    {
        return $"/api/v1/users/{userId}/avatar";
    }
}
```

**Dialogs:**

```razor
<!-- Components/Dialogs/WorkItemDetailDialog.razor -->
@inject WorkItemApiClient ApiClient

<RadzenStack Orientation="Orientation.Vertical" Gap="1rem">
    <!-- Header -->
    <RadzenRow>
        <RadzenColumn Size="10">
            <RadzenText TextStyle="TextStyle.H5">@WorkItem.Title</RadzenText>
        </RadzenColumn>
        <RadzenColumn Size="2" class="text-right">
            <RadzenButton Icon="close" ButtonStyle="ButtonStyle.Light"
                          Click="@(() => DialogService.Close())" />
        </RadzenColumn>
    </RadzenRow>

    <!-- Tabs -->
    <RadzenTabs>
        <Tabs>
            <RadzenTabsItem Text="Details">
                <WorkItemDetailsTab WorkItem="@WorkItem" OnSave="HandleSave" />
            </RadzenTabsItem>

            <RadzenTabsItem Text="Comments">
                <CommentsTab WorkItemId="@WorkItem.Id" />
            </RadzenTabsItem>

            <RadzenTabsItem Text="Time Tracking">
                <TimeTrackingTab WorkItemId="@WorkItem.Id" />
            </RadzenTabsItem>

            <RadzenTabsItem Text="History">
                <ActivityHistoryTab WorkItemId="@WorkItem.Id" />
            </RadzenTabsItem>
        </Tabs>
    </RadzenTabs>
</RadzenStack>

@code {
    [Parameter] public WorkItem WorkItem { get; set; } = null!;

    private async Task HandleSave(UpdateWorkItemDto dto)
    {
        await ApiClient.UpdateAsync(WorkItem.Id, dto);
        DialogService.Close(true);
    }
}
```

**Dependencies:**
- Microsoft.AspNetCore.Components.Web
- Microsoft.AspNetCore.Components.WebAssembly
- Radzen.Blazor
- ProjectManagement.Core
- ProjectManagement.Services

---

### ProjectManagement.Wasm (Standalone Host)

**Purpose**: Entry point for standalone deployment

```csharp
// Program.cs
using Microsoft.AspNetCore.Components.Web;
using Microsoft.AspNetCore.Components.WebAssembly.Hosting;
using ProjectManagement.Wasm;
using ProjectManagement.Services;
using ProjectManagement.Components;
using Radzen;

var builder = WebAssemblyHostBuilder.CreateDefault(args);
builder.RootComponents.Add<App>("#app");
builder.RootComponents.Add<HeadOutlet>("head::after");

// HTTP Client
builder.Services.AddScoped(sp => new HttpClient
{
    BaseAddress = new Uri(builder.Configuration["ApiBaseUrl"] ?? builder.HostEnvironment.BaseAddress)
});

// API Clients
builder.Services.AddScoped<WorkItemApiClient>();
builder.Services.AddScoped<SprintApiClient>();
builder.Services.AddScoped<CommentApiClient>();
builder.Services.AddScoped<TimeEntryApiClient>();

// WebSocket
builder.Services.AddScoped(sp =>
{
    var config = sp.GetRequiredService<IConfiguration>();
    var token = GetJwtToken(); // From localStorage or auth service
    return new ProjectManagementWebSocketClient(
        config["WebSocketUrl"] ?? "ws://localhost:3000/ws",
        token
    );
});

// State Management
builder.Services.AddScoped<ProjectState>();
builder.Services.AddScoped<ProjectStateManager>();

// Radzen Services
builder.Services.AddRadzenComponents();

await builder.Build().RunAsync();
```

```razor
<!-- App.razor -->
<Router AppAssembly="@typeof(App).Assembly">
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

**Dependencies:**
- Microsoft.AspNetCore.Components.WebAssembly
- Microsoft.AspNetCore.Components.WebAssembly.DevServer
- Radzen.Blazor
- ProjectManagement.Components

---

## Plugin Integration

For integration into the coaching SaaS platform:

```csharp
// In host platform's Program.cs
builder.Services.AddProjectManagementPlugin(options =>
{
    options.ApiBaseUrl = "/api/pm";
    options.WebSocketUrl = "/ws";
});

// Host platform provides implementations
builder.Services.AddScoped<IProjectManagementService, PlatformProjectManagementService>();
```

The RCL components can be used directly:

```razor
<!-- In host platform -->
@page "/projects/{projectId}/manage"

<ProjectManagement.Components.Pages.ProjectBoard ProjectId="@ProjectId" />
```

---

## Styling & Theming

```css
/* wwwroot/css/app.css */
@import "_content/Radzen.Blazor/css/material-base.css";

:root {
    --primary-color: #1976d2;
    --secondary-color: #dc004e;
    --background-color: #fafafa;
}

.work-item-card {
    cursor: pointer;
    transition: box-shadow 0.2s;
}

.work-item-card:hover {
    box-shadow: 0 4px 8px rgba(0,0,0,0.2);
}

/* Kanban board swim lanes */
.kanban-lane {
    min-height: 500px;
    background-color: var(--background-color);
    padding: 1rem;
}
```

---

## Testing

### Blazor Component Tests (bUnit)

```csharp
public class WorkItemCardTests : TestContext
{
    [Fact]
    public void WorkItemCard_DisplaysTitle()
    {
        // Arrange
        var workItem = new WorkItem { Title = "Test Task" };

        // Act
        var cut = RenderComponent<WorkItemCard>(parameters => parameters
            .Add(p => p.WorkItem, workItem));

        // Assert
        cut.Find(".work-item-title").TextContent.Should().Be("Test Task");
    }

    [Fact]
    public async Task WorkItemCard_OnClick_InvokesCallback()
    {
        // Arrange
        var clicked = false;
        var workItem = new WorkItem { Title = "Test" };

        var cut = RenderComponent<WorkItemCard>(parameters => parameters
            .Add(p => p.WorkItem, workItem)
            .Add(p => p.OnClick, EventCallback.Factory.Create(this, () => clicked = true)));

        // Act
        cut.Find(".work-item-card").Click();

        // Assert
        clicked.Should().BeTrue();
    }
}
```

---

## Build & Deployment

### Development
```bash
cd frontend/ProjectManagement.Wasm
dotnet watch run
```

### Production Build
```bash
dotnet publish -c Release -o publish
```

### Docker
```dockerfile
FROM mcr.microsoft.com/dotnet/sdk:8.0 AS build
WORKDIR /src
COPY . .
RUN dotnet publish ProjectManagement.Wasm/ProjectManagement.Wasm.csproj -c Release -o /app/publish

FROM nginx:alpine
COPY --from=build /app/publish/wwwroot /usr/share/nginx/html
COPY nginx.conf /etc/nginx/nginx.conf
```

---

## Summary

This architecture provides:
- ✅ **Reusable components** via Razor Class Library
- ✅ **Clean separation** of concerns (Core/Services/Components/Host)
- ✅ **Radzen UI** for professional appearance
- ✅ **Real-time updates** via WebSocket + Protobuf
- ✅ **State management** with reactive updates
- ✅ **Plugin-ready** for SaaS platform integration
- ✅ **Type-safe** protobuf communication
- ✅ **Testable** architecture with bUnit
