using System.Collections.Concurrent;
using Microsoft.Extensions.Logging;
using ProjectManagement.Core.Interfaces;
using ProjectManagement.Core.Models;

namespace ProjectManagement.Services.State;

/// <summary>
///     Thread-safe state management for sprints.
///     Follows same pattern as WorkItemStore.
///     Note: Full WebSocket integration deferred to Session 40 (Sprints & Comments).
/// </summary>
public sealed class SprintStore : ISprintStore
{
    private readonly IWebSocketClient _client;
    private readonly ILogger<SprintStore> _logger;
    private readonly ConcurrentDictionary<Guid, Sprint> _sprints = new();

    private bool _disposed;

    public SprintStore(
        IWebSocketClient client,
        ILogger<SprintStore> logger)
    {
        _client = client;
        _logger = logger;

        // Sprint events will be wired up in Session 40 when backend handlers are implemented
        // _client.OnSprintCreated += HandleSprintCreated;
        // _client.OnSprintUpdated += HandleSprintUpdated;
    }

    public event Action? OnChanged;

    public void Dispose()
    {
        if (_disposed) return;
        _disposed = true;
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

    public Task<Sprint> CreateAsync(
        CreateSprintRequest request,
        CancellationToken ct = default)
    {
        ThrowIfDisposed();

        // Local-only creation until Session 40 implements WebSocket handlers
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
            CreatedBy = Guid.Empty, // Will be set by server in Session 40
            UpdatedBy = Guid.Empty
        };

        _sprints[sprint.Id] = sprint;
        NotifyChanged();

        _logger.LogDebug("Sprint created locally: {Id}", sprint.Id);
        return Task.FromResult(sprint);
    }

    public Task<Sprint> UpdateAsync(
        UpdateSprintRequest request,
        CancellationToken ct = default)
    {
        ThrowIfDisposed();

        if (!_sprints.TryGetValue(request.SprintId, out var current))
            throw new KeyNotFoundException($"Sprint not found: {request.SprintId}");

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

        return Task.FromResult(updated);
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

        var started = current with
        {
            Status = SprintStatus.Active,
            UpdatedAt = DateTime.UtcNow
        };

        _sprints[sprintId] = started;
        NotifyChanged();

        return Task.FromResult(started);
    }

    public Task<Sprint> CompleteSprintAsync(Guid sprintId, CancellationToken ct = default)
    {
        ThrowIfDisposed();

        if (!_sprints.TryGetValue(sprintId, out var current))
            throw new KeyNotFoundException($"Sprint not found: {sprintId}");

        if (current.Status != SprintStatus.Active)
            throw new InvalidOperationException($"Cannot complete sprint in {current.Status} state");

        var completed = current with
        {
            Status = SprintStatus.Completed,
            UpdatedAt = DateTime.UtcNow
        };

        _sprints[sprintId] = completed;
        NotifyChanged();

        return Task.FromResult(completed);
    }

    public Task DeleteAsync(Guid id, CancellationToken ct = default)
    {
        ThrowIfDisposed();

        if (!_sprints.TryGetValue(id, out var current))
            return Task.CompletedTask; // Already deleted

        var deleted = current with { DeletedAt = DateTime.UtcNow };
        _sprints[id] = deleted;
        NotifyChanged();

        return Task.CompletedTask;
    }

    public Task RefreshAsync(Guid projectId, CancellationToken ct = default)
    {
        ThrowIfDisposed();

        // Will call _client.GetSprintsAsync when backend handler is implemented in Session 40
        _logger.LogDebug("Sprint refresh for project {ProjectId} - backend not yet implemented", projectId);
        return Task.CompletedTask;
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
}