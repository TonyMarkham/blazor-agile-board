using System.Collections.Concurrent;
using Microsoft.Extensions.Logging;
using ProjectManagement.Core.Exceptions;
using ProjectManagement.Core.Interfaces;
using ProjectManagement.Core.Models;
using ProjectManagement.Services.Notifications;

namespace ProjectManagement.Services.State;

/// <summary>
///     Thread-safe state management for sprints.
///     Follows same pattern as WorkItemStore.
///     Note: Full WebSocket integration deferred to Session 40 (Sprints & Comments).
/// </summary>
public sealed class SprintStore : ISprintStore
{
    private readonly IWebSocketClient _client;
    private readonly IToastService _toast;
    private readonly ILogger<SprintStore> _logger;
    private readonly ConcurrentDictionary<Guid, Sprint> _sprints = new();
    private readonly ConcurrentDictionary<Guid, bool> _pendingUpdates = new();

    private bool _disposed;

    public SprintStore(
        IWebSocketClient client,
        IToastService toast,
        ILogger<SprintStore> logger)
    {
        _client = client ?? throw new ArgumentNullException(nameof(client));
        _toast = toast ?? throw new ArgumentNullException(nameof(toast));
        _logger = logger ?? throw new ArgumentNullException(nameof(logger));

        // Wire up WebSocket events for real-time updates from other clients
        _client.OnSprintCreated += HandleSprintCreated;
        _client.OnSprintUpdated += HandleSprintUpdated;
        _client.OnSprintDeleted += HandleSprintDeleted;
    }

    public event Action? OnChanged;

    public void Dispose()
    {
        if (_disposed) return;
        _disposed = true;

        _client.OnSprintCreated -= HandleSprintCreated;
        _client.OnSprintUpdated -= HandleSprintUpdated;
        _client.OnSprintDeleted -= HandleSprintDeleted;
    }

    #region Read Operations

    public IReadOnlyList<Sprint> GetByProject(Guid projectId)
    {
        return _sprints.Values
            .Where(s => s.ProjectId == projectId && s.DeletedAt == null)
            .OrderBy(s => s.StartDate)
            .ToList();
    }

    public Sprint? GetById(Guid id)
    {
        return _sprints.TryGetValue(id, out var sprint) && sprint.DeletedAt == null
            ? sprint
            : null;
    }

    public Sprint? GetActiveSprint(Guid projectId)
    {
        return _sprints.Values
            .FirstOrDefault(s => s.ProjectId == projectId
                                 && s.Status == SprintStatus.Active
                                 && s.DeletedAt == null);
    }

    #endregion

    #region Write Operations

    public async Task<Sprint> CreateAsync(
        CreateSprintRequest request,
        CancellationToken ct = default)
    {
        ThrowIfDisposed();

        // Create optimistic sprint with temporary ID
        var tempId = Guid.NewGuid();
        var optimistic = new Sprint
        {
            Id = tempId,
            ProjectId = request.ProjectId,
            Name = request.Name,
            Goal = request.Goal,
            StartDate = request.StartDate,
            EndDate = request.EndDate,
            Status = SprintStatus.Planned,
            Version = 1,
            CreatedAt = DateTime.UtcNow,
            UpdatedAt = DateTime.UtcNow,
            CreatedBy = Guid.Empty,
            UpdatedBy = Guid.Empty
        };

        // Optimistic update
        _sprints[tempId] = optimistic;
        _pendingUpdates[tempId] = true;
        NotifyChanged();

        try
        {
            // Send to server
            var confirmed = await _client.CreateSprintAsync(request, ct);

            // Replace temp with confirmed
            _sprints.TryRemove(tempId, out _);
            _sprints[confirmed.Id] = confirmed;
            _pendingUpdates.TryRemove(tempId, out _);
            NotifyChanged();

            _toast.ShowSuccess($"Created sprint \"{confirmed.Name}\"");
            _logger.LogDebug("Sprint created: {Id}", confirmed.Id);
            return confirmed;
        }
        catch (ValidationException ex)
        {
            _sprints.TryRemove(tempId, out _);
            _pendingUpdates.TryRemove(tempId, out _);
            NotifyChanged();
            _toast.ShowError(ex.UserMessage, "Validation Error");
            throw;
        }
        catch (Exception ex)
        {
            _sprints.TryRemove(tempId, out _);
            _pendingUpdates.TryRemove(tempId, out _);
            NotifyChanged();
            _logger.LogError(ex, "Failed to create sprint");
            _toast.ShowError("Failed to create sprint. Please try again.");
            throw;
        }
    }

    public async Task<Sprint> UpdateAsync(
        UpdateSprintRequest request,
        CancellationToken ct = default)
    {
        ThrowIfDisposed();

        if (!_sprints.TryGetValue(request.SprintId, out var current))
            throw new KeyNotFoundException($"Sprint not found: {request.SprintId}");

        // Create optimistic update
        var optimistic = current with
        {
            Name = request.Name ?? current.Name,
            Goal = request.Goal ?? current.Goal,
            StartDate = request.StartDate ?? current.StartDate,
            EndDate = request.EndDate ?? current.EndDate,
            Status = request.Status ?? current.Status,
            Version = current.Version + 1,
            UpdatedAt = DateTime.UtcNow
        };

        // Store previous value for rollback
        var previousValue = _sprints[request.SprintId];
        _sprints[request.SprintId] = optimistic;
        _pendingUpdates[request.SprintId] = true;
        NotifyChanged();

        try
        {
            var confirmed = await _client.UpdateSprintAsync(request, ct);
            _sprints[request.SprintId] = confirmed;
            _pendingUpdates.TryRemove(request.SprintId, out _);
            NotifyChanged();

            _toast.ShowSuccess("Sprint updated");
            return confirmed;
        }
        catch (ValidationException ex)
        {
            _sprints[request.SprintId] = previousValue;
            _pendingUpdates.TryRemove(request.SprintId, out _);
            NotifyChanged();
            _toast.ShowError(ex.UserMessage, "Validation Error");
            throw;
        }
        catch (VersionConflictException ex)
        {
            _sprints[request.SprintId] = previousValue;
            _pendingUpdates.TryRemove(request.SprintId, out _);
            NotifyChanged();
            _toast.ShowError(ex.UserMessage, "Conflict");
            throw;
        }
        catch (Exception ex)
        {
            _sprints[request.SprintId] = previousValue;
            _pendingUpdates.TryRemove(request.SprintId, out _);
            NotifyChanged();
            _logger.LogError(ex, "Failed to update sprint {Id}", request.SprintId);
            _toast.ShowError("Failed to update sprint. Please try again.");
            throw;
        }
    }

    public Task<Sprint> StartSprintAsync(Guid sprintId, CancellationToken ct = default)
    {
        ThrowIfDisposed();

        if (!_sprints.TryGetValue(sprintId, out var current))
            throw new KeyNotFoundException($"Sprint not found: {sprintId}");

        if (current.Status != SprintStatus.Planned)
            throw new InvalidOperationException($"Cannot start sprint in {current.Status} state");

        // Check for existing active sprint in same project
        var activeSprint = GetActiveSprint(current.ProjectId);
        if (activeSprint != null)
            throw new InvalidOperationException(
                $"Project already has an active sprint: {activeSprint.Name}");

        _pendingUpdates[sprintId] = true;
        try
        {
            var started = current with
            {
                Status = SprintStatus.Active,
                UpdatedAt = DateTime.UtcNow
            };

            _sprints[sprintId] = started;
            NotifyChanged();

            _toast.ShowSuccess($"Sprint started: \"{started.Name}\"");
            return Task.FromResult(started);
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Failed to start sprint {Id}", sprintId);
            _toast.ShowError(ex.Message, "Cannot Start Sprint");
            throw;
        }
        finally
        {
            _pendingUpdates.TryRemove(sprintId, out _);
        }
    }

    public Task<Sprint> CompleteSprintAsync(Guid sprintId, CancellationToken ct = default)
    {
        ThrowIfDisposed();

        if (!_sprints.TryGetValue(sprintId, out var current))
            throw new KeyNotFoundException($"Sprint not found: {sprintId}");

        if (current.Status != SprintStatus.Active)
            throw new InvalidOperationException($"Cannot complete sprint in {current.Status} state");

        _pendingUpdates[sprintId] = true;
        try
        {
            var completed = current with
            {
                Status = SprintStatus.Completed,
                UpdatedAt = DateTime.UtcNow
            };

            _sprints[sprintId] = completed;
            NotifyChanged();

            _toast.ShowSuccess($"Sprint completed: \"{completed.Name}\"");
            return Task.FromResult(completed);
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Failed to complete sprint {Id}", sprintId);
            _toast.ShowError(ex.Message, "Cannot Complete Sprint");
            throw;
        }
        finally
        {
            _pendingUpdates.TryRemove(sprintId, out _);
        }
    }

    public Task DeleteAsync(Guid id, CancellationToken ct = default)
    {
        ThrowIfDisposed();

        if (!_sprints.TryGetValue(id, out var current))
            return Task.CompletedTask; // Already deleted

        _pendingUpdates[id] = true;
        try
        {
            var deleted = current with { DeletedAt = DateTime.UtcNow };
            _sprints[id] = deleted;
            NotifyChanged();

            _toast.ShowSuccess($"Sprint deleted: \"{current.Name}\"");
            return Task.CompletedTask;
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Failed to delete sprint {Id}", id);
            _toast.ShowError("Failed to delete sprint. Please try again.");
            throw;
        }
        finally
        {
            _pendingUpdates.TryRemove(id, out _);
        }
    }

    public async Task RefreshAsync(Guid projectId, CancellationToken ct = default)
    {
        ThrowIfDisposed();

        var sprints = await _client.GetSprintsAsync(projectId, ct);

        // Remove existing sprints for this project
        var toRemove = _sprints.Values
            .Where(s => s.ProjectId == projectId)
            .Select(s => s.Id)
            .ToList();

        foreach (var id in toRemove)
            _sprints.TryRemove(id, out _);

        // Add fetched sprints
        foreach (var sprint in sprints)
            _sprints[sprint.Id] = sprint;

        NotifyChanged();
        _logger.LogDebug("Refreshed {Count} sprints for project {ProjectId}", sprints.Count, projectId);
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

    #region WebSocket Event Handlers

    private void HandleSprintCreated(Sprint sprint)
    {
        // Skip if we have a pending update (we created this)
        if (_pendingUpdates.ContainsKey(sprint.Id)) return;

        _sprints[sprint.Id] = sprint;
        NotifyChanged();
        _logger.LogDebug("Received sprint created: {Id}", sprint.Id);
    }

    private void HandleSprintUpdated(Sprint sprint, IReadOnlyList<FieldChange> changes)
    {
        // Skip if we have a pending update (we updated this)
        if (_pendingUpdates.ContainsKey(sprint.Id)) return;

        _sprints[sprint.Id] = sprint;
        NotifyChanged();
        _logger.LogDebug("Received sprint updated: {Id}", sprint.Id);
    }

    private void HandleSprintDeleted(Guid id)
    {
        // Skip if we have a pending update (we deleted this)
        if (_pendingUpdates.ContainsKey(id)) return;

        if (_sprints.TryGetValue(id, out var sprint))
        {
            _sprints[id] = sprint with { DeletedAt = DateTime.UtcNow };
            NotifyChanged();
        }

        _logger.LogDebug("Received sprint deleted: {Id}", id);
    }

    #endregion

    public bool IsPending(Guid id) => _pendingUpdates.ContainsKey(id);
}