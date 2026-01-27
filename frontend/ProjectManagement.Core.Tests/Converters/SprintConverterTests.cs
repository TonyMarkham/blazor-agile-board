using FluentAssertions;
using ProjectManagement.Core.Converters;
using ProjectManagement.Core.Models;
using Xunit;

namespace ProjectManagement.Core.Tests.Converters;

public class SprintConverterTests
{
    [Fact]
    public void ToDomain_GivenValidProto_ReturnsCorrectSprint()
    {
        // Given
        var proto = new Proto.Sprint
        {
            Id = "550e8400-e29b-41d4-a716-446655440000",
            ProjectId = "660e8400-e29b-41d4-a716-446655440000",
            Name = "Sprint 1",
            Goal = "Complete MVP features",
            Status = Proto.SprintStatus.Active,
            StartDate = 1704067200, // 2024-01-01 00:00:00 UTC
            EndDate = 1705276800, // 2024-01-15 00:00:00 UTC
            Version = 3,
            CreatedAt = 1704067200,
            UpdatedAt = 1704153600,
            CreatedBy = "770e8400-e29b-41d4-a716-446655440000",
            UpdatedBy = "770e8400-e29b-41d4-a716-446655440000",
        };

        // When
        var result = ProtoConverter.ToDomain(proto);

        // Then
        result.Id.Should().Be(Guid.Parse("550e8400-e29b-41d4-a716-446655440000"));
        result.ProjectId.Should().Be(Guid.Parse("660e8400-e29b-41d4-a716-446655440000"));
        result.Name.Should().Be("Sprint 1");
        result.Goal.Should().Be("Complete MVP features");
        result.Status.Should().Be(SprintStatus.Active);
        result.Version.Should().Be(3);
        result.CreatedBy.Should().Be(Guid.Parse("770e8400-e29b-41d4-a716-446655440000"));
    }
    
    [Fact]
    public void ToDomain_GivenProtoWithEmptyGoal_ReturnsNullGoal()
    {
        // Given
        var proto = new Proto.Sprint
        {
            Id = "550e8400-e29b-41d4-a716-446655440000",
            ProjectId = "660e8400-e29b-41d4-a716-446655440000",
            Name = "Sprint 1",
            // Goal not set - defaults to empty string in protobuf
            Status = Proto.SprintStatus.Planned,
            StartDate = 1704067200,
            EndDate = 1705276800,
            Version = 1,
            CreatedAt = 1704067200,
            UpdatedAt = 1704067200,
            CreatedBy = "770e8400-e29b-41d4-a716-446655440000",
            UpdatedBy = "770e8400-e29b-41d4-a716-446655440000",
        };

        // When
        var result = ProtoConverter.ToDomain(proto);

        // Then
        result.Goal.Should().BeNull();
    }

    [Fact]
    public void ToDomain_GivenDeletedSprint_ReturnsDeletedAtSet()
    {
        // Given
        var proto = new Proto.Sprint
        {
            Id = "550e8400-e29b-41d4-a716-446655440000",
            ProjectId = "660e8400-e29b-41d4-a716-446655440000",
            Name = "Old Sprint",
            Status = Proto.SprintStatus.Cancelled,
            StartDate = 1704067200,
            EndDate = 1705276800,
            Version = 5,
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
    public void ToProto_GivenCreateRequest_ReturnsCorrectProto()
    {
        // Given
        var request = new CreateSprintRequest
        {
            ProjectId = Guid.Parse("660e8400-e29b-41d4-a716-446655440000"),
            Name = "Sprint 2",
            Goal = "Implement authentication",
            StartDate = new DateTime(2024, 2, 1, 0, 0, 0, DateTimeKind.Utc),
            EndDate = new DateTime(2024, 2, 15, 0, 0, 0, DateTimeKind.Utc),
        };

        // When
        var result = ProtoConverter.ToProto(request);

        // Then
        result.ProjectId.Should().Be("660e8400-e29b-41d4-a716-446655440000");
        result.Name.Should().Be("Sprint 2");
        result.Goal.Should().Be("Implement authentication");
        result.StartDate.Should().BeGreaterThan(0);
        result.EndDate.Should().BeGreaterThan(result.StartDate);
    }

    [Fact]
    public void ToProto_GivenUpdateRequest_ReturnsCorrectProto()
    {
        // Given
        var request = new UpdateSprintRequest
        {
            SprintId = Guid.Parse("550e8400-e29b-41d4-a716-446655440000"),
            ExpectedVersion = 5,
            Name = "Updated Name",
        };

        // When
        var result = ProtoConverter.ToProto(request);

        // Then
        result.SprintId.Should().Be("550e8400-e29b-41d4-a716-446655440000");
        result.ExpectedVersion.Should().Be(5);
        result.Name.Should().Be("Updated Name");
        result.Goal.Should().BeEmpty(); // Default, not set
    }

    [Fact]
    public void SprintStatus_Conversions_AreCorrect()
    {
        // Test all status conversions
        ProtoConverter.ToDomain(new Proto.Sprint
        {
            Id = Guid.NewGuid().ToString(),
            ProjectId = Guid.NewGuid().ToString(),
            Name = "Test",
            Status = Proto.SprintStatus.Planned,
            StartDate = 1704067200,
            EndDate = 1705276800,
            Version = 1,
            CreatedAt = 1704067200,
            UpdatedAt = 1704067200,
            CreatedBy = Guid.NewGuid().ToString(),
            UpdatedBy = Guid.NewGuid().ToString(),
        }).Status.Should().Be(SprintStatus.Planned);

        ProtoConverter.ToDomain(new Proto.Sprint
        {
            Id = Guid.NewGuid().ToString(),
            ProjectId = Guid.NewGuid().ToString(),
            Name = "Test",
            Status = Proto.SprintStatus.Active,
            StartDate = 1704067200,
            EndDate = 1705276800,
            Version = 1,
            CreatedAt = 1704067200,
            UpdatedAt = 1704067200,
            CreatedBy = Guid.NewGuid().ToString(),
            UpdatedBy = Guid.NewGuid().ToString(),
        }).Status.Should().Be(SprintStatus.Active);

        ProtoConverter.ToDomain(new Proto.Sprint
        {
            Id = Guid.NewGuid().ToString(),
            ProjectId = Guid.NewGuid().ToString(),
            Name = "Test",
            Status = Proto.SprintStatus.Completed,
            StartDate = 1704067200,
            EndDate = 1705276800,
            Version = 1,
            CreatedAt = 1704067200,
            UpdatedAt = 1704067200,
            CreatedBy = Guid.NewGuid().ToString(),
            UpdatedBy = Guid.NewGuid().ToString(),
        }).Status.Should().Be(SprintStatus.Completed);

        ProtoConverter.ToDomain(new Proto.Sprint
        {
            Id = Guid.NewGuid().ToString(),
            ProjectId = Guid.NewGuid().ToString(),
            Name = "Test",
            Status = Proto.SprintStatus.Cancelled,
            StartDate = 1704067200,
            EndDate = 1705276800,
            Version = 1,
            CreatedAt = 1704067200,
            UpdatedAt = 1704067200,
            CreatedBy = Guid.NewGuid().ToString(),
            UpdatedBy = Guid.NewGuid().ToString(),
        }).Status.Should().Be(SprintStatus.Cancelled);
    }
}