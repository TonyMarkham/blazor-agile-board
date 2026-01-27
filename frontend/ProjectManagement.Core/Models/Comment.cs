using ProjectManagement.Core.Interfaces;

namespace ProjectManagement.Core.Models;

/// <summary>
/// A comment on a work item.
/// Comments can only be edited/deleted by their author.
/// </summary>
public sealed record Comment : IAuditable
{
    /// <summary>
    /// Unique identifier for this comment.
    /// </summary>
    public Guid Id { get; init; }

    /// <summary>
    /// ID of the work item this comment is attached to.
    /// </summary>
    public Guid WorkItemId { get; init; }

    /// <summary>
    /// The comment content (plain text).
    /// </summary>
    public string Content { get; init; } = string.Empty;

    /// <summary>
    /// When the comment was created.
    /// </summary>
    public DateTime CreatedAt { get; init; }

    /// <summary>
    /// When the comment was last updated.
    /// </summary>
    public DateTime UpdatedAt { get; init; }

    /// <summary>
    /// User who created the comment (the author).
    /// Only this user can edit/delete the comment.
    /// </summary>
    public Guid CreatedBy { get; init; }

    /// <summary>
    /// User who last updated the comment (should be same as CreatedBy).
    /// </summary>
    public Guid UpdatedBy { get; init; }

    /// <summary>
    /// When the comment was soft-deleted, or null if active.
    /// </summary>
    public DateTime? DeletedAt { get; init; }
}