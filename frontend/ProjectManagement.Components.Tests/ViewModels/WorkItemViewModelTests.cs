using FluentAssertions;
using ProjectManagement.Core.Models;
using ProjectManagement.Core.ViewModels;

namespace ProjectManagement.Components.Tests.ViewModels;

public class WorkItemViewModelTests
{
    #region Constructor Tests

    [Fact]
    public void Constructor_ThrowsArgumentNullException_WhenModelIsNull()
    {
        // Act
        var act = () => new WorkItemViewModel(null!);

        // Assert
        act.Should().Throw<ArgumentNullException>();
    }

    [Fact]
    public void Constructor_SetsModelCorrectly()
    {
        // Arrange
        var workItem = CreateTestWorkItem();

        // Act
        var viewModel = new WorkItemViewModel(workItem);

        // Assert
        viewModel.Model.Should().BeSameAs(workItem);
    }

    [Fact]
    public void Constructor_DefaultsIsPendingSyncToFalse()
    {
        // Arrange
        var workItem = CreateTestWorkItem();

        // Act
        var viewModel = new WorkItemViewModel(workItem);

        // Assert
        viewModel.IsPendingSync.Should().BeFalse();
    }

    [Fact]
    public void Constructor_SetsIsPendingSyncFromParameter()
    {
        // Arrange
        var workItem = CreateTestWorkItem();

        // Act
        var viewModel = new WorkItemViewModel(workItem, isPendingSync: true);

        // Assert
        viewModel.IsPendingSync.Should().BeTrue();
    }

    #endregion

    #region Property Accessor Tests

    [Fact]
    public void PropertyAccessors_ReturnModelValues()
    {
        // Arrange
        var workItem = CreateTestWorkItem();

        // Act
        var viewModel = new WorkItemViewModel(workItem);

        // Assert
        viewModel.Id.Should().Be(workItem.Id);
        viewModel.Title.Should().Be(workItem.Title);
        viewModel.Description.Should().Be(workItem.Description);
        viewModel.ItemType.Should().Be(workItem.ItemType);
        viewModel.Status.Should().Be(workItem.Status);
        viewModel.Priority.Should().Be(workItem.Priority);
        viewModel.StoryPoints.Should().Be(workItem.StoryPoints);
        viewModel.Position.Should().Be(workItem.Position);
        viewModel.ProjectId.Should().Be(workItem.ProjectId);
        viewModel.ParentId.Should().Be(workItem.ParentId);
        viewModel.SprintId.Should().Be(workItem.SprintId);
        viewModel.AssigneeId.Should().Be(workItem.AssigneeId);
        viewModel.Version.Should().Be(workItem.Version);
    }

    #endregion

    #region Computed Property Tests

    [Theory]
    [InlineData(null, false)]
    [InlineData("2024-01-01", true)]
    public void IsDeleted_ReturnsCorrectValue(string? deletedAtString, bool expected)
    {
        // Arrange
        DateTime? deletedAt = deletedAtString is null ? null : DateTime.Parse(deletedAtString);
        var workItem = CreateTestWorkItem() with { DeletedAt = deletedAt };

        // Act
        var viewModel = new WorkItemViewModel(workItem);

        // Assert
        viewModel.IsDeleted.Should().Be(expected);
    }

    [Theory]
    [InlineData("done", true)]
    [InlineData("backlog", false)]
    [InlineData("in_progress", false)]
    public void IsCompleted_ReturnsCorrectValue(string status, bool expected)
    {
        // Arrange
        var workItem = CreateTestWorkItem() with { Status = status };

        // Act
        var viewModel = new WorkItemViewModel(workItem);

        // Assert
        viewModel.IsCompleted.Should().Be(expected);
    }

    [Theory]
    [InlineData("backlog", "Backlog")]
    [InlineData("todo", "To Do")]
    [InlineData("in_progress", "In Progress")]
    [InlineData("review", "Review")]
    [InlineData("done", "Done")]
    [InlineData("custom", "custom")]
    public void StatusDisplayName_ReturnsCorrectValue(string status, string expected)
    {
        // Arrange
        var workItem = CreateTestWorkItem() with { Status = status };

        // Act
        var viewModel = new WorkItemViewModel(workItem);

        // Assert
        viewModel.StatusDisplayName.Should().Be(expected);
    }

    [Theory]
    [InlineData("critical", "Critical")]
    [InlineData("high", "High")]
    [InlineData("medium", "Medium")]
    [InlineData("low", "Low")]
    [InlineData("custom", "custom")]
    public void PriorityDisplayName_ReturnsCorrectValue(string priority, string expected)
    {
        // Arrange
        var workItem = CreateTestWorkItem() with { Priority = priority };

        // Act
        var viewModel = new WorkItemViewModel(workItem);

        // Assert
        viewModel.PriorityDisplayName.Should().Be(expected);
    }

    [Theory]
    [InlineData("critical", 0)]
    [InlineData("high", 1)]
    [InlineData("medium", 2)]
    [InlineData("low", 3)]
    [InlineData("unknown", 4)]
    public void PrioritySortOrder_ReturnsCorrectValue(string priority, int expected)
    {
        // Arrange
        var workItem = CreateTestWorkItem() with { Priority = priority };

        // Act
        var viewModel = new WorkItemViewModel(workItem);

        // Assert
        viewModel.PrioritySortOrder.Should().Be(expected);
    }

    [Theory]
    [InlineData(WorkItemType.Project, "Project")]
    [InlineData(WorkItemType.Epic, "Epic")]
    [InlineData(WorkItemType.Story, "Story")]
    [InlineData(WorkItemType.Task, "Task")]
    public void ItemTypeDisplayName_ReturnsCorrectValue(WorkItemType type, string expected)
    {
        // Arrange
        var workItem = CreateTestWorkItem() with { ItemType = type };

        // Act
        var viewModel = new WorkItemViewModel(workItem);

        // Assert
        viewModel.ItemTypeDisplayName.Should().Be(expected);
    }

    #endregion

    #region Equality Tests

    [Fact]
    public void Equals_ReturnsFalse_WhenOtherIsNull()
    {
        // Arrange
        var viewModel = new WorkItemViewModel(CreateTestWorkItem());

        // Act & Assert
        viewModel.Equals(null).Should().BeFalse();
    }

    [Fact]
    public void Equals_ReturnsTrue_WhenSameReference()
    {
        // Arrange
        var viewModel = new WorkItemViewModel(CreateTestWorkItem());

        // Act & Assert
        viewModel.Equals(viewModel).Should().BeTrue();
    }

    [Fact]
    public void Equals_ReturnsTrue_WhenSameIdVersionAndPendingState()
    {
        // Arrange
        var workItem = CreateTestWorkItem();
        var viewModel1 = new WorkItemViewModel(workItem, false);
        var viewModel2 = new WorkItemViewModel(workItem, false);

        // Act & Assert
        viewModel1.Equals(viewModel2).Should().BeTrue();
    }

    [Fact]
    public void Equals_ReturnsFalse_WhenDifferentPendingState()
    {
        // Arrange
        var workItem = CreateTestWorkItem();
        var viewModel1 = new WorkItemViewModel(workItem, false);
        var viewModel2 = new WorkItemViewModel(workItem, true);

        // Act & Assert
        viewModel1.Equals(viewModel2).Should().BeFalse();
    }

    [Fact]
    public void Equals_ReturnsFalse_WhenDifferentVersion()
    {
        // Arrange
        var workItem1 = CreateTestWorkItem() with { Version = 1 };
        var workItem2 = workItem1 with { Version = 2 };
        var viewModel1 = new WorkItemViewModel(workItem1);
        var viewModel2 = new WorkItemViewModel(workItem2);

        // Act & Assert
        viewModel1.Equals(viewModel2).Should().BeFalse();
    }

    [Fact]
    public void GetHashCode_ReturnsSameValue_ForEqualViewModels()
    {
        // Arrange
        var workItem = CreateTestWorkItem();
        var viewModel1 = new WorkItemViewModel(workItem);
        var viewModel2 = new WorkItemViewModel(workItem);

        // Act & Assert
        viewModel1.GetHashCode().Should().Be(viewModel2.GetHashCode());
    }

    #endregion

    #region Helper Methods

    private static WorkItem CreateTestWorkItem() => new()
    {
        Id = Guid.NewGuid(),
        Title = "Test Work Item",
        Description = "Test Description",
        ItemType = WorkItemType.Story,
        ProjectId = Guid.NewGuid(),
        ParentId = Guid.NewGuid(),
        SprintId = Guid.NewGuid(),
        AssigneeId = Guid.NewGuid(),
        Status = "backlog",
        Priority = "medium",
        StoryPoints = 5,
        Position = 1,
        Version = 1,
        CreatedAt = DateTime.UtcNow,
        UpdatedAt = DateTime.UtcNow,
        CreatedBy = Guid.NewGuid(),
        UpdatedBy = Guid.NewGuid()
    };

    #endregion
}
