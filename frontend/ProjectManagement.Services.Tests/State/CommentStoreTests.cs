using FluentAssertions;
using Microsoft.Extensions.Logging;
using Moq;
using ProjectManagement.Core.Interfaces;
using ProjectManagement.Core.Models;
using ProjectManagement.Services.State;
using Xunit;

namespace ProjectManagement.Services.Tests.State;

public class CommentStoreTests
{
    private readonly Mock<IWebSocketClient> _mockClient;
    private readonly Mock<ILogger<CommentStore>> _mockLogger;
    private readonly CommentStore _store;

    public CommentStoreTests()
    {
        _mockClient = new Mock<IWebSocketClient>();
        _mockLogger = new Mock<ILogger<CommentStore>>();
        _store = new CommentStore(_mockClient.Object, _mockLogger.Object);
    }

    [Fact]
    public async Task CreateAsync_GivenValidRequest_ReturnsConfirmedComment()
    {
        // Given
        var workItemId = Guid.NewGuid();
        var request = new CreateCommentRequest
        {
            WorkItemId = workItemId,
            Content = "Test comment",
        };

        var confirmedComment = new Comment
        {
            Id = Guid.NewGuid(),
            WorkItemId = workItemId,
            Content = "Test comment",
            CreatedAt = DateTime.UtcNow,
            UpdatedAt = DateTime.UtcNow,
            CreatedBy = Guid.NewGuid(),
            UpdatedBy = Guid.NewGuid(),
        };

        _mockClient
            .Setup(c => c.CreateCommentAsync(request, It.IsAny<CancellationToken>()))
            .ReturnsAsync(confirmedComment);

        // When
        var result = await _store.CreateAsync(request);

        // Then
        result.Should().BeEquivalentTo(confirmedComment);
        _store.GetComments(workItemId).Should().Contain(c => c.Id == confirmedComment.Id);
    }

    [Fact]
    public async Task CreateAsync_GivenServerError_RollsBackOptimisticUpdate()
    {
        // Given
        var workItemId = Guid.NewGuid();
        var request = new CreateCommentRequest
        {
            WorkItemId = workItemId,
            Content = "Test comment",
        };

        _mockClient
            .Setup(c => c.CreateCommentAsync(request, It.IsAny<CancellationToken>()))
            .ThrowsAsync(new Exception("Server error"));

        // When
        var act = async () => await _store.CreateAsync(request);

        // Then
        await act.Should().ThrowAsync<Exception>();
        _store.GetComments(workItemId).Should().BeEmpty();
    }

    [Fact]
    public async Task UpdateAsync_GivenValidRequest_ReturnsUpdatedComment()
    {
        // Given - First create a comment
        var workItemId = Guid.NewGuid();
        var commentId = Guid.NewGuid();

        var existingComment = new Comment
        {
            Id = commentId,
            WorkItemId = workItemId,
            Content = "Original content",
            CreatedAt = DateTime.UtcNow,
            UpdatedAt = DateTime.UtcNow,
            CreatedBy = Guid.NewGuid(),
            UpdatedBy = Guid.NewGuid(),
        };

        _mockClient
            .Setup(c => c.GetCommentsAsync(workItemId, It.IsAny<CancellationToken>()))
            .ReturnsAsync(new List<Comment> { existingComment });

        await _store.RefreshAsync(workItemId);

        var updateRequest = new UpdateCommentRequest
        {
            CommentId = commentId,
            Content = "Updated content",
        };

        var updatedComment = existingComment with
        {
            Content = "Updated content",
            UpdatedAt = DateTime.UtcNow,
        };

        _mockClient
            .Setup(c => c.UpdateCommentAsync(updateRequest, It.IsAny<CancellationToken>()))
            .ReturnsAsync(updatedComment);

        // When
        var result = await _store.UpdateAsync(updateRequest);

        // Then
        result.Content.Should().Be("Updated content");
    }

    [Fact]
    public async Task DeleteAsync_GivenValidRequest_RemovesComment()
    {
        // Given - First create a comment
        var workItemId = Guid.NewGuid();
        var commentId = Guid.NewGuid();

        var existingComment = new Comment
        {
            Id = commentId,
            WorkItemId = workItemId,
            Content = "To be deleted",
            CreatedAt = DateTime.UtcNow,
            UpdatedAt = DateTime.UtcNow,
            CreatedBy = Guid.NewGuid(),
            UpdatedBy = Guid.NewGuid(),
        };

        _mockClient
            .Setup(c => c.GetCommentsAsync(workItemId, It.IsAny<CancellationToken>()))
            .ReturnsAsync(new List<Comment> { existingComment });

        await _store.RefreshAsync(workItemId);

        _mockClient
            .Setup(c => c.DeleteCommentAsync(commentId, It.IsAny<CancellationToken>()))
            .Returns(Task.CompletedTask);

        // When
        await _store.DeleteAsync(commentId);

        // Then
        _store.GetComments(workItemId).Should().NotContain(c => c.Id == commentId);
    }

    [Fact]
    public void HandleCommentCreated_GivenExternalEvent_UpdatesStore()
    {
        // Given
        var comment = new Comment
        {
            Id = Guid.NewGuid(),
            WorkItemId = Guid.NewGuid(),
            Content = "External comment",
            CreatedAt = DateTime.UtcNow,
            UpdatedAt = DateTime.UtcNow,
            CreatedBy = Guid.NewGuid(),
            UpdatedBy = Guid.NewGuid(),
        };

        // When - Simulate WebSocket event
        _mockClient.Raise(c => c.OnCommentCreated += null, comment);

        // Then
        _store.GetComments(comment.WorkItemId).Should().Contain(c => c.Id == comment.Id);
    }

    [Fact]
    public void HandleCommentUpdated_GivenExternalEvent_UpdatesStore()
    {
        // Given - First add a comment
        var comment = new Comment
        {
            Id = Guid.NewGuid(),
            WorkItemId = Guid.NewGuid(),
            Content = "Original content",
            CreatedAt = DateTime.UtcNow,
            UpdatedAt = DateTime.UtcNow,
            CreatedBy = Guid.NewGuid(),
            UpdatedBy = Guid.NewGuid(),
        };

        _mockClient.Raise(c => c.OnCommentCreated += null, comment);

        var updatedComment = comment with { Content = "Updated content" };

        // When - Simulate WebSocket event
        _mockClient.Raise(c => c.OnCommentUpdated += null, updatedComment);

        // Then
        var stored = _store.GetComments(comment.WorkItemId).First(c => c.Id == comment.Id);
        stored.Content.Should().Be("Updated content");
    }

    [Fact]
    public void HandleCommentDeleted_GivenExternalEvent_RemovesFromStore()
    {
        // Given - First add a comment
        var comment = new Comment
        {
            Id = Guid.NewGuid(),
            WorkItemId = Guid.NewGuid(),
            Content = "To be deleted",
            CreatedAt = DateTime.UtcNow,
            UpdatedAt = DateTime.UtcNow,
            CreatedBy = Guid.NewGuid(),
            UpdatedBy = Guid.NewGuid(),
        };

        _mockClient.Raise(c => c.OnCommentCreated += null, comment);

        // When - Simulate WebSocket delete event
        _mockClient.Raise(c => c.OnCommentDeleted += null, comment.Id);

        // Then
        _store.GetComments(comment.WorkItemId).Should().NotContain(c => c.Id == comment.Id);
    }

    [Fact]
    public async Task RefreshAsync_GivenWorkItemId_LoadsAllComments()
    {
        // Given
        var workItemId = Guid.NewGuid();
        var comments = new List<Comment>
        {
            new Comment
            {
                Id = Guid.NewGuid(),
                WorkItemId = workItemId,
                Content = "Comment 1",
                CreatedAt = DateTime.UtcNow,
                UpdatedAt = DateTime.UtcNow,
                CreatedBy = Guid.NewGuid(),
                UpdatedBy = Guid.NewGuid(),
            },
            new Comment
            {
                Id = Guid.NewGuid(),
                WorkItemId = workItemId,
                Content = "Comment 2",
                CreatedAt = DateTime.UtcNow,
                UpdatedAt = DateTime.UtcNow,
                CreatedBy = Guid.NewGuid(),
                UpdatedBy = Guid.NewGuid(),
            },
        };

        _mockClient
            .Setup(c => c.GetCommentsAsync(workItemId, It.IsAny<CancellationToken>()))
            .ReturnsAsync(comments);

        // When
        await _store.RefreshAsync(workItemId);

        // Then
        _store.GetComments(workItemId).Should().HaveCount(2);
    }

    [Fact]
    public void Dispose_UnsubscribesFromEvents()
    {
        // When
        _store.Dispose();

        // Then - Should not throw when events fire
        _mockClient.Raise(c => c.OnCommentCreated += null, new Comment { Id = Guid.NewGuid() });
        // If it didn't unsubscribe, this would cause issues
    }
}