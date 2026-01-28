using FluentAssertions;
using ProjectManagement.Core.Converters;
using ProjectManagement.Core.Models;
using Proto = ProjectManagement.Core.Proto;

namespace ProjectManagement.Core.Tests.Converters;

public class TimeEntryConverterTests
{
    [Fact]
    public void ToDomain_ConvertsAllFields()
    {
        // Given
        var proto = new Proto.TimeEntry
        {
            Id = Guid.NewGuid().ToString(),
            WorkItemId = Guid.NewGuid().ToString(),
            UserId = Guid.NewGuid().ToString(),
            StartedAt = DateTimeOffset.UtcNow.AddHours(-1).ToUnixTimeSeconds(),
            EndedAt = DateTimeOffset.UtcNow.ToUnixTimeSeconds(),
            DurationSeconds = 3600,
            Description = "Test work",
            CreatedAt = DateTimeOffset.UtcNow.AddDays(-1).ToUnixTimeSeconds(),
            UpdatedAt = DateTimeOffset.UtcNow.ToUnixTimeSeconds(),
        };

        // When
        var result = ProtoConverter.ToDomain(proto);

        // Then
        result.Id.ToString().Should().Be(proto.Id);
        result.WorkItemId.ToString().Should().Be(proto.WorkItemId);
        result.UserId.ToString().Should().Be(proto.UserId);
        result.EndedAt.Should().NotBeNull();
        result.DurationSeconds.Should().Be(3600);
        result.Description.Should().Be("Test work");
        result.IsRunning.Should().BeFalse();
    }

    [Fact]
    public void ToDomain_HandlesNullOptionalFields()
    {
        // Given - Proto with no optional fields set
        var proto = new Proto.TimeEntry
        {
            Id = Guid.NewGuid().ToString(),
            WorkItemId = Guid.NewGuid().ToString(),
            UserId = Guid.NewGuid().ToString(),
            StartedAt = DateTimeOffset.UtcNow.ToUnixTimeSeconds(),
            CreatedAt = DateTimeOffset.UtcNow.ToUnixTimeSeconds(),
            UpdatedAt = DateTimeOffset.UtcNow.ToUnixTimeSeconds(),
        };

        // When
        var result = ProtoConverter.ToDomain(proto);

        // Then
        result.EndedAt.Should().BeNull();
        result.DurationSeconds.Should().BeNull();
        result.Description.Should().BeNull();
        result.DeletedAt.Should().BeNull();
        result.IsRunning.Should().BeTrue();
    }

    [Fact]
    public void ToProto_ConvertsAllFields()
    {
        // Given
        var entry = new TimeEntry
        {
            Id = Guid.NewGuid(),
            WorkItemId = Guid.NewGuid(),
            UserId = Guid.NewGuid(),
            StartedAt = DateTime.UtcNow.AddHours(-1),
            EndedAt = DateTime.UtcNow,
            DurationSeconds = 3600,
            Description = "Test",
            CreatedAt = DateTime.UtcNow.AddDays(-1),
            UpdatedAt = DateTime.UtcNow,
        };

        // When
        var result = ProtoConverter.ToProto(entry);

        // Then
        result.Id.Should().Be(entry.Id.ToString());
        result.HasEndedAt.Should().BeTrue();
        result.EndedAt.Should().BeGreaterThan(0);
        result.HasDurationSeconds.Should().BeTrue();
        result.DurationSeconds.Should().Be(3600);
        result.HasDescription.Should().BeTrue();
        result.Description.Should().Be("Test");
    }

    [Fact]
    public void RoundTrip_PreservesData()
    {
        // Given
        var original = new TimeEntry
        {
            Id = Guid.NewGuid(),
            WorkItemId = Guid.NewGuid(),
            UserId = Guid.NewGuid(),
            StartedAt = new DateTime(2024, 1, 15, 10, 0, 0, DateTimeKind.Utc),
            EndedAt = new DateTime(2024, 1, 15, 11, 30, 0, DateTimeKind.Utc),
            DurationSeconds = 5400,
            Description = "Round trip test",
            CreatedAt = new DateTime(2024, 1, 15, 9, 0, 0, DateTimeKind.Utc),
            UpdatedAt = new DateTime(2024, 1, 15, 11, 30, 0, DateTimeKind.Utc),
        };

        // When
        var proto = ProtoConverter.ToProto(original);
        var result = ProtoConverter.ToDomain(proto);

        // Then
        result.Id.Should().Be(original.Id);
        result.WorkItemId.Should().Be(original.WorkItemId);
        result.DurationSeconds.Should().Be(original.DurationSeconds);
        result.Description.Should().Be(original.Description);
    }

    [Fact]
    public void ToDomain_ThrowsOnNull()
    {
        // Given
        Proto.TimeEntry? proto = null;

        // When/Then
        FluentActions.Invoking(() => ProtoConverter.ToDomain(proto!))
            .Should().Throw<ArgumentNullException>();
    }

    [Fact]
    public void TimeEntry_Elapsed_CalculatesCorrectly_ForStoppedEntry()
    {
        // Given
        var entry = new TimeEntry
        {
            StartedAt = DateTime.UtcNow.AddHours(-2),
            EndedAt = DateTime.UtcNow.AddHours(-1),
            DurationSeconds = 3600,
        };

        // When
        var elapsed = entry.Elapsed;

        // Then
        elapsed.TotalSeconds.Should().Be(3600);
    }

    [Fact]
    public void TimeEntry_IsRunning_TrueWhenNoEndedAt()
    {
        // Given
        var entry = new TimeEntry
        {
            StartedAt = DateTime.UtcNow,
            EndedAt = null,
            DeletedAt = null,
        };

        // Then
        entry.IsRunning.Should().BeTrue();
    }

    [Fact]
    public void TimeEntry_IsRunning_FalseWhenDeleted()
    {
        // Given
        var entry = new TimeEntry
        {
            StartedAt = DateTime.UtcNow,
            EndedAt = null,
            DeletedAt = DateTime.UtcNow, // Deleted
        };

        // Then
        entry.IsRunning.Should().BeFalse();
    }
}