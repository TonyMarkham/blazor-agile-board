namespace ProjectManagement.Core.Models;

/// <summary>
/// Request to create a manual (already completed) time entry.
/// Use this for logging time after the fact.
/// </summary>
public sealed record CreateTimeEntryRequest
{
    /// <summary>The work item to log time against.</summary>
    public Guid WorkItemId { get; init; }

    /// <summary>When the work started (UTC).</summary>
    public DateTime StartedAt { get; init; }

    /// <summary>When the work ended (UTC).</summary>
    public DateTime EndedAt { get; init; }

    /// <summary>Optional description of what was done.</summary>
    public string? Description { get; init; }
}