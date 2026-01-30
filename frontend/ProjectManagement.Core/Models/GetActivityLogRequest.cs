namespace ProjectManagement.Core.Models;

/// <summary>
/// Request to fetch activity log entries for a specific entity.
/// </summary>
public sealed record GetActivityLogRequest
{
    /// <summary>Entity type (e.g., "work_item", "sprint").</summary>
    public string EntityType { get; init; } = string.Empty;

    /// <summary>Entity ID to fetch activity for.</summary>
    public Guid EntityId { get; init; }

    /// <summary>Maximum number of entries to return.</summary>
    public int Limit { get; init; } = 50;

    /// <summary>Number of entries to skip (pagination offset).</summary>
    public int Offset { get; init; }
}