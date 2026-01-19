# Session 20.4: State Management

**Parent Plan**: [session-20-plan.md](session-20-plan.md)
**Target**: ~30k tokens
**Prerequisites**: Session 20.3 complete (Resilience patterns)

---

## Scope

**Goal**: Thread-safe state stores with optimistic updates

**Estimated Tokens**: ~30k

### Phase 4.1: Work Item Store

```csharp
// WorkItemStore.cs
namespace ProjectManagement.Services.State;

/// <summary>
/// Thread-safe state management for work items with optimistic updates.
/// </summary>
public sealed class WorkItemStore : IWorkItemStore
{
    private readonly ConcurrentDictionary<Guid, WorkItem> _workItems = new();
    private readonly ConcurrentDictionary<Guid, OptimisticUpdate<WorkItem>> _pendingUpdates = new();
    private readonly IWebSocketClient _client;
    private readonly ILogger<WorkItemStore> _logger;
    private readonly SemaphoreSlim _operationLock = new(1, 1);

    private bool _disposed;

    public event Action? OnChanged;

    public WorkItemStore(
        IWebSocketClient client,
        ILogger<WorkItemStore> logger)
    {
        _client = client;
        _logger = logger;

        _client.OnWorkItemCreated += HandleWorkItemCreated;
        _client.OnWorkItemUpdated += HandleWorkItemUpdated;
        _client.OnWorkItemDeleted += HandleWorkItemDeleted;
    }

    #region Read Operations

    public IReadOnlyList<WorkItem> GetByProject(Guid projectId)
    {
        return _workItems.Values
            .Where(w => w.ProjectId == projectId && !w.IsDeleted)
            .OrderBy(w => w.Position)
            .ToList();
    }

    public WorkItem? GetById(Guid id)
    {
        return _workItems.TryGetValue(id, out var item) && !item.IsDeleted
            ? item
            : null;
    }

    public IReadOnlyList<WorkItem> GetBySprint(Guid sprintId)
    {
        return _workItems.Values
            .Where(w => w.SprintId == sprintId && !w.IsDeleted)
            .OrderBy(w => w.Position)
            .ToList();
    }

    public IReadOnlyList<WorkItem> GetChildren(Guid parentId)
    {
        return _workItems.Values
            .Where(w => w.ParentId == parentId && !w.IsDeleted)
            .OrderBy(w => w.Position)
            .ToList();
    }

    #endregion

    #region Write Operations

    public async Task<WorkItem> CreateAsync(
        CreateWorkItemRequest request,
        CancellationToken ct = default)
    {
        ThrowIfDisposed();

        // Create optimistic item (temporary ID)
        var tempId = Guid.NewGuid();
        var optimistic = new WorkItem
        {
            Id = tempId,
            ItemType = request.ItemType,
            Title = request.Title,
            Description = request.Description,
            ProjectId = request.ProjectId,
            ParentId = request.ParentId,
            Status = WorkItemDefaults.Status,
            Priority = WorkItemDefaults.Priority,
            Position = int.MaxValue, // Will be fixed by server
            CreatedAt = DateTime.UtcNow,
            UpdatedAt = DateTime.UtcNow
        };

        // Apply optimistically
        _workItems[tempId] = optimistic;
        var pendingUpdate = new OptimisticUpdate<WorkItem>(tempId, null, optimistic);
        _pendingUpdates[tempId] = pendingUpdate;
        NotifyChanged();

        try
        {
            var confirmed = await _client.CreateWorkItemAsync(request, ct);

            // Replace temp with confirmed
            _workItems.TryRemove(tempId, out _);
            _workItems[confirmed.Id] = confirmed;
            _pendingUpdates.TryRemove(tempId, out _);

            NotifyChanged();

            _logger.LogDebug("Work item created: {Id}", confirmed.Id);
            return confirmed;
        }
        catch
        {
            // Rollback
            _workItems.TryRemove(tempId, out _);
            _pendingUpdates.TryRemove(tempId, out _);
            NotifyChanged();
            throw;
        }
    }

    public async Task<WorkItem> UpdateAsync(
        UpdateWorkItemRequest request,
        CancellationToken ct = default)
    {
        ThrowIfDisposed();

        var id = request.WorkItemId;

        if (!_workItems.TryGetValue(id, out var current))
        {
            throw new KeyNotFoundException($"Work item not found: {id}");
        }

        // Build optimistic update
        var optimistic = ApplyUpdate(current, request);

        // Apply optimistically
        var previousValue = _workItems[id];
        _workItems[id] = optimistic;
        var pendingUpdate = new OptimisticUpdate<WorkItem>(id, previousValue, optimistic);
        _pendingUpdates[id] = pendingUpdate;
        NotifyChanged();

        try
        {
            var confirmed = await _client.UpdateWorkItemAsync(request, ct);

            // Apply confirmed version
            _workItems[id] = confirmed;
            _pendingUpdates.TryRemove(id, out _);
            NotifyChanged();

            _logger.LogDebug("Work item updated: {Id}", id);
            return confirmed;
        }
        catch
        {
            // Rollback
            _workItems[id] = previousValue;
            _pendingUpdates.TryRemove(id, out _);
            NotifyChanged();
            throw;
        }
    }

    public async Task DeleteAsync(Guid id, CancellationToken ct = default)
    {
        ThrowIfDisposed();

        if (!_workItems.TryGetValue(id, out var current))
        {
            return; // Already deleted
        }

        // Apply optimistic soft delete
        var optimistic = current with { DeletedAt = DateTime.UtcNow };
        var previousValue = _workItems[id];
        _workItems[id] = optimistic;
        var pendingUpdate = new OptimisticUpdate<WorkItem>(id, previousValue, optimistic);
        _pendingUpdates[id] = pendingUpdate;
        NotifyChanged();

        try
        {
            await _client.DeleteWorkItemAsync(id, ct);
            _pendingUpdates.TryRemove(id, out _);

            _logger.LogDebug("Work item deleted: {Id}", id);
        }
        catch
        {
            // Rollback
            _workItems[id] = previousValue;
            _pendingUpdates.TryRemove(id, out _);
            NotifyChanged();
            throw;
        }
    }

    public async Task RefreshAsync(Guid projectId, CancellationToken ct = default)
    {
        ThrowIfDisposed();

        var items = await _client.GetWorkItemsAsync(projectId, null, ct);

        // Remove old items for this project
        var toRemove = _workItems.Values
            .Where(w => w.ProjectId == projectId)
            .Select(w => w.Id)
            .ToList();

        foreach (var id in toRemove)
        {
            _workItems.TryRemove(id, out _);
        }

        // Add new items
        foreach (var item in items)
        {
            _workItems[item.Id] = item;
        }

        NotifyChanged();

        _logger.LogDebug("Refreshed {Count} work items for project {ProjectId}",
            items.Count, projectId);
    }

    #endregion

    #region Event Handlers

    private void HandleWorkItemCreated(WorkItem item)
    {
        // Don't overwrite pending updates
        if (_pendingUpdates.ContainsKey(item.Id))
            return;

        _workItems[item.Id] = item;
        NotifyChanged();

        _logger.LogDebug("Received work item created: {Id}", item.Id);
    }

    private void HandleWorkItemUpdated(WorkItem item, IReadOnlyList<FieldChange> changes)
    {
        // Don't overwrite pending updates
        if (_pendingUpdates.ContainsKey(item.Id))
            return;

        _workItems[item.Id] = item;
        NotifyChanged();

        _logger.LogDebug("Received work item updated: {Id}, changes: {Changes}",
            item.Id, string.Join(", ", changes.Select(c => c.FieldName)));
    }

    private void HandleWorkItemDeleted(Guid id)
    {
        // Don't process if we have a pending update
        if (_pendingUpdates.ContainsKey(id))
            return;

        if (_workItems.TryGetValue(id, out var item))
        {
            _workItems[id] = item with { DeletedAt = DateTime.UtcNow };
            NotifyChanged();
        }

        _logger.LogDebug("Received work item deleted: {Id}", id);
    }

    #endregion

    #region Helpers

    private static WorkItem ApplyUpdate(WorkItem current, UpdateWorkItemRequest request)
    {
        return current with
        {
            Title = request.Title ?? current.Title,
            Description = request.Description ?? current.Description,
            Status = request.Status ?? current.Status,
            Priority = request.Priority ?? current.Priority,
            AssigneeId = request.AssigneeId ?? current.AssigneeId,
            SprintId = request.SprintId ?? current.SprintId,
            StoryPoints = request.StoryPoints ?? current.StoryPoints,
            Position = request.Position ?? current.Position,
            UpdatedAt = DateTime.UtcNow,
            Version = current.Version + 1
        };
    }

    private void NotifyChanged()
    {
        try
        {
            OnChanged?.Invoke();
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Error in OnChanged handler");
        }
    }

    private void ThrowIfDisposed()
    {
        ObjectDisposedException.ThrowIf(_disposed, this);
    }

    #endregion

    public void Dispose()
    {
        if (_disposed) return;
        _disposed = true;

        _client.OnWorkItemCreated -= HandleWorkItemCreated;
        _client.OnWorkItemUpdated -= HandleWorkItemUpdated;
        _client.OnWorkItemDeleted -= HandleWorkItemDeleted;

        _operationLock.Dispose();
    }
}
```

### Phase 4.2: Optimistic Update Tracking

```csharp
// OptimisticUpdate.cs
namespace ProjectManagement.Services.State;

/// <summary>
/// Tracks a pending optimistic update for rollback capability.
/// </summary>
internal sealed record OptimisticUpdate<T>(
    Guid EntityId,
    T? OriginalValue,
    T OptimisticValue)
{
    public DateTime CreatedAt { get; } = DateTime.UtcNow;

    public bool IsCreate => OriginalValue is null;
}
```

### Phase 4.3: Sprint Store

```csharp
// SprintStore.cs
namespace ProjectManagement.Services.State;

/// <summary>
/// Thread-safe state management for sprints.
/// Follows same pattern as WorkItemStore.
/// </summary>
public sealed class SprintStore : ISprintStore
{
    private readonly ConcurrentDictionary<Guid, Sprint> _sprints = new();
    private readonly IWebSocketClient _client;
    private readonly ILogger<SprintStore> _logger;

    private bool _disposed;

    public event Action? OnChanged;

    public SprintStore(
        IWebSocketClient client,
        ILogger<SprintStore> logger)
    {
        _client = client;
        _logger = logger;

        // TODO: Wire up sprint events when backend adds them
        // _client.OnSprintCreated += HandleSprintCreated;
        // _client.OnSprintUpdated += HandleSprintUpdated;
    }

    #region Read Operations

    public IReadOnlyList<Sprint> GetByProject(Guid projectId)
    {
        return _sprints.Values
            .Where(s => s.ProjectId == projectId && !s.IsDeleted)
            .OrderBy(s => s.StartDate)
            .ToList();
    }

    public Sprint? GetById(Guid id)
    {
        return _sprints.TryGetValue(id, out var sprint) && !sprint.IsDeleted
            ? sprint
            : null;
    }

    public Sprint? GetActiveSprint(Guid projectId)
    {
        return _sprints.Values
            .FirstOrDefault(s => s.ProjectId == projectId
                && s.Status == SprintStatus.Active
                && !s.IsDeleted);
    }

    #endregion

    #region Write Operations

    public async Task<Sprint> CreateAsync(
        CreateSprintRequest request,
        CancellationToken ct = default)
    {
        ThrowIfDisposed();

        // TODO: Call _client.CreateSprintAsync when backend handler is implemented
        // For now, create locally (will be replaced in Session 50)
        var sprint = new Sprint
        {
            Id = Guid.NewGuid(),
            ProjectId = request.ProjectId,
            Name = request.Name,
            Goal = request.Goal,
            StartDate = request.StartDate,
            EndDate = request.EndDate,
            Status = SprintStatus.Planned,
            CreatedAt = DateTime.UtcNow,
            UpdatedAt = DateTime.UtcNow,
            CreatedBy = Guid.Empty, // Will be set by server
            UpdatedBy = Guid.Empty
        };

        _sprints[sprint.Id] = sprint;
        NotifyChanged();

        _logger.LogDebug("Sprint created locally: {Id}", sprint.Id);
        return sprint;
    }

    public async Task<Sprint> UpdateAsync(
        UpdateSprintRequest request,
        CancellationToken ct = default)
    {
        ThrowIfDisposed();

        if (!_sprints.TryGetValue(request.SprintId, out var current))
        {
            throw new KeyNotFoundException($"Sprint not found: {request.SprintId}");
        }

        var updated = current with
        {
            Name = request.Name ?? current.Name,
            Goal = request.Goal ?? current.Goal,
            StartDate = request.StartDate ?? current.StartDate,
            EndDate = request.EndDate ?? current.EndDate,
            UpdatedAt = DateTime.UtcNow
        };

        _sprints[request.SprintId] = updated;
        NotifyChanged();

        return updated;
    }

    public async Task<Sprint> StartSprintAsync(Guid sprintId, CancellationToken ct = default)
    {
        ThrowIfDisposed();

        if (!_sprints.TryGetValue(sprintId, out var current))
        {
            throw new KeyNotFoundException($"Sprint not found: {sprintId}");
        }

        if (current.Status != SprintStatus.Planned)
        {
            throw new InvalidOperationException($"Cannot start sprint in {current.Status} state");
        }

        // Check for existing active sprint in same project
        var activeSprint = GetActiveSprint(current.ProjectId);
        if (activeSprint != null)
        {
            throw new InvalidOperationException(
                $"Project already has an active sprint: {activeSprint.Name}");
        }

        var started = current with
        {
            Status = SprintStatus.Active,
            UpdatedAt = DateTime.UtcNow
        };

        _sprints[sprintId] = started;
        NotifyChanged();

        return started;
    }

    public async Task<Sprint> CompleteSprintAsync(Guid sprintId, CancellationToken ct = default)
    {
        ThrowIfDisposed();

        if (!_sprints.TryGetValue(sprintId, out var current))
        {
            throw new KeyNotFoundException($"Sprint not found: {sprintId}");
        }

        if (current.Status != SprintStatus.Active)
        {
            throw new InvalidOperationException($"Cannot complete sprint in {current.Status} state");
        }

        var completed = current with
        {
            Status = SprintStatus.Completed,
            UpdatedAt = DateTime.UtcNow
        };

        _sprints[sprintId] = completed;
        NotifyChanged();

        return completed;
    }

    public async Task DeleteAsync(Guid id, CancellationToken ct = default)
    {
        ThrowIfDisposed();

        if (!_sprints.TryGetValue(id, out var current))
        {
            return; // Already deleted
        }

        var deleted = current with { DeletedAt = DateTime.UtcNow };
        _sprints[id] = deleted;
        NotifyChanged();
    }

    public async Task RefreshAsync(Guid projectId, CancellationToken ct = default)
    {
        ThrowIfDisposed();

        // TODO: Call _client.GetSprintsAsync when backend handler is implemented
        // For now, this is a no-op (will be replaced in Session 50)
        _logger.LogDebug("Sprint refresh for project {ProjectId} - not yet implemented", projectId);
    }

    #endregion

    #region Helpers

    private void NotifyChanged()
    {
        try
        {
            OnChanged?.Invoke();
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Error in OnChanged handler");
        }
    }

    private void ThrowIfDisposed()
    {
        ObjectDisposedException.ThrowIf(_disposed, this);
    }

    #endregion

    public void Dispose()
    {
        if (_disposed) return;
        _disposed = true;
    }
}
```

### Phase 4.4: App State Container

```csharp
// AppState.cs
namespace ProjectManagement.Services.State;

/// <summary>
/// Root state container for the application.
/// Provides centralized access to all stores.
/// </summary>
public sealed class AppState : IDisposable
{
    private readonly IWebSocketClient _client;
    private readonly ILogger<AppState> _logger;

    public IWorkItemStore WorkItems { get; }
    public ISprintStore Sprints { get; }
    public IConnectionHealth ConnectionHealth => _client.Health;
    public ConnectionState ConnectionState => _client.State;

    public event Action? OnStateChanged;
    public event Action<ConnectionState>? OnConnectionStateChanged;

    private bool _disposed;

    public AppState(
        IWebSocketClient client,
        IWorkItemStore workItems,
        ISprintStore sprints,
        ILogger<AppState> logger)
    {
        _client = client;
        _logger = logger;

        WorkItems = workItems;
        Sprints = sprints;

        // Forward events
        _client.OnStateChanged += state =>
        {
            OnConnectionStateChanged?.Invoke(state);
            OnStateChanged?.Invoke();
        };

        workItems.OnChanged += () => OnStateChanged?.Invoke();
        sprints.OnChanged += () => OnStateChanged?.Invoke();
    }

    /// <summary>
    /// Initialize state by connecting and loading initial data.
    /// </summary>
    public async Task InitializeAsync(CancellationToken ct = default)
    {
        ThrowIfDisposed();

        _logger.LogInformation("Initializing application state");

        await _client.ConnectAsync(ct);

        _logger.LogInformation("Application state initialized");
    }

    /// <summary>
    /// Load data for a specific project.
    /// </summary>
    public async Task LoadProjectAsync(Guid projectId, CancellationToken ct = default)
    {
        ThrowIfDisposed();

        _logger.LogInformation("Loading project {ProjectId}", projectId);

        // Subscribe to project updates
        await _client.SubscribeAsync([projectId], ct);

        // Load initial data
        await WorkItems.RefreshAsync(projectId, ct);
        await Sprints.RefreshAsync(projectId, ct);

        _logger.LogInformation("Project {ProjectId} loaded", projectId);
    }

    private void ThrowIfDisposed()
    {
        ObjectDisposedException.ThrowIf(_disposed, this);
    }

    public void Dispose()
    {
        if (_disposed) return;
        _disposed = true;

        if (WorkItems is IDisposable workItemsDisposable)
            workItemsDisposable.Dispose();
        if (Sprints is IDisposable sprintsDisposable)
            sprintsDisposable.Dispose();
    }
}
```

### Files Summary for Sub-Session 20.4

| File | Purpose |
|------|---------|
| `WorkItemStore.cs` | Work item state with optimistic updates |
| `SprintStore.cs` | Sprint state management (stub - full impl in Session 50) |
| `OptimisticUpdate.cs` | Pending update tracking |
| `AppState.cs` | Root state container |
| **Total** | **4 files** |

> **Note**: `CommentStore.cs` deferred to Session 50 (Sprints & Comments) to avoid stub proliferation.

### Success Criteria for 20.4

- [ ] Thread-safe with ConcurrentDictionary
- [ ] Optimistic updates apply immediately
- [ ] Rollback works on server rejection
- [ ] Server broadcasts update local state
- [ ] Change notifications fire correctly
- [ ] Proper disposal of resources

---

