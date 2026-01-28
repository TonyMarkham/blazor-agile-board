namespace ProjectManagement.Core.Models;

/// <summary>
/// A time tracking entry on a work item.
/// Can be a running timer (EndedAt = null) or a completed entry.
/// </summary>
public sealed record TimeEntry
{
    /// <summary>Unique identifier for this time entry.</summary>
    public Guid Id { get; init; }

    /// <summary>The work item this time was logged against.</summary>
    public Guid WorkItemId { get; init; }

    /// <summary>The user who logged this time.</summary>
    public Guid UserId { get; init; }

    /// <summary>When the timer started (UTC).</summary>
    public DateTime StartedAt { get; init; }

    /// <summary>
    /// When the timer stopped (UTC).
    /// Null if timer is still running.
    /// </summary>
    public DateTime? EndedAt { get; init; }

    /// <summary>
    /// Pre-calculated duration in seconds.
    /// Null if timer is still running.
    /// </summary>
    public int? DurationSeconds { get; init; }

    /// <summary>Optional description of what was worked on.</summary>
    public string? Description { get; init; }

    /// <summary>When this entry was created (UTC).</summary>
    public DateTime CreatedAt { get; init; }

    /// <summary>When this entry was last updated (UTC).</summary>
    public DateTime UpdatedAt { get; init; }

    /// <summary>
    /// When this entry was deleted (UTC).
    /// Null if not deleted (soft delete pattern).
    /// </summary>
    public DateTime? DeletedAt { get; init; }

    /// <summary>
    /// True if this timer is currently running.
    /// A timer is running if EndedAt is null and it's not deleted.
    /// </summary>
    public bool IsRunning => EndedAt == null && DeletedAt == null;

    /// <summary>
    /// Get the elapsed time for this entry.
    /// If running, calculates from StartedAt to now.
    /// If stopped, uses DurationSeconds or calculates from EndedAt.
    /// </summary>
    public TimeSpan Elapsed
    {
        get
        {
            if (DurationSeconds.HasValue)
            {
                return TimeSpan.FromSeconds(DurationSeconds.Value);
            }

            if (EndedAt.HasValue)
            {
                return EndedAt.Value - StartedAt;
            }

            // Running timer - calculate from now
            return DateTime.UtcNow - StartedAt;
        }
    }

    /// <summary>
    /// Format the elapsed time as a human-readable string.
    /// Examples: "1:23:45" (hours), "23:45" (minutes), "00:45" (seconds only)
    /// </summary>
    public string ElapsedFormatted
    {
        get
        {
            var elapsed = Elapsed;
            return elapsed.TotalHours >= 1
                ? $"{(int)elapsed.TotalHours}:{elapsed.Minutes:D2}:{elapsed.Seconds:D2}"
                : $"{elapsed.Minutes:D2}:{elapsed.Seconds:D2}";
        }
    }
}