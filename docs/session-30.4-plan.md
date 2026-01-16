# Session 30.4: Frontend Foundation (Core + Services)

> **⚠️ CRITICAL**: The contents of [`CRITICAL_OPERATING_CONSTRAINTS.md`](../CRITICAL_OPERATING_CONSTRAINTS.md) apply to this implementation session.
> - **Teaching Mode**: Do NOT write/edit files unless explicitly asked to "implement"
> - **Production-Grade**: No shortcuts, no TODOs, comprehensive error handling
> - **Plan First**: Read entire step, identify sub-tasks, present approach before coding

---

## Overview

Set up the Blazor solution structure, create domain models, protobuf mapping, WebSocket client with request/response correlation, and state management.

**Estimated Files**: 12
**Dependencies**: Session 30.3 complete (backend handlers work)

---

## Phase 1: Solution Setup

### 1.1 Solution File

**File**: `frontend/ProjectManagement.sln`

```xml
Microsoft Visual Studio Solution File, Format Version 12.00
# Visual Studio Version 17
VisualStudioVersion = 17.0.31903.59
MinimumVisualStudioVersion = 10.0.40219.1
Project("{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}") = "ProjectManagement.Core", "ProjectManagement.Core\ProjectManagement.Core.csproj", "{GUID1}"
EndProject
Project("{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}") = "ProjectManagement.Services", "ProjectManagement.Services\ProjectManagement.Services.csproj", "{GUID2}"
EndProject
Project("{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}") = "ProjectManagement.Components", "ProjectManagement.Components\ProjectManagement.Components.csproj", "{GUID3}"
EndProject
Project("{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}") = "ProjectManagement.Wasm", "ProjectManagement.Wasm\ProjectManagement.Wasm.csproj", "{GUID4}"
EndProject
Global
    GlobalSection(SolutionConfigurationPlatforms) = preSolution
        Debug|Any CPU = Debug|Any CPU
        Release|Any CPU = Release|Any CPU
    EndGlobalSection
    GlobalSection(ProjectConfigurationPlatforms) = postSolution
        {GUID1}.Debug|Any CPU.ActiveCfg = Debug|Any CPU
        {GUID1}.Debug|Any CPU.Build.0 = Debug|Any CPU
        {GUID1}.Release|Any CPU.ActiveCfg = Release|Any CPU
        {GUID1}.Release|Any CPU.Build.0 = Release|Any CPU
        {GUID2}.Debug|Any CPU.ActiveCfg = Debug|Any CPU
        {GUID2}.Debug|Any CPU.Build.0 = Debug|Any CPU
        {GUID2}.Release|Any CPU.ActiveCfg = Release|Any CPU
        {GUID2}.Release|Any CPU.Build.0 = Release|Any CPU
        {GUID3}.Debug|Any CPU.ActiveCfg = Debug|Any CPU
        {GUID3}.Debug|Any CPU.Build.0 = Debug|Any CPU
        {GUID3}.Release|Any CPU.ActiveCfg = Release|Any CPU
        {GUID3}.Release|Any CPU.Build.0 = Release|Any CPU
        {GUID4}.Debug|Any CPU.ActiveCfg = Debug|Any CPU
        {GUID4}.Debug|Any CPU.Build.0 = Debug|Any CPU
        {GUID4}.Release|Any CPU.ActiveCfg = Release|Any CPU
        {GUID4}.Release|Any CPU.Build.0 = Release|Any CPU
    EndGlobalSection
EndGlobal
```

---

## Phase 2: Core Project

### 2.1 Project File

**File**: `frontend/ProjectManagement.Core/ProjectManagement.Core.csproj`

```xml
<Project Sdk="Microsoft.NET.Sdk">

  <PropertyGroup>
    <TargetFramework>net8.0</TargetFramework>
    <ImplicitUsings>enable</ImplicitUsings>
    <Nullable>enable</Nullable>
  </PropertyGroup>

  <ItemGroup>
    <PackageReference Include="Google.Protobuf" Version="3.25.1" />
    <PackageReference Include="Grpc.Tools" Version="2.60.0" PrivateAssets="All" />
  </ItemGroup>

  <ItemGroup>
    <Protobuf Include="..\..\proto\messages.proto" GrpcServices="None" />
  </ItemGroup>

</Project>
```

---

### 2.2 WorkItem Model

**File**: `frontend/ProjectManagement.Core/Models/WorkItem.cs`

```csharp
namespace ProjectManagement.Core.Models;

/// <summary>
/// Domain model for work items (Projects, Epics, Stories, Tasks).
/// Matches the backend Rust model exactly.
/// </summary>
public class WorkItem
{
    public Guid Id { get; set; }
    public WorkItemType ItemType { get; set; }
    public string Title { get; set; } = string.Empty;
    public string? Description { get; set; }
    public WorkItemStatus Status { get; set; }
    public WorkItemPriority Priority { get; set; }
    public Guid? ParentId { get; set; }
    public Guid ProjectId { get; set; }
    public Guid? AssigneeId { get; set; }
    public int Position { get; set; }
    public int? StoryPoints { get; set; }
    public int Version { get; set; }
    public DateTimeOffset CreatedAt { get; set; }
    public DateTimeOffset UpdatedAt { get; set; }
    public Guid CreatedBy { get; set; }
    public Guid UpdatedBy { get; set; }
}
```

---

### 2.3 Enums

**File**: `frontend/ProjectManagement.Core/Enums/WorkItemType.cs`

```csharp
namespace ProjectManagement.Core.Enums;

public enum WorkItemType
{
    Project = 0,
    Epic = 1,
    Story = 2,
    Task = 3
}
```

**File**: `frontend/ProjectManagement.Core/Enums/WorkItemStatus.cs`

```csharp
namespace ProjectManagement.Core.Enums;

public enum WorkItemStatus
{
    New = 0,
    Active = 1,
    Resolved = 2,
    Closed = 3
}
```

**File**: `frontend/ProjectManagement.Core/Enums/WorkItemPriority.cs`

```csharp
namespace ProjectManagement.Core.Enums;

public enum WorkItemPriority
{
    Low = 0,
    Medium = 1,
    High = 2,
    Critical = 3
}
```

---

### 2.4 Proto Mapper

**File**: `frontend/ProjectManagement.Core/Mapping/ProtoMapper.cs`

```csharp
using ProjectManagement.Core.Models;
using ProjectManagement.Core.Enums;
using Proto = ProjectManagement.Proto;

namespace ProjectManagement.Core.Mapping;

/// <summary>
/// Maps between protobuf messages and domain models.
/// </summary>
public static class ProtoMapper
{
    public static WorkItem FromProto(Proto.WorkItem proto)
    {
        return new WorkItem
        {
            Id = Guid.Parse(proto.Id),
            ItemType = (WorkItemType)proto.ItemType,
            Title = proto.Title,
            Description = proto.Description,
            Status = (WorkItemStatus)proto.Status,
            Priority = (WorkItemPriority)proto.Priority,
            ParentId = string.IsNullOrEmpty(proto.ParentId) ? null : Guid.Parse(proto.ParentId),
            ProjectId = Guid.Parse(proto.ProjectId),
            AssigneeId = string.IsNullOrEmpty(proto.AssigneeId) ? null : Guid.Parse(proto.AssigneeId),
            Position = proto.Position,
            StoryPoints = proto.StoryPoints == 0 ? null : proto.StoryPoints,
            Version = proto.Version,
            CreatedAt = DateTimeOffset.FromUnixTimeSeconds(proto.CreatedAt),
            UpdatedAt = DateTimeOffset.FromUnixTimeSeconds(proto.UpdatedAt),
            CreatedBy = Guid.Parse(proto.CreatedBy),
            UpdatedBy = Guid.Parse(proto.UpdatedBy),
        };
    }

    public static Proto.WorkItem ToProto(WorkItem model)
    {
        return new Proto.WorkItem
        {
            Id = model.Id.ToString(),
            ItemType = (int)model.ItemType,
            Title = model.Title,
            Description = model.Description ?? string.Empty,
            Status = (int)model.Status,
            Priority = (int)model.Priority,
            ParentId = model.ParentId?.ToString() ?? string.Empty,
            ProjectId = model.ProjectId.ToString(),
            AssigneeId = model.AssigneeId?.ToString() ?? string.Empty,
            Position = model.Position,
            StoryPoints = model.StoryPoints ?? 0,
            Version = model.Version,
            CreatedAt = model.CreatedAt.ToUnixTimeSeconds(),
            UpdatedAt = model.UpdatedAt.ToUnixTimeSeconds(),
            CreatedBy = model.CreatedBy.ToString(),
            UpdatedBy = model.UpdatedBy.ToString(),
        };
    }

    public static Proto.CreateWorkItemRequest ToCreateRequest(
        WorkItemType itemType,
        string title,
        string? description,
        Guid projectId,
        Guid? parentId)
    {
        return new Proto.CreateWorkItemRequest
        {
            ItemType = (int)itemType,
            Title = title,
            Description = description ?? string.Empty,
            ProjectId = projectId.ToString(),
            ParentId = parentId?.ToString() ?? string.Empty,
        };
    }

    public static Proto.UpdateWorkItemRequest ToUpdateRequest(
        Guid workItemId,
        int expectedVersion,
        string? title = null,
        string? description = null,
        WorkItemStatus? status = null,
        WorkItemPriority? priority = null,
        Guid? assigneeId = null,
        int? storyPoints = null,
        int? position = null)
    {
        var request = new Proto.UpdateWorkItemRequest
        {
            WorkItemId = workItemId.ToString(),
            ExpectedVersion = expectedVersion,
        };

        if (title != null) request.Title = title;
        if (description != null) request.Description = description;
        if (status.HasValue) request.Status = (int)status.Value;
        if (priority.HasValue) request.Priority = (int)priority.Value;
        if (assigneeId.HasValue) request.AssigneeId = assigneeId.Value.ToString();
        if (storyPoints.HasValue) request.StoryPoints = storyPoints.Value;
        if (position.HasValue) request.Position = position.Value;

        return request;
    }

    public static Proto.DeleteWorkItemRequest ToDeleteRequest(Guid workItemId)
    {
        return new Proto.DeleteWorkItemRequest
        {
            WorkItemId = workItemId.ToString(),
        };
    }

    public static Proto.GetWorkItemsRequest ToGetWorkItemsRequest(
        Guid projectId,
        long? sinceTimestamp = null)
    {
        var request = new Proto.GetWorkItemsRequest
        {
            ProjectId = projectId.ToString(),
        };

        if (sinceTimestamp.HasValue)
        {
            request.SinceTimestamp = sinceTimestamp.Value;
        }

        return request;
    }
}
```

---

## Phase 3: Services Project

### 3.1 Project File

**File**: `frontend/ProjectManagement.Services/ProjectManagement.Services.csproj`

```xml
<Project Sdk="Microsoft.NET.Sdk">

  <PropertyGroup>
    <TargetFramework>net8.0</TargetFramework>
    <ImplicitUsings>enable</ImplicitUsings>
    <Nullable>enable</Nullable>
  </PropertyGroup>

  <ItemGroup>
    <ProjectReference Include="..\ProjectManagement.Core\ProjectManagement.Core.csproj" />
  </ItemGroup>

  <ItemGroup>
    <PackageReference Include="Microsoft.Extensions.Logging.Abstractions" Version="8.0.0" />
  </ItemGroup>

</Project>
```

---

### 3.2 WebSocket Client with Correlation

**File**: `frontend/ProjectManagement.Services/WebSocket/ProjectManagementWebSocketClient.cs`

```csharp
using System.Collections.Concurrent;
using System.Net.WebSockets;
using System.Threading.Channels;
using Google.Protobuf;
using Microsoft.Extensions.Logging;
using Proto = ProjectManagement.Proto;

namespace ProjectManagement.Services.WebSocket;

/// <summary>
/// WebSocket client for communicating with the backend.
/// Supports request/response correlation via message_id.
/// </summary>
public class ProjectManagementWebSocketClient : IAsyncDisposable
{
    private readonly ILogger<ProjectManagementWebSocketClient> _logger;
    private readonly ClientWebSocket _webSocket = new();
    private readonly ConcurrentDictionary<string, TaskCompletionSource<Proto.WebSocketMessage>> _pendingRequests = new();
    private readonly Channel<Proto.WebSocketMessage> _incoming = Channel.CreateUnbounded<Proto.WebSocketMessage>();
    private readonly Channel<Proto.WebSocketMessage> _outgoing = Channel.CreateUnbounded<Proto.WebSocketMessage>();
    private readonly CancellationTokenSource _cts = new();

    private Task? _receiveTask;
    private Task? _sendTask;

    public event Func<Proto.WebSocketMessage, Task>? OnBroadcastReceived;
    public event Func<Task>? OnDisconnected;
    public event Func<Exception, Task>? OnError;

    public bool IsConnected => _webSocket.State == WebSocketState.Open;

    public ProjectManagementWebSocketClient(ILogger<ProjectManagementWebSocketClient> logger)
    {
        _logger = logger;
    }

    public async Task ConnectAsync(Uri serverUri, string jwtToken, CancellationToken ct = default)
    {
        _webSocket.Options.SetRequestHeader("Authorization", $"Bearer {jwtToken}");

        await _webSocket.ConnectAsync(serverUri, ct);
        _logger.LogInformation("Connected to WebSocket server at {Uri}", serverUri);

        // Start receive and send loops
        _receiveTask = ReceiveLoopAsync(_cts.Token);
        _sendTask = SendLoopAsync(_cts.Token);
    }

    /// <summary>
    /// Send a request and wait for the correlated response.
    /// </summary>
    public async Task<TResponse> SendRequestAsync<TResponse>(
        Proto.WebSocketMessage request,
        Func<Proto.WebSocketMessage, TResponse?> extractResponse,
        CancellationToken ct = default)
        where TResponse : class
    {
        if (string.IsNullOrEmpty(request.MessageId))
        {
            request.MessageId = Guid.NewGuid().ToString();
        }

        var tcs = new TaskCompletionSource<Proto.WebSocketMessage>(TaskCreationOptions.RunContinuationsAsynchronously);
        _pendingRequests[request.MessageId] = tcs;

        try
        {
            await _outgoing.Writer.WriteAsync(request, ct);

            using var cts = CancellationTokenSource.CreateLinkedTokenSource(ct);
            cts.CancelAfter(TimeSpan.FromSeconds(30)); // Request timeout

            var response = await tcs.Task.WaitAsync(cts.Token);

            // Check for error response
            if (response.PayloadCase == Proto.WebSocketMessage.PayloadOneofCase.Error)
            {
                throw new WebSocketRequestException(
                    response.Error.Code,
                    response.Error.Message,
                    response.Error.Field);
            }

            var result = extractResponse(response);
            if (result == null)
            {
                throw new InvalidOperationException($"Unexpected response type: {response.PayloadCase}");
            }

            return result;
        }
        finally
        {
            _pendingRequests.TryRemove(request.MessageId, out _);
        }
    }

    /// <summary>
    /// Subscribe to a project to receive broadcasts.
    /// </summary>
    public async Task SubscribeAsync(Guid projectId, CancellationToken ct = default)
    {
        var request = new Proto.WebSocketMessage
        {
            MessageId = Guid.NewGuid().ToString(),
            Timestamp = DateTimeOffset.UtcNow.ToUnixTimeSeconds(),
            Subscribe = new Proto.Subscribe
            {
                ProjectId = projectId.ToString(),
            },
        };

        await SendRequestAsync(
            request,
            msg => msg.PayloadCase == Proto.WebSocketMessage.PayloadOneofCase.SubscribeAck ? msg.SubscribeAck : null,
            ct);
    }

    /// <summary>
    /// Unsubscribe from a project.
    /// </summary>
    public async Task UnsubscribeAsync(Guid projectId, CancellationToken ct = default)
    {
        var message = new Proto.WebSocketMessage
        {
            MessageId = Guid.NewGuid().ToString(),
            Timestamp = DateTimeOffset.UtcNow.ToUnixTimeSeconds(),
            Unsubscribe = new Proto.Unsubscribe
            {
                ProjectId = projectId.ToString(),
            },
        };

        await _outgoing.Writer.WriteAsync(message, ct);
    }

    private async Task ReceiveLoopAsync(CancellationToken ct)
    {
        var buffer = new byte[4096];
        var messageBuffer = new MemoryStream();

        try
        {
            while (!ct.IsCancellationRequested && _webSocket.State == WebSocketState.Open)
            {
                var result = await _webSocket.ReceiveAsync(buffer, ct);

                if (result.MessageType == WebSocketMessageType.Close)
                {
                    _logger.LogInformation("WebSocket closed by server");
                    break;
                }

                if (result.MessageType == WebSocketMessageType.Binary)
                {
                    messageBuffer.Write(buffer, 0, result.Count);

                    if (result.EndOfMessage)
                    {
                        var data = messageBuffer.ToArray();
                        messageBuffer.SetLength(0);

                        var message = Proto.WebSocketMessage.Parser.ParseFrom(data);
                        await HandleReceivedMessageAsync(message);
                    }
                }
            }
        }
        catch (OperationCanceledException)
        {
            // Expected on shutdown
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Error in receive loop");
            if (OnError != null) await OnError(ex);
        }
        finally
        {
            if (OnDisconnected != null) await OnDisconnected();
        }
    }

    private async Task HandleReceivedMessageAsync(Proto.WebSocketMessage message)
    {
        // Check if this is a response to a pending request
        if (_pendingRequests.TryGetValue(message.MessageId, out var tcs))
        {
            tcs.TrySetResult(message);
        }
        else
        {
            // Broadcast event - route to subscribers
            if (OnBroadcastReceived != null)
            {
                await OnBroadcastReceived(message);
            }
        }
    }

    private async Task SendLoopAsync(CancellationToken ct)
    {
        try
        {
            await foreach (var message in _outgoing.Reader.ReadAllAsync(ct))
            {
                var data = message.ToByteArray();
                await _webSocket.SendAsync(data, WebSocketMessageType.Binary, true, ct);
            }
        }
        catch (OperationCanceledException)
        {
            // Expected on shutdown
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Error in send loop");
            if (OnError != null) await OnError(ex);
        }
    }

    public async ValueTask DisposeAsync()
    {
        _cts.Cancel();

        if (_webSocket.State == WebSocketState.Open)
        {
            try
            {
                await _webSocket.CloseAsync(WebSocketCloseStatus.NormalClosure, "Client closing", CancellationToken.None);
            }
            catch
            {
                // Ignore errors during close
            }
        }

        if (_receiveTask != null) await _receiveTask;
        if (_sendTask != null) await _sendTask;

        _webSocket.Dispose();
        _cts.Dispose();
    }
}

/// <summary>
/// Exception thrown when a WebSocket request fails.
/// </summary>
public class WebSocketRequestException : Exception
{
    public string ErrorCode { get; }
    public string? Field { get; }

    public WebSocketRequestException(string errorCode, string message, string? field = null)
        : base(message)
    {
        ErrorCode = errorCode;
        Field = field;
    }
}
```

---

### 3.3 Work Item State

**File**: `frontend/ProjectManagement.Services/State/WorkItemState.cs`

```csharp
using System.Collections.Concurrent;
using ProjectManagement.Core.Models;

namespace ProjectManagement.Services.State;

/// <summary>
/// Thread-safe state container for work items.
/// Supports version-based conflict resolution.
/// </summary>
public class WorkItemState
{
    private readonly ConcurrentDictionary<Guid, WorkItem> _items = new();
    private long _lastSyncTimestamp;

    public event Action? OnChange;

    /// <summary>
    /// Get all work items.
    /// </summary>
    public IReadOnlyCollection<WorkItem> GetAll() => _items.Values.ToList();

    /// <summary>
    /// Get work items for a specific project.
    /// </summary>
    public IReadOnlyCollection<WorkItem> GetByProject(Guid projectId)
    {
        return _items.Values
            .Where(i => i.ProjectId == projectId)
            .OrderBy(i => i.Position)
            .ToList();
    }

    /// <summary>
    /// Get work items by parent.
    /// </summary>
    public IReadOnlyCollection<WorkItem> GetChildren(Guid parentId)
    {
        return _items.Values
            .Where(i => i.ParentId == parentId)
            .OrderBy(i => i.Position)
            .ToList();
    }

    /// <summary>
    /// Get a work item by ID.
    /// </summary>
    public WorkItem? GetById(Guid id)
    {
        return _items.TryGetValue(id, out var item) ? item : null;
    }

    /// <summary>
    /// Upsert a work item, respecting version ordering.
    /// Only updates if the new version is >= existing version.
    /// </summary>
    public void Upsert(WorkItem item)
    {
        _items.AddOrUpdate(
            item.Id,
            item,
            (_, existing) => item.Version >= existing.Version ? item : existing
        );
        OnChange?.Invoke();
    }

    /// <summary>
    /// Remove a work item (for delete events).
    /// </summary>
    public void Remove(Guid id)
    {
        _items.TryRemove(id, out _);
        OnChange?.Invoke();
    }

    /// <summary>
    /// Clear all items.
    /// </summary>
    public void Clear()
    {
        _items.Clear();
        OnChange?.Invoke();
    }

    /// <summary>
    /// Get the last sync timestamp for incremental sync.
    /// </summary>
    public long GetSyncTimestamp() => _lastSyncTimestamp;

    /// <summary>
    /// Set the sync timestamp after receiving data.
    /// </summary>
    public void SetSyncTimestamp(long timestamp) => _lastSyncTimestamp = timestamp;
}
```

---

### 3.4 Project State Manager

**File**: `frontend/ProjectManagement.Services/State/ProjectStateManager.cs`

```csharp
using Microsoft.Extensions.Logging;
using ProjectManagement.Core.Mapping;
using ProjectManagement.Core.Models;
using ProjectManagement.Services.WebSocket;
using Proto = ProjectManagement.Proto;

namespace ProjectManagement.Services.State;

/// <summary>
/// Manages state for a single project, handling WebSocket communication
/// and maintaining local state synchronized with the server.
/// </summary>
public class ProjectStateManager : IAsyncDisposable
{
    private readonly ILogger<ProjectStateManager> _logger;
    private readonly ProjectManagementWebSocketClient _client;
    private readonly WorkItemState _state;
    private readonly Uri _serverUri;
    private readonly string _jwtToken;

    private Guid? _currentProjectId;
    private readonly ExponentialBackoff _backoff = new();
    private bool _disposed;

    public WorkItemState State => _state;

    public event Action? OnStateChanged
    {
        add => _state.OnChange += value;
        remove => _state.OnChange -= value;
    }

    public ProjectStateManager(
        ILogger<ProjectStateManager> logger,
        ILogger<ProjectManagementWebSocketClient> clientLogger,
        Uri serverUri,
        string jwtToken)
    {
        _logger = logger;
        _serverUri = serverUri;
        _jwtToken = jwtToken;
        _client = new ProjectManagementWebSocketClient(clientLogger);
        _state = new WorkItemState();

        _client.OnBroadcastReceived += HandleBroadcastAsync;
        _client.OnDisconnected += HandleDisconnectedAsync;
        _client.OnError += HandleErrorAsync;
    }

    /// <summary>
    /// Connect and load initial data for a project.
    /// </summary>
    public async Task InitializeAsync(Guid projectId, CancellationToken ct = default)
    {
        _currentProjectId = projectId;

        await _client.ConnectAsync(_serverUri, _jwtToken, ct);
        await _client.SubscribeAsync(projectId, ct);
        await LoadInitialDataAsync(projectId, ct);

        _backoff.Reset();
    }

    /// <summary>
    /// Load all work items for the project.
    /// </summary>
    private async Task LoadInitialDataAsync(Guid projectId, CancellationToken ct)
    {
        var request = new Proto.WebSocketMessage
        {
            MessageId = Guid.NewGuid().ToString(),
            Timestamp = DateTimeOffset.UtcNow.ToUnixTimeSeconds(),
            GetWorkItemsRequest = ProtoMapper.ToGetWorkItemsRequest(projectId),
        };

        var response = await _client.SendRequestAsync(
            request,
            msg => msg.PayloadCase == Proto.WebSocketMessage.PayloadOneofCase.WorkItemsList
                ? msg.WorkItemsList
                : null,
            ct);

        foreach (var protoItem in response.WorkItems)
        {
            _state.Upsert(ProtoMapper.FromProto(protoItem));
        }
        _state.SetSyncTimestamp(response.AsOfTimestamp);

        _logger.LogInformation("Loaded {Count} work items for project {ProjectId}",
            response.WorkItems.Count, projectId);
    }

    /// <summary>
    /// Handle incoming broadcast events.
    /// </summary>
    private Task HandleBroadcastAsync(Proto.WebSocketMessage message)
    {
        switch (message.PayloadCase)
        {
            case Proto.WebSocketMessage.PayloadOneofCase.WorkItemCreated:
                var created = ProtoMapper.FromProto(message.WorkItemCreated.WorkItem);
                _state.Upsert(created);
                _logger.LogDebug("Work item created: {Id}", created.Id);
                break;

            case Proto.WebSocketMessage.PayloadOneofCase.WorkItemUpdated:
                var updated = ProtoMapper.FromProto(message.WorkItemUpdated.WorkItem);
                _state.Upsert(updated);
                _logger.LogDebug("Work item updated: {Id}, changes: {Changes}",
                    updated.Id, message.WorkItemUpdated.Changes.Count);
                break;

            case Proto.WebSocketMessage.PayloadOneofCase.WorkItemDeleted:
                var deletedId = Guid.Parse(message.WorkItemDeleted.WorkItemId);
                _state.Remove(deletedId);
                _logger.LogDebug("Work item deleted: {Id}", deletedId);
                break;
        }

        return Task.CompletedTask;
    }

    private async Task HandleDisconnectedAsync()
    {
        _logger.LogWarning("Disconnected from WebSocket server");
        await ReconnectAsync();
    }

    private Task HandleErrorAsync(Exception ex)
    {
        _logger.LogError(ex, "WebSocket error");
        return Task.CompletedTask;
    }

    /// <summary>
    /// Reconnect and sync missed changes.
    /// </summary>
    private async Task ReconnectAsync()
    {
        while (!_disposed && _currentProjectId.HasValue)
        {
            try
            {
                await _client.ConnectAsync(_serverUri, _jwtToken);
                await _client.SubscribeAsync(_currentProjectId.Value);

                // Incremental sync - fetch items since last sync
                var request = new Proto.WebSocketMessage
                {
                    MessageId = Guid.NewGuid().ToString(),
                    Timestamp = DateTimeOffset.UtcNow.ToUnixTimeSeconds(),
                    GetWorkItemsRequest = ProtoMapper.ToGetWorkItemsRequest(
                        _currentProjectId.Value,
                        _state.GetSyncTimestamp()),
                };

                var response = await _client.SendRequestAsync(
                    request,
                    msg => msg.PayloadCase == Proto.WebSocketMessage.PayloadOneofCase.WorkItemsList
                        ? msg.WorkItemsList
                        : null);

                foreach (var protoItem in response.WorkItems)
                {
                    _state.Upsert(ProtoMapper.FromProto(protoItem));
                }
                _state.SetSyncTimestamp(response.AsOfTimestamp);

                _backoff.Reset();
                _logger.LogInformation("Reconnected and synced {Count} items",
                    response.WorkItems.Count);
                break;
            }
            catch (Exception ex)
            {
                _logger.LogWarning(ex, "Reconnection failed, retrying in {Delay}ms",
                    _backoff.CurrentDelayMs);
                await Task.Delay(_backoff.GetNextDelay());
            }
        }
    }

    public async ValueTask DisposeAsync()
    {
        _disposed = true;
        await _client.DisposeAsync();
    }
}

/// <summary>
/// Simple exponential backoff for reconnection.
/// </summary>
public class ExponentialBackoff
{
    private int _currentDelayMs = 1000;
    private const int MaxDelayMs = 30000;

    public int CurrentDelayMs => _currentDelayMs;

    public TimeSpan GetNextDelay()
    {
        var delay = TimeSpan.FromMilliseconds(_currentDelayMs);
        _currentDelayMs = Math.Min(_currentDelayMs * 2, MaxDelayMs);
        return delay;
    }

    public void Reset() => _currentDelayMs = 1000;
}
```

---

### 3.5 Work Item Commands

**File**: `frontend/ProjectManagement.Services/Commands/WorkItemCommands.cs`

```csharp
using ProjectManagement.Core.Enums;
using ProjectManagement.Core.Mapping;
using ProjectManagement.Core.Models;
using ProjectManagement.Services.WebSocket;
using Proto = ProjectManagement.Proto;

namespace ProjectManagement.Services.Commands;

/// <summary>
/// Command methods for work item CRUD operations.
/// </summary>
public class WorkItemCommands
{
    private readonly ProjectManagementWebSocketClient _client;

    public WorkItemCommands(ProjectManagementWebSocketClient client)
    {
        _client = client;
    }

    /// <summary>
    /// Create a new work item.
    /// </summary>
    public async Task<WorkItem> CreateAsync(
        WorkItemType itemType,
        string title,
        string? description,
        Guid projectId,
        Guid? parentId,
        CancellationToken ct = default)
    {
        var request = new Proto.WebSocketMessage
        {
            MessageId = Guid.NewGuid().ToString(),
            Timestamp = DateTimeOffset.UtcNow.ToUnixTimeSeconds(),
            CreateWorkItemRequest = ProtoMapper.ToCreateRequest(
                itemType, title, description, projectId, parentId),
        };

        var response = await _client.SendRequestAsync(
            request,
            msg => msg.PayloadCase == Proto.WebSocketMessage.PayloadOneofCase.WorkItemCreated
                ? msg.WorkItemCreated
                : null,
            ct);

        return ProtoMapper.FromProto(response.WorkItem);
    }

    /// <summary>
    /// Update an existing work item.
    /// </summary>
    public async Task<WorkItem> UpdateAsync(
        Guid workItemId,
        int expectedVersion,
        string? title = null,
        string? description = null,
        WorkItemStatus? status = null,
        WorkItemPriority? priority = null,
        Guid? assigneeId = null,
        int? storyPoints = null,
        int? position = null,
        CancellationToken ct = default)
    {
        var request = new Proto.WebSocketMessage
        {
            MessageId = Guid.NewGuid().ToString(),
            Timestamp = DateTimeOffset.UtcNow.ToUnixTimeSeconds(),
            UpdateWorkItemRequest = ProtoMapper.ToUpdateRequest(
                workItemId, expectedVersion, title, description,
                status, priority, assigneeId, storyPoints, position),
        };

        var response = await _client.SendRequestAsync(
            request,
            msg => msg.PayloadCase == Proto.WebSocketMessage.PayloadOneofCase.WorkItemUpdated
                ? msg.WorkItemUpdated
                : null,
            ct);

        return ProtoMapper.FromProto(response.WorkItem);
    }

    /// <summary>
    /// Delete a work item.
    /// </summary>
    public async Task DeleteAsync(Guid workItemId, CancellationToken ct = default)
    {
        var request = new Proto.WebSocketMessage
        {
            MessageId = Guid.NewGuid().ToString(),
            Timestamp = DateTimeOffset.UtcNow.ToUnixTimeSeconds(),
            DeleteWorkItemRequest = ProtoMapper.ToDeleteRequest(workItemId),
        };

        await _client.SendRequestAsync(
            request,
            msg => msg.PayloadCase == Proto.WebSocketMessage.PayloadOneofCase.WorkItemDeleted
                ? msg.WorkItemDeleted
                : null,
            ct);
    }
}
```

---

## File Summary

| Action | Path |
|--------|------|
| Create | `frontend/ProjectManagement.sln` |
| Create | `frontend/ProjectManagement.Core/ProjectManagement.Core.csproj` |
| Create | `frontend/ProjectManagement.Core/Models/WorkItem.cs` |
| Create | `frontend/ProjectManagement.Core/Enums/WorkItemType.cs` |
| Create | `frontend/ProjectManagement.Core/Enums/WorkItemStatus.cs` |
| Create | `frontend/ProjectManagement.Core/Enums/WorkItemPriority.cs` |
| Create | `frontend/ProjectManagement.Core/Mapping/ProtoMapper.cs` |
| Create | `frontend/ProjectManagement.Services/ProjectManagement.Services.csproj` |
| Create | `frontend/ProjectManagement.Services/WebSocket/ProjectManagementWebSocketClient.cs` |
| Create | `frontend/ProjectManagement.Services/State/WorkItemState.cs` |
| Create | `frontend/ProjectManagement.Services/State/ProjectStateManager.cs` |
| Create | `frontend/ProjectManagement.Services/Commands/WorkItemCommands.cs` |

---

## Verification

```bash
cd frontend

# Build Core project (generates proto)
dotnet build ProjectManagement.Core

# Build Services project
dotnet build ProjectManagement.Services

# Run unit tests (if any)
dotnet test
```

---

## Testing Strategy

At this point, you can write unit tests for:

1. **ProtoMapper**: Verify round-trip conversion between models and proto
2. **WorkItemState**: Test version-based conflict resolution, add/remove/get operations
3. **ExponentialBackoff**: Verify delay progression and reset

Integration tests with the backend will require mocking or a test server.
