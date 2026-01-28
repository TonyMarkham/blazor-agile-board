using FluentAssertions;
using ProjectManagement.Core.Converters;
using ProjectManagement.Core.Models;
using Proto = ProjectManagement.Core.Proto;

namespace ProjectManagement.Core.Tests.Converters;

public class DependencyConverterTests
{
    [Fact]
    public void ToDomain_ConvertsAllFields()
    {
        // Given
        var proto = new Proto.Dependency
        {
            Id = Guid.NewGuid().ToString(),
            BlockingItemId = Guid.NewGuid().ToString(),
            BlockedItemId = Guid.NewGuid().ToString(),
            DependencyType = Proto.DependencyType.Blocks,
            CreatedAt = DateTimeOffset.UtcNow.ToUnixTimeSeconds(),
            CreatedBy = Guid.NewGuid().ToString(),
        };

        // When
        var result = ProtoConverter.ToDomain(proto);

        // Then
        result.Id.ToString().Should().Be(proto.Id);
        result.BlockingItemId.ToString().Should().Be(proto.BlockingItemId);
        result.BlockedItemId.ToString().Should().Be(proto.BlockedItemId);
        result.Type.Should().Be(DependencyType.Blocks);
        result.DeletedAt.Should().BeNull();
    }

    [Fact]
    public void ToDomain_HandlesRelatesTo()
    {
        // Given
        var proto = new Proto.Dependency
        {
            Id = Guid.NewGuid().ToString(),
            BlockingItemId = Guid.NewGuid().ToString(),
            BlockedItemId = Guid.NewGuid().ToString(),
            DependencyType = Proto.DependencyType.RelatesTo,
            CreatedAt = DateTimeOffset.UtcNow.ToUnixTimeSeconds(),
            CreatedBy = Guid.NewGuid().ToString(),
        };

        // When
        var result = ProtoConverter.ToDomain(proto);

        // Then
        result.Type.Should().Be(DependencyType.RelatesTo);
    }

    [Fact]
    public void ToProto_ConvertsAllFields()
    {
        // Given
        var dep = new Dependency
        {
            Id = Guid.NewGuid(),
            BlockingItemId = Guid.NewGuid(),
            BlockedItemId = Guid.NewGuid(),
            Type = DependencyType.Blocks,
            CreatedAt = DateTime.UtcNow,
            CreatedBy = Guid.NewGuid(),
        };

        // When
        var result = ProtoConverter.ToProto(dep);

        // Then
        result.Id.Should().Be(dep.Id.ToString());
        result.DependencyType.Should().Be(Proto.DependencyType.Blocks);
    }

    [Fact]
    public void RoundTrip_PreservesData()
    {
        // Given
        var original = new Dependency
        {
            Id = Guid.NewGuid(),
            BlockingItemId = Guid.NewGuid(),
            BlockedItemId = Guid.NewGuid(),
            Type = DependencyType.RelatesTo,
            CreatedAt = new DateTime(2024, 1, 15, 10, 0, 0, DateTimeKind.Utc),
            CreatedBy = Guid.NewGuid(),
        };

        // When
        var proto = ProtoConverter.ToProto(original);
        var result = ProtoConverter.ToDomain(proto);

        // Then
        result.Id.Should().Be(original.Id);
        result.BlockingItemId.Should().Be(original.BlockingItemId);
        result.BlockedItemId.Should().Be(original.BlockedItemId);
        result.Type.Should().Be(original.Type);
    }

    [Fact]
    public void DependencyType_Conversion_AllValues()
    {
        // Test all enum values round-trip correctly
        ProtoConverter.ToDomain(Proto.DependencyType.Blocks).Should().Be(DependencyType.Blocks);
        ProtoConverter.ToDomain(Proto.DependencyType.RelatesTo).Should().Be(DependencyType.RelatesTo);

        ProtoConverter.ToProto(DependencyType.Blocks).Should().Be(Proto.DependencyType.Blocks);
        ProtoConverter.ToProto(DependencyType.RelatesTo).Should().Be(Proto.DependencyType.RelatesTo);
    }
}