namespace ProjectManagement.Core.Models;

/// <summary>
/// Request to create a new comment on a work item.
/// </summary>
public sealed record CreateCommentRequest
{
    /// <summary>
    /// ID of the work item to comment on.
    /// </summary>
    public required Guid WorkItemId { get; init; }

    /// <summary>
    /// The comment content.
    /// Must be 1-5000 characters.
    /// </summary>
    public required string Content { get; init; }
}