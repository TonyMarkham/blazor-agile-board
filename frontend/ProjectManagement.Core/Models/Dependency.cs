namespace ProjectManagement.Core.Models;

/// <summary>
/// A dependency relationship between two work items.
/// </summary>
public sealed record Dependency
{
    /// <summary>Unique identifier for this dependency.</summary>
    public Guid Id { get; init; }

    /// <summary>
    /// The work item that is doing the blocking.
    /// This item must be completed before the blocked item can start.
    /// </summary>
    public Guid BlockingItemId { get; init; }

    /// <summary>
    /// The work item that is being blocked.
    /// This item cannot start until the blocking item is complete.
    /// </summary>
    public Guid BlockedItemId { get; init; }

    /// <summary>The type of dependency relationship.</summary>
    public DependencyType Type { get; init; }

    /// <summary>When this dependency was created (UTC).</summary>
    public DateTime CreatedAt { get; init; }

    /// <summary>The user who created this dependency.</summary>
    public Guid CreatedBy { get; init; }

    /// <summary>
    /// When this dependency was deleted (UTC).
    /// Null if not deleted (soft delete pattern).
    /// </summary>
    public DateTime? DeletedAt { get; init; }
}