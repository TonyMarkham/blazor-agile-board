namespace ProjectManagement.Core.Models;

/// <summary>
/// Request to update an existing comment.
/// Only the comment author can update their comment.
/// </summary>
public sealed record UpdateCommentRequest
{
    /// <summary>
    /// ID of the comment to update.
    /// </summary>
    public required Guid CommentId { get; init; }

    /// <summary>
    /// The new comment content.
    /// Must be 1-5000 characters.
    /// </summary>
    public required string Content { get; init; }
}