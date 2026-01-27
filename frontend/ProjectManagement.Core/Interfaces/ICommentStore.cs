using ProjectManagement.Core.Models;

namespace ProjectManagement.Core.Interfaces;

/// <summary>
/// State store for comments.
/// Manages comment collections per work item with real-time updates.
/// </summary>
public interface ICommentStore : IDisposable
{
    /// <summary>
    /// Fired when the comment collection changes.
    /// </summary>
    event Action? OnChanged;

    /// <summary>
    /// Get all comments for a work item.
    /// </summary>
    IReadOnlyList<Comment> GetComments(Guid workItemId);

    /// <summary>
    /// Check if a comment has a pending update.
    /// </summary>
    bool IsPending(Guid commentId);

    /// <summary>
    /// Create a new comment on a work item.
    /// </summary>
    Task<Comment> CreateAsync(CreateCommentRequest request, CancellationToken ct = default);

    /// <summary>
    /// Update an existing comment.
    /// Only works if the current user is the author.
    /// </summary>
    Task<Comment> UpdateAsync(UpdateCommentRequest request, CancellationToken ct = default);

    /// <summary>
    /// Delete a comment.
    /// Only works if the current user is the author.
    /// </summary>
    Task DeleteAsync(Guid commentId, CancellationToken ct = default);

    /// <summary>
    /// Refresh comments for a work item from the server.
    /// </summary>
    Task RefreshAsync(Guid workItemId, CancellationToken ct = default);
}