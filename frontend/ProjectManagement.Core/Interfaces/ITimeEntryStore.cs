using ProjectManagement.Core.Models;

namespace ProjectManagement.Core.Interfaces;

/// <summary>
/// Store for managing time entry state with optimistic updates.
/// Handles running timer tracking and time entry CRUD operations.
/// </summary>
public interface ITimeEntryStore : IDisposable
{
    /// <summary>Fired when any time entry state changes.</summary>
    event Action? OnChanged;

    /// <summary>
    /// Get all time entries for a work item (non-deleted only).
    /// Entries are ordered by started_at descending (most recent first).
    /// </summary>
    IReadOnlyList<TimeEntry> GetByWorkItem(Guid workItemId);

    /// <summary>
    /// Get the currently running timer for the current user.
    /// Returns null if no timer is running.
    /// </summary>
    TimeEntry? GetRunningTimer();

    /// <summary>
    /// Check if a time entry has a pending server operation.
    /// UI can show loading indicators for pending items.
    /// </summary>
    bool IsPending(Guid timeEntryId);

    /// <summary>
    /// Start a timer on a work item.
    /// Automatically stops any existing running timer for this user.
    /// Uses optimistic update pattern.
    /// </summary>
    Task<TimeEntry> StartTimerAsync(StartTimerRequest request, CancellationToken ct = default);

    /// <summary>
    /// Stop the specified timer.
    /// Only the owner can stop their timer.
    /// </summary>
    Task<TimeEntry> StopTimerAsync(Guid timeEntryId, CancellationToken ct = default);

    /// <summary>
    /// Create a manual (already completed) time entry.
    /// Use this for logging time after the fact.
    /// </summary>
    Task<TimeEntry> CreateAsync(CreateTimeEntryRequest request, CancellationToken ct = default);

    /// <summary>
    /// Update a time entry.
    /// Only the owner can update their entries.
    /// </summary>
    Task<TimeEntry> UpdateAsync(UpdateTimeEntryRequest request, CancellationToken ct = default);

    /// <summary>
    /// Delete a time entry (soft delete).
    /// Only the owner can delete their entries.
    /// </summary>
    Task DeleteAsync(Guid timeEntryId, CancellationToken ct = default);

    /// <summary>
    /// Refresh entries for a work item from the server.
    /// Call when navigating to a work item detail view.
    /// </summary>
    Task RefreshAsync(Guid workItemId, CancellationToken ct = default);

    /// <summary>
    /// Fetch the current running timer from the server.
    /// Call on app startup and after reconnection.
    /// </summary>
    Task RefreshRunningTimerAsync(CancellationToken ct = default);

    /// <summary>
    /// Calculate total time logged for a work item.
    /// Only includes completed entries (not running timer).
    /// </summary>
    TimeSpan GetTotalTime(Guid workItemId);
}