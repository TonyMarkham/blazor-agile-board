using System.Collections.Concurrent;
using Microsoft.Extensions.Logging;
using ProjectManagement.Core.Interfaces;
using ProjectManagement.Core.Models;

namespace ProjectManagement.Services.State;

/// <summary>
/// State store for comments with WebSocket integration.
/// Comments are organized by work item ID.
/// </summary>
public sealed class CommentStore : ICommentStore
{
    private readonly IWebSocketClient _client;
    private readonly ILogger<CommentStore> _logger;
    private readonly ConcurrentDictionary<Guid, Comment> _comments = new();
    private readonly ConcurrentDictionary<Guid, bool> _pendingUpdates = new();
    private bool _disposed;

    public event Action? OnChanged;

    public CommentStore(IWebSocketClient client, ILogger<CommentStore> logger)
    {
        _client = client ?? throw new ArgumentNullException(nameof(client));
        _logger = logger ?? throw new ArgumentNullException(nameof(logger));

        // Subscribe to WebSocket events
        _client.OnCommentCreated += HandleCommentCreated;
        _client.OnCommentUpdated += HandleCommentUpdated;
        _client.OnCommentDeleted += HandleCommentDeleted;
    }

    public void Dispose()
    {
        if (_disposed) return;
        _disposed = true;

        _client.OnCommentCreated -= HandleCommentCreated;
        _client.OnCommentUpdated -= HandleCommentUpdated;
        _client.OnCommentDeleted -= HandleCommentDeleted;
    }

    public IReadOnlyList<Comment> GetComments(Guid workItemId)
    {
        return _comments.Values
            .Where(c => c.WorkItemId == workItemId && c.DeletedAt == null)
            .OrderBy(c => c.CreatedAt)
            .ToList();
    }

    public bool IsPending(Guid commentId)
    {
        return _pendingUpdates.ContainsKey(commentId);
    }

    public async Task<Comment> CreateAsync(
        CreateCommentRequest request,
        CancellationToken ct = default)
    {
        ThrowIfDisposed();

        // Create optimistic comment with temporary ID
        var tempId = Guid.NewGuid();
        var optimistic = new Comment
        {
            Id = tempId,
            WorkItemId = request.WorkItemId,
            Content = request.Content,
            CreatedAt = DateTime.UtcNow,
            UpdatedAt = DateTime.UtcNow,
            CreatedBy = Guid.Empty, // Will be set by server
            UpdatedBy = Guid.Empty
        };

        // Optimistic update
        _comments[tempId] = optimistic;
        _pendingUpdates[tempId] = true;
        NotifyChanged();

        try
        {
            var confirmed = await _client.CreateCommentAsync(request, ct);

            // Replace temp with confirmed
            _comments.TryRemove(tempId, out _);
            _comments[confirmed.Id] = confirmed;
            _pendingUpdates.TryRemove(tempId, out _);
            NotifyChanged();

            _logger.LogDebug("Comment created: {Id}", confirmed.Id);
            return confirmed;
        }
        catch
        {
            // Rollback
            _comments.TryRemove(tempId, out _);
            _pendingUpdates.TryRemove(tempId, out _);
            NotifyChanged();
            throw;
        }
    }

    public async Task<Comment> UpdateAsync(
        UpdateCommentRequest request,
        CancellationToken ct = default)
    {
        ThrowIfDisposed();

        if (!_comments.TryGetValue(request.CommentId, out var current))
            throw new KeyNotFoundException($"Comment not found: {request.CommentId}");

        // Optimistic update
        var optimistic = current with
        {
            Content = request.Content,
            UpdatedAt = DateTime.UtcNow
        };

        var previousValue = _comments[request.CommentId];
        _comments[request.CommentId] = optimistic;
        _pendingUpdates[request.CommentId] = true;
        NotifyChanged();

        try
        {
            var confirmed = await _client.UpdateCommentAsync(request, ct);
            _comments[request.CommentId] = confirmed;
            _pendingUpdates.TryRemove(request.CommentId, out _);
            NotifyChanged();
            return confirmed;
        }
        catch
        {
            // Rollback
            _comments[request.CommentId] = previousValue;
            _pendingUpdates.TryRemove(request.CommentId, out _);
            NotifyChanged();
            throw;
        }
    }

    public async Task DeleteAsync(Guid commentId, CancellationToken ct = default)
    {
        ThrowIfDisposed();

        if (!_comments.TryGetValue(commentId, out var current))
            throw new KeyNotFoundException($"Comment not found: {commentId}");

        // Optimistic delete (soft delete)
        var optimistic = current with { DeletedAt = DateTime.UtcNow };
        _comments[commentId] = optimistic;
        _pendingUpdates[commentId] = true;
        NotifyChanged();

        try
        {
            await _client.DeleteCommentAsync(commentId, ct);
            _pendingUpdates.TryRemove(commentId, out _);
            NotifyChanged();
            _logger.LogDebug("Comment deleted: {Id}", commentId);
        }
        catch
        {
            // Rollback
            _comments[commentId] = current;
            _pendingUpdates.TryRemove(commentId, out _);
            NotifyChanged();
            throw;
        }
    }

    public async Task RefreshAsync(Guid workItemId, CancellationToken ct = default)
    {
        ThrowIfDisposed();

        var comments = await _client.GetCommentsAsync(workItemId, ct);

        // Remove existing comments for this work item
        var toRemove = _comments.Values
            .Where(c => c.WorkItemId == workItemId)
            .Select(c => c.Id)
            .ToList();

        foreach (var id in toRemove)
            _comments.TryRemove(id, out _);

        // Add fetched comments
        foreach (var comment in comments)
            _comments[comment.Id] = comment;

        NotifyChanged();
        _logger.LogDebug("Refreshed {Count} comments for work item {WorkItemId}", comments.Count, workItemId);
    }

    #region WebSocket Event Handlers

    private void HandleCommentCreated(Comment comment)
    {
        if (_pendingUpdates.ContainsKey(comment.Id)) return;

        _comments[comment.Id] = comment;
        NotifyChanged();
        _logger.LogDebug("Received comment created: {Id}", comment.Id);
    }

    private void HandleCommentUpdated(Comment comment)
    {
        if (_pendingUpdates.ContainsKey(comment.Id)) return;

        _comments[comment.Id] = comment;
        NotifyChanged();
        _logger.LogDebug("Received comment updated: {Id}", comment.Id);
    }

    private void HandleCommentDeleted(Guid id)
    {
        if (_pendingUpdates.ContainsKey(id)) return;

        if (_comments.TryGetValue(id, out var comment))
        {
            _comments[id] = comment with { DeletedAt = DateTime.UtcNow };
            NotifyChanged();
        }

        _logger.LogDebug("Received comment deleted: {Id}", id);
    }

    #endregion

    private void NotifyChanged() => OnChanged?.Invoke();

    private void ThrowIfDisposed()
    {
        ObjectDisposedException.ThrowIf(_disposed, this);
    }
}