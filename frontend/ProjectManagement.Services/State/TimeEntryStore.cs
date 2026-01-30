using System.Collections.Concurrent;
using Microsoft.Extensions.Logging;
using ProjectManagement.Core.Exceptions;
using ProjectManagement.Core.Interfaces;
using ProjectManagement.Core.Models;
using ProjectManagement.Services.Notifications;

namespace ProjectManagement.Services.State;

/// <summary>
/// Time entry store with optimistic updates and rollback support.
/// </summary>
public sealed class TimeEntryStore : ITimeEntryStore
{
    private readonly IWebSocketClient _client;
    private readonly IToastService _toast;
    private readonly ILogger<TimeEntryStore> _logger;
    private readonly Guid _currentUserId;

    // State
    private readonly ConcurrentDictionary<Guid, TimeEntry> _entries = new();
    private readonly ConcurrentDictionary<Guid, TimeEntry> _rollbackState = new();
    private readonly ConcurrentDictionary<Guid, bool> _pendingUpdates = new();
    private TimeEntry? _runningTimer;
    private bool _disposed;

    public event Action? OnChanged;

    public TimeEntryStore(
        IWebSocketClient client,
        AppState appState,
        IToastService toast,
        ILogger<TimeEntryStore> logger)
    {
        _client = client ?? throw new ArgumentNullException(nameof(client));
        _currentUserId = appState.CurrentUser?.Id ?? throw new InvalidOperationException("CurrentUser not set");
        _toast = toast ?? throw new ArgumentNullException(nameof(toast));
        _logger = logger ?? throw new ArgumentNullException(nameof(logger));

        // Subscribe to WebSocket events for real-time updates
        _client.OnTimerStarted += HandleTimerStarted;
        _client.OnTimerStopped += HandleTimerStopped;
        _client.OnTimeEntryCreated += HandleTimeEntryCreated;
        _client.OnTimeEntryUpdated += HandleTimeEntryUpdated;
        _client.OnTimeEntryDeleted += HandleTimeEntryDeleted;
    }

    public IReadOnlyList<TimeEntry> GetByWorkItem(Guid workItemId)
    {
        return _entries.Values
            .Where(e => e.WorkItemId == workItemId && e.DeletedAt == null)
            .OrderByDescending(e => e.StartedAt)
            .ToList();
    }

    public TimeEntry? GetRunningTimer() => _runningTimer;

    public bool IsPending(Guid timeEntryId) => _pendingUpdates.ContainsKey(timeEntryId);

    public TimeSpan GetTotalTime(Guid workItemId)
    {
        return _entries.Values
            .Where(e => e.WorkItemId == workItemId
                        && e.DeletedAt == null
                        && e.DurationSeconds.HasValue)
            .Aggregate(TimeSpan.Zero, (total, entry) =>
                total + TimeSpan.FromSeconds(entry.DurationSeconds!.Value));
    }

    public async Task<TimeEntry> StartTimerAsync(StartTimerRequest request, CancellationToken ct)
    {
        ThrowIfDisposed();

        // Optimistic: Create temp entry
        var tempId = Guid.NewGuid();
        var optimistic = new TimeEntry
        {
            Id = tempId,
            WorkItemId = request.WorkItemId,
            UserId = _currentUserId,
            StartedAt = DateTime.UtcNow,
            Description = request.Description,
            CreatedAt = DateTime.UtcNow,
            UpdatedAt = DateTime.UtcNow,
        };

        // Track previous running timer for rollback
        var previousRunning = _runningTimer;

        _entries[tempId] = optimistic;
        _runningTimer = optimistic;
        _pendingUpdates[tempId] = true;
        NotifyChanged();

        try
        {
            var (started, stopped) = await _client.StartTimerAsync(request, ct);

            // Remove temp, add confirmed
            _entries.TryRemove(tempId, out _);
            _pendingUpdates.TryRemove(tempId, out _);

            _entries[started.Id] = started;
            _runningTimer = started;

            // Update stopped entry if any
            if (stopped != null && _entries.ContainsKey(stopped.Id))
            {
                _entries[stopped.Id] = stopped;
            }

            NotifyChanged();
            _toast.ShowSuccess("Timer started");
            _logger.LogInformation("Started timer {TimerId} on {WorkItemId}", started.Id, request.WorkItemId);
            return started;
        }
        catch (ValidationException ex)
        {
            _entries.TryRemove(tempId, out _);
            _pendingUpdates.TryRemove(tempId, out _);
            _runningTimer = previousRunning;
            NotifyChanged();

            _toast.ShowError(ex.UserMessage, "Validation Error");
            throw;
        }
        catch (Exception ex)
        {
            // Rollback
            _entries.TryRemove(tempId, out _);
            _pendingUpdates.TryRemove(tempId, out _);
            _runningTimer = previousRunning;
            NotifyChanged();

            _logger.LogWarning(ex, "Failed to start timer on {WorkItemId}", request.WorkItemId);
            _toast.ShowError("Failed to start timer. Please try again.");
            throw;
        }
    }

    public async Task<TimeEntry> StopTimerAsync(Guid timeEntryId, CancellationToken ct)
    {
        ThrowIfDisposed();

        if (!_entries.TryGetValue(timeEntryId, out var existing))
        {
            throw new InvalidOperationException($"Time entry {timeEntryId} not found");
        }

        // Optimistic: Update entry with stopped state
        var optimistic = existing with
        {
            EndedAt = DateTime.UtcNow,
            DurationSeconds = (int)(DateTime.UtcNow - existing.StartedAt).TotalSeconds,
            UpdatedAt = DateTime.UtcNow,
        };

        _rollbackState[timeEntryId] = existing;
        _entries[timeEntryId] = optimistic;
        _pendingUpdates[timeEntryId] = true;

        if (_runningTimer?.Id == timeEntryId)
        {
            _runningTimer = null;
        }

        NotifyChanged();

        try
        {
            var result = await _client.StopTimerAsync(timeEntryId, ct);

            _entries[timeEntryId] = result;
            _rollbackState.TryRemove(timeEntryId, out _);
            _pendingUpdates.TryRemove(timeEntryId, out _);

            NotifyChanged();
            var duration = TimeSpan.FromSeconds(result.DurationSeconds ?? 0);
            _toast.ShowSuccess($"Timer stopped ({duration:g})");
            _logger.LogInformation("Stopped timer {TimerId}, duration: {Duration}s",
                result.Id, result.DurationSeconds);
            return result;
        }
        catch (ValidationException ex)
        {
            if (_rollbackState.TryRemove(timeEntryId, out var rollback))
            {
                _entries[timeEntryId] = rollback;
                if (rollback.IsRunning)
                {
                    _runningTimer = rollback;
                }
            }

            _pendingUpdates.TryRemove(timeEntryId, out _);
            NotifyChanged();

            _toast.ShowError(ex.UserMessage, "Validation Error");
            throw;
        }
        catch (Exception ex)
        {
            // Rollback
            if (_rollbackState.TryRemove(timeEntryId, out var rollback))
            {
                _entries[timeEntryId] = rollback;
                if (rollback.IsRunning)
                {
                    _runningTimer = rollback;
                }
            }

            _pendingUpdates.TryRemove(timeEntryId, out _);
            NotifyChanged();

            _logger.LogWarning(ex, "Failed to stop timer {TimerId}", timeEntryId);
            _toast.ShowError("Failed to stop timer. Please try again.");
            throw;
        }
    }

    public async Task<TimeEntry> CreateAsync(CreateTimeEntryRequest request, CancellationToken ct)
    {
        ThrowIfDisposed();

        // Optimistic: Create temp entry
        var tempId = Guid.NewGuid();
        var optimistic = new TimeEntry
        {
            Id = tempId,
            WorkItemId = request.WorkItemId,
            UserId = _currentUserId,
            StartedAt = request.StartedAt,
            EndedAt = request.EndedAt,
            DurationSeconds = (int)(request.EndedAt - request.StartedAt).TotalSeconds,
            Description = request.Description,
            CreatedAt = DateTime.UtcNow,
            UpdatedAt = DateTime.UtcNow,
        };

        _entries[tempId] = optimistic;
        _pendingUpdates[tempId] = true;
        NotifyChanged();

        try
        {
            var result = await _client.CreateTimeEntryAsync(request, ct);

            _entries.TryRemove(tempId, out _);
            _pendingUpdates.TryRemove(tempId, out _);
            _entries[result.Id] = result;

            NotifyChanged();
            _toast.ShowSuccess("Time entry added");
            _logger.LogInformation("Created time entry {EntryId} for {WorkItemId}",
                result.Id, request.WorkItemId);
            return result;
        }
        catch (ValidationException ex)
        {
            _entries.TryRemove(tempId, out _);
            _pendingUpdates.TryRemove(tempId, out _);
            NotifyChanged();

            _toast.ShowError(ex.UserMessage, "Validation Error");
            throw;
        }
        catch (Exception ex)
        {
            _entries.TryRemove(tempId, out _);
            _pendingUpdates.TryRemove(tempId, out _);
            NotifyChanged();

            _logger.LogWarning(ex, "Failed to create time entry for {WorkItemId}", request.WorkItemId);
            _toast.ShowError("Failed to create time entry. Please try again.");
            throw;
        }
    }

    public async Task<TimeEntry> UpdateAsync(UpdateTimeEntryRequest request, CancellationToken ct)
    {
        ThrowIfDisposed();

        if (!_entries.TryGetValue(request.TimeEntryId, out var existing))
        {
            throw new InvalidOperationException($"Time entry {request.TimeEntryId} not found");
        }

        // Apply optimistic updates
        var optimistic = existing with
        {
            StartedAt = request.StartedAt ?? existing.StartedAt,
            EndedAt = request.EndedAt ?? existing.EndedAt,
            Description = request.Description ?? existing.Description,
            UpdatedAt = DateTime.UtcNow,
        };

        // Recalculate duration if timestamps changed
        if (optimistic.EndedAt.HasValue)
        {
            optimistic = optimistic with
            {
                DurationSeconds = (int)(optimistic.EndedAt.Value - optimistic.StartedAt).TotalSeconds
            };
        }

        _rollbackState[request.TimeEntryId] = existing;
        _entries[request.TimeEntryId] = optimistic;
        _pendingUpdates[request.TimeEntryId] = true;
        NotifyChanged();

        try
        {
            var result = await _client.UpdateTimeEntryAsync(request, ct);

            _entries[request.TimeEntryId] = result;
            _rollbackState.TryRemove(request.TimeEntryId, out _);
            _pendingUpdates.TryRemove(request.TimeEntryId, out _);

            NotifyChanged();
            _toast.ShowSuccess("Time entry updated");
            return result;
        }
        catch (ValidationException ex)
        {
            if (_rollbackState.TryRemove(request.TimeEntryId, out var rollback))
            {
                _entries[request.TimeEntryId] = rollback;
            }

            _pendingUpdates.TryRemove(request.TimeEntryId, out _);
            NotifyChanged();

            _toast.ShowError(ex.UserMessage, "Validation Error");
            throw;
        }
        catch (VersionConflictException ex)
        {
            if (_rollbackState.TryRemove(request.TimeEntryId, out var rollback))
            {
                _entries[request.TimeEntryId] = rollback;
            }

            _pendingUpdates.TryRemove(request.TimeEntryId, out _);
            NotifyChanged();

            _toast.ShowError(ex.UserMessage, "Conflict");
            throw;
        }
        catch (Exception ex)
        {
            if (_rollbackState.TryRemove(request.TimeEntryId, out var rollback))
            {
                _entries[request.TimeEntryId] = rollback;
            }

            _pendingUpdates.TryRemove(request.TimeEntryId, out _);
            NotifyChanged();

            _logger.LogWarning(ex, "Failed to update time entry {EntryId}", request.TimeEntryId);
            _toast.ShowError("Failed to update time entry. Please try again.");
            throw;
        }
    }

    public async Task DeleteAsync(Guid timeEntryId, CancellationToken ct)
    {
        ThrowIfDisposed();

        if (!_entries.TryGetValue(timeEntryId, out var existing))
        {
            return; // Already deleted or not found
        }

        // Optimistic: Mark as deleted
        var optimistic = existing with { DeletedAt = DateTime.UtcNow };

        _rollbackState[timeEntryId] = existing;
        _entries[timeEntryId] = optimistic;
        _pendingUpdates[timeEntryId] = true;

        if (_runningTimer?.Id == timeEntryId)
        {
            _runningTimer = null;
        }

        NotifyChanged();

        try
        {
            await _client.DeleteTimeEntryAsync(timeEntryId, ct);

            _entries.TryRemove(timeEntryId, out _);
            _rollbackState.TryRemove(timeEntryId, out _);
            _pendingUpdates.TryRemove(timeEntryId, out _);

            NotifyChanged();
            _toast.ShowSuccess("Time entry deleted");
            _logger.LogInformation("Deleted time entry {EntryId}", timeEntryId);
        }
        catch (ValidationException ex)
        {
            if (_rollbackState.TryRemove(timeEntryId, out var rollback))
            {
                _entries[timeEntryId] = rollback;
                if (rollback.IsRunning)
                {
                    _runningTimer = rollback;
                }
            }

            _pendingUpdates.TryRemove(timeEntryId, out _);
            NotifyChanged();

            _toast.ShowError(ex.UserMessage, "Validation Error");
            throw;
        }
        catch (Exception ex)
        {
            if (_rollbackState.TryRemove(timeEntryId, out var rollback))
            {
                _entries[timeEntryId] = rollback;
                if (rollback.IsRunning)
                {
                    _runningTimer = rollback;
                }
            }

            _pendingUpdates.TryRemove(timeEntryId, out _);
            NotifyChanged();

            _logger.LogWarning(ex, "Failed to delete time entry {EntryId}", timeEntryId);
            _toast.ShowError("Failed to delete time entry. Please try again.");
            throw;
        }
    }

    public async Task RefreshAsync(Guid workItemId, CancellationToken ct)
    {
        ThrowIfDisposed();

        var (entries, _) = await _client.GetTimeEntriesAsync(workItemId, ct: ct);

        foreach (var entry in entries)
        {
            _entries[entry.Id] = entry;
        }

        NotifyChanged();
    }

    public async Task RefreshRunningTimerAsync(CancellationToken ct)
    {
        ThrowIfDisposed();

        var running = await _client.GetRunningTimerAsync(ct);
        _runningTimer = running;

        if (running != null)
        {
            _entries[running.Id] = running;
        }

        NotifyChanged();
    }

    // Event handlers for real-time updates from other clients
    private void HandleTimerStarted(TimeEntry started, TimeEntry? stopped)
    {
        // Skip if this is from our own pending operation
        if (_pendingUpdates.ContainsKey(started.Id)) return;

        _entries[started.Id] = started;

        // Only update running timer if it's the current user's
        if (started.UserId == _currentUserId)
        {
            _runningTimer = started;
        }

        if (stopped != null)
        {
            _entries[stopped.Id] = stopped;
            if (_runningTimer?.Id == stopped.Id)
            {
                _runningTimer = null;
            }
        }

        NotifyChanged();
    }

    private void HandleTimerStopped(TimeEntry entry)
    {
        if (_pendingUpdates.ContainsKey(entry.Id)) return;

        _entries[entry.Id] = entry;

        if (_runningTimer?.Id == entry.Id)
        {
            _runningTimer = null;
        }

        NotifyChanged();
    }

    private void HandleTimeEntryCreated(TimeEntry entry)
    {
        if (_pendingUpdates.ContainsKey(entry.Id)) return;

        _entries[entry.Id] = entry;
        NotifyChanged();
    }

    private void HandleTimeEntryUpdated(TimeEntry entry)
    {
        if (_pendingUpdates.ContainsKey(entry.Id)) return;

        _entries[entry.Id] = entry;
        NotifyChanged();
    }

    private void HandleTimeEntryDeleted(Guid timeEntryId, Guid workItemId)
    {
        if (_pendingUpdates.ContainsKey(timeEntryId)) return;

        _entries.TryRemove(timeEntryId, out _);

        if (_runningTimer?.Id == timeEntryId)
        {
            _runningTimer = null;
        }

        NotifyChanged();
    }

    private void NotifyChanged() => OnChanged?.Invoke();

    private void ThrowIfDisposed()
    {
        if (_disposed)
            throw new ObjectDisposedException(nameof(TimeEntryStore));
    }

    public void Dispose()
    {
        if (_disposed) return;
        _disposed = true;

        _client.OnTimerStarted -= HandleTimerStarted;
        _client.OnTimerStopped -= HandleTimerStopped;
        _client.OnTimeEntryCreated -= HandleTimeEntryCreated;
        _client.OnTimeEntryUpdated -= HandleTimeEntryUpdated;
        _client.OnTimeEntryDeleted -= HandleTimeEntryDeleted;
    }
}