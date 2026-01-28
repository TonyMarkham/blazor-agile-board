namespace ProjectManagement.Core.Models;

/// <summary>
/// Request to update an existing time entry.
/// Only the owner can update their time entries.
/// </summary>
public sealed record UpdateTimeEntryRequest
{
    /// <summary>The ID of the time entry to update.</summary>
    public Guid TimeEntryId { get; init; }

    /// <summary>New start time (if changing).</summary>
    public DateTime? StartedAt { get; init; }

    /// <summary>New end time (if changing).</summary>
    public DateTime? EndedAt { get; init; }

    /// <summary>New description (if changing).</summary>
    public string? Description { get; init; }
}