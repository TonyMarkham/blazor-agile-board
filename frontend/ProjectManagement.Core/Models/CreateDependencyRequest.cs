namespace ProjectManagement.Core.Models;

/// <summary>
/// Request to create a dependency between two work items.
/// </summary>
public sealed record CreateDependencyRequest
{
    /// <summary>
    /// The work item that will be blocking.
    /// Must be in the same project as the blocked item.
    /// </summary>
    public Guid BlockingItemId { get; init; }

    /// <summary>
    /// The work item that will be blocked.
    /// Must be in the same project as the blocking item.
    /// </summary>
    public Guid BlockedItemId { get; init; }

    /// <summary>
    /// The type of dependency to create.
    /// Blocks = requires blocking item to complete first.
    /// RelatesTo = informational only.
    /// </summary>
    public DependencyType Type { get; init; }
}