namespace ProjectManagement.Core.Models;

/// <summary>
/// Paginated activity log response.
/// Mirrors ActivityLogList in protobuf.
/// </summary>
public sealed record ActivityLogPage
{
    /// <summary>Activity log entries for the current page.</summary>
    public IReadOnlyList<ActivityLog> Entries { get; init; } = Array.Empty<ActivityLog>();

    /// <summary>Total count of entries (for pagination UI).</summary>
    public int TotalCount { get; init; }

    /// <summary>True if there are more entries beyond this page.</summary>
    public bool HasMore { get; init; }
}