using FluentAssertions;
using ProjectManagement.Core.Models;
using Xunit;

namespace ProjectManagement.Core.Tests.Models;

public class WorkItemTests
{
    [Fact]
    public void GetDisplayKey_WithValidProjectKey_ReturnsFormattedKey()
    {
        // Arrange
        var workItem = new WorkItem { ItemNumber = 123 };

        // Act
        var displayKey = workItem.GetDisplayKey("PROJ");

        // Assert
        displayKey.Should().Be("PROJ-123");
    }

    [Fact]
    public void GetDisplayKey_WithNullProjectKey_ThrowsArgumentException()
    {
        // Arrange
        var workItem = new WorkItem { ItemNumber = 123 };

        // Act
        var act = () => workItem.GetDisplayKey(null!);

        // Assert
        act.Should().Throw<ArgumentException>()
            .WithParameterName("projectKey");
    }

    [Fact]
    public void GetDisplayKey_WithEmptyProjectKey_ThrowsArgumentException()
    {
        // Arrange
        var workItem = new WorkItem { ItemNumber = 123 };

        // Act
        var act = () => workItem.GetDisplayKey("");

        // Assert
        act.Should().Throw<ArgumentException>()
            .WithParameterName("projectKey");
    }

    [Fact]
    public void GetDisplayKey_WithWhitespaceProjectKey_ThrowsArgumentException()
    {
        // Arrange
        var workItem = new WorkItem { ItemNumber = 123 };

        // Act
        var act = () => workItem.GetDisplayKey("   ");

        // Assert
        act.Should().Throw<ArgumentException>()
            .WithParameterName("projectKey");
    }
}