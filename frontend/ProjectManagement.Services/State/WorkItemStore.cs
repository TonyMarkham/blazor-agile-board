using System.Collections.Concurrent;
using Microsoft.Extensions.Logging;
using ProjectManagement.Core.Exceptions;
using ProjectManagement.Core.Interfaces;
using ProjectManagement.Core.Models;
using ProjectManagement.Services.Notifications;

namespace ProjectManagement.Services.State;

/// <summary>
///     Thread-safe state management for work items with optimistic updates.
///     Uses ConcurrentDictionary for thread-safe access without explicit locks.
/// </summary>
public sealed class WorkItemStore : IWorkItemStore
{
    private readonly IWebSocketClient _client;
    private readonly IToastService _toast;
    private readonly ILogger<WorkItemStore> _logger;
    private readonly ConcurrentDictionary<Guid, OptimisticUpdate<WorkItem>> _pendingUpdates = new();
    private readonly ConcurrentDictionary<Guid, WorkItem> _workItems = new();

    private bool _disposed;

    public WorkItemStore(
        IWebSocketClient client,
        IToastService toast,
        ILogger<WorkItemStore> logger)
    {
        _client = client ?? throw new ArgumentNullException(nameof(client));
        _toast = toast ?? throw new ArgumentNullException(nameof(toast));
        _logger = logger ?? throw new ArgumentNullException(nameof(logger));

        _client.OnWorkItemCreated += HandleWorkItemCreated;
        _client.OnWorkItemUpdated += HandleWorkItemUpdated;
        _client.OnWorkItemDeleted += HandleWorkItemDeleted;
    }

    public event Action? OnChanged;

    public void Dispose()
    {
        if (_disposed) return;
        _disposed = true;

        _client.OnWorkItemCreated -= HandleWorkItemCreated;
        _client.OnWorkItemUpdated -= HandleWorkItemUpdated;
        _client.OnWorkItemDeleted -= HandleWorkItemDeleted;
    }

    #region Read Operations

    public IReadOnlyList<WorkItem> GetByProject(Guid projectId)
    {
        return _workItems.Values
            .Where(w => w.ProjectId == projectId && w.DeletedAt == null)
            .OrderBy(w => w.Position)
            .ToList();
    }

    public WorkItem? GetById(Guid id)
    {
        return _workItems.TryGetValue(id, out var item) && item.DeletedAt == null
            ? item
            : null;
    }

    public IReadOnlyList<WorkItem> GetBySprint(Guid sprintId)
    {
        return _workItems.Values
            .Where(w => w.SprintId == sprintId && w.DeletedAt == null)
            .OrderBy(w => w.Position)
            .ToList();
    }

    public IReadOnlyList<WorkItem> GetChildren(Guid parentId)
    {
        return _workItems.Values
            .Where(w => w.ParentId == parentId && w.DeletedAt == null)
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
            Status = request.Status, // Use request status (defaults to "backlog")                                    
            Priority = "medium",
            Position = int.MaxValue, // Will be fixed by server                                                       
            Version = 1,
            CreatedAt = DateTime.UtcNow,
            UpdatedAt = DateTime.UtcNow,
            CreatedBy = Guid.Empty, // Will be set by server                                                          
            UpdatedBy = Guid.Empty // Will be set by server                                                          
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

            _toast.ShowSuccess($"Created \"{confirmed.Title}\"");
            _logger.LogDebug("Work item created: {Id}", confirmed.Id);
            return confirmed;
        }
        catch (ValidationException ex)
        {
            _workItems.TryRemove(tempId, out _);
            _pendingUpdates.TryRemove(tempId, out _);
            NotifyChanged();
            _toast.ShowError(ex.UserMessage, "Validation Error");
            throw;
        }
        catch (Exception ex)
        {
            _workItems.TryRemove(tempId, out _);
            _pendingUpdates.TryRemove(tempId, out _);
            NotifyChanged();
            _logger.LogError(ex, "Failed to create work item");
            _toast.ShowError("Failed to create work item. Please try again.");
            throw;
        }
    }

    public async Task<WorkItem> UpdateAsync(
        UpdateWorkItemRequest request,
        CancellationToken ct = default)
    {
        ThrowIfDisposed();

        var id = request.WorkItemId;

        if (!_workItems.TryGetValue(id, out var current)) throw new KeyNotFoundException($"Work item not found: {id}");

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

            _toast.ShowSuccess("Saved changes");
            _logger.LogDebug("Work item updated: {Id}", id);
            return confirmed;
        }
        catch (ValidationException ex)
        {
            _workItems[id] = previousValue;
            _pendingUpdates.TryRemove(id, out _);
            NotifyChanged();
            _toast.ShowError(ex.UserMessage, "Validation Error");
            throw;
        }
        catch (VersionConflictException ex)
        {
            _workItems[id] = previousValue;
            _pendingUpdates.TryRemove(id, out _);
            NotifyChanged();
            _toast.ShowError(ex.UserMessage, "Conflict");
            throw;
        }
        catch (Exception ex)
        {
            _workItems[id] = previousValue;
            _pendingUpdates.TryRemove(id, out _);
            NotifyChanged();
            _logger.LogError(ex, "Failed to update work item {Id}", id);
            _toast.ShowError("Failed to save changes. Please try again.");
            throw;
        }
    }

    public async Task DeleteAsync(Guid id, CancellationToken ct = default)
    {
        ThrowIfDisposed();

        if (!_workItems.TryGetValue(id,
                out var current))
            return; // Already deleted                                                                                

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

            _toast.ShowSuccess($"Deleted \"{current.Title}\"");
            _logger.LogDebug("Work item deleted: {Id}", id);
        }
        catch (ValidationException ex)
        {
            _workItems[id] = previousValue;
            _pendingUpdates.TryRemove(id, out _);
            NotifyChanged();
            _toast.ShowError(ex.UserMessage, "Validation Error");
            throw;
        }
        catch (Exception ex)
        {
            _workItems[id] = previousValue;
            _pendingUpdates.TryRemove(id, out _);
            NotifyChanged();
            _logger.LogError(ex, "Failed to delete work item {Id}", id);
            _toast.ShowError("Failed to delete work item. Please try again.");
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

        foreach (var id in toRemove) _workItems.TryRemove(id, out _);

        // Add new items                                                                                              
        foreach (var item in items) _workItems[item.Id] = item;

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
            ParentId = request.UpdateParent ? request.ParentId : current.ParentId,
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

    public bool IsPending(Guid id) => _pendingUpdates.ContainsKey(id);
}