using FluentAssertions;
using ProjectManagement.Core.Converters;
using ProjectManagement.Core.Models;
using Xunit;
using Pm = ProjectManagement.Core.Proto;

namespace ProjectManagement.Core.Tests.Converters;

public class CommentConverterTests
{
    [Fact]
    public void ToDomain_GivenValidProto_ReturnsCorrectComment()
    {
        // Given
        var proto = new Proto.Comment
        {
            Id = "550e8400-e29b-41d4-a716-446655440000",
            WorkItemId = "660e8400-e29b-41d4-a716-446655440000",
            Content = "This is a test comment",
            CreatedAt = 1704067200,
            UpdatedAt = 1704153600,
            CreatedBy = "770e8400-e29b-41d4-a716-446655440000",
            UpdatedBy = "770e8400-e29b-41d4-a716-446655440000",
        };

        // When
        var result = ProtoConverter.ToDomain(proto);

        // Then
        result.Id.Should().Be(Guid.Parse("550e8400-e29b-41d4-a716-446655440000"));
        result.WorkItemId.Should().Be(Guid.Parse("660e8400-e29b-41d4-a716-446655440000"));
        result.Content.Should().Be("This is a test comment");
        result.CreatedBy.Should().Be(Guid.Parse("770e8400-e29b-41d4-a716-446655440000"));
        result.UpdatedBy.Should().Be(Guid.Parse("770e8400-e29b-41d4-a716-446655440000"));
        result.DeletedAt.Should().BeNull();
    }

    [Fact]
    public void ToDomain_GivenDeletedComment_ReturnsDeletedAtSet()
    {
        // Given
        var proto = new Proto.Comment
        {
            Id = "550e8400-e29b-41d4-a716-446655440000",
            WorkItemId = "660e8400-e29b-41d4-a716-446655440000",
            Content = "Deleted comment",
            CreatedAt = 1704067200,
            UpdatedAt = 1704153600,
            CreatedBy = "770e8400-e29b-41d4-a716-446655440000",
            UpdatedBy = "770e8400-e29b-41d4-a716-446655440000",
            DeletedAt = 1704240000, // Non-zero means deleted
        };

        // When
        var result = ProtoConverter.ToDomain(proto);

        // Then
        result.DeletedAt.Should().NotBeNull();
    }

    [Fact]
    public void ToDomain_GivenLongContent_PreservesFullContent()
    {
        // Given
        var longContent = new string('x', 5000);
        var proto = new Proto.Comment
        {
            Id = "550e8400-e29b-41d4-a716-446655440000",
            WorkItemId = "660e8400-e29b-41d4-a716-446655440000",
            Content = longContent,
            CreatedAt = 1704067200,
            UpdatedAt = 1704153600,
            CreatedBy = "770e8400-e29b-41d4-a716-446655440000",
            UpdatedBy = "770e8400-e29b-41d4-a716-446655440000",
        };

        // When
        var result = ProtoConverter.ToDomain(proto);

        // Then
        result.Content.Should().Be(longContent);
        result.Content.Length.Should().Be(5000);
    }

    [Fact]
    public void ToProto_GivenCreateRequest_ReturnsCorrectProto()
    {
        // Given
        var request = new CreateCommentRequest
        {
            WorkItemId = Guid.Parse("660e8400-e29b-41d4-a716-446655440000"),
            Content = "New comment content",
        };

        // When
        var result = ProtoConverter.ToProto(request);

        // Then
        result.WorkItemId.Should().Be("660e8400-e29b-41d4-a716-446655440000");
        result.Content.Should().Be("New comment content");
    }

    [Fact]
    public void ToProto_GivenUpdateRequest_ReturnsCorrectProto()
    {
        // Given
        var request = new UpdateCommentRequest
        {
            CommentId = Guid.Parse("550e8400-e29b-41d4-a716-446655440000"),
            Content = "Updated content",
        };

        // When
        var result = ProtoConverter.ToProto(request);

        // Then
        result.CommentId.Should().Be("550e8400-e29b-41d4-a716-446655440000");
        result.Content.Should().Be("Updated content");
    }

    [Fact]
    public void RoundTrip_CreateAndConvert_PreservesData()
    {
        // Given - Create a request
        var createRequest = new CreateCommentRequest
        {
            WorkItemId = Guid.Parse("660e8400-e29b-41d4-a716-446655440000"),
            Content = "Round trip test content",
        };

        // When - Convert to proto and simulate server response
        var protoRequest = ProtoConverter.ToProto(createRequest);

        var protoResponse = new Proto.Comment
        {
            Id = Guid.NewGuid().ToString(),
            WorkItemId = protoRequest.WorkItemId,
            Content = protoRequest.Content,
            CreatedAt = DateTimeOffset.UtcNow.ToUnixTimeSeconds(),
            UpdatedAt = DateTimeOffset.UtcNow.ToUnixTimeSeconds(),
            CreatedBy = Guid.NewGuid().ToString(),
            UpdatedBy = Guid.NewGuid().ToString(),
        };

        var domainComment = ProtoConverter.ToDomain(protoResponse);

        // Then - Data should be preserved
        domainComment.WorkItemId.Should().Be(createRequest.WorkItemId);
        domainComment.Content.Should().Be(createRequest.Content);
    }
}