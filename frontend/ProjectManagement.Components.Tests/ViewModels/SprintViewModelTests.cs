using FluentAssertions;
using ProjectManagement.Core.Models;
using ProjectManagement.Core.ViewModels;

namespace ProjectManagement.Components.Tests.ViewModels;

public class SprintViewModelTests
{
    #region Constructor Tests

    [Fact]
    public void Constructor_ThrowsArgumentNullException_WhenModelIsNull()
    {
        // Act
        var act = () => new SprintViewModel(null!);

        // Assert
        act.Should().Throw<ArgumentNullException>();
    }

    [Fact]
    public void Constructor_SetsModelCorrectly()
    {
        // Arrange
        var sprint = CreateTestSprint();

        // Act
        var viewModel = new SprintViewModel(sprint);

        // Assert
        viewModel.Model.Should().BeSameAs(sprint);
    }

    [Fact]
    public void Constructor_DefaultsIsPendingSyncToFalse()
    {
        // Arrange
        var sprint = CreateTestSprint();

        // Act
        var viewModel = new SprintViewModel(sprint);

        // Assert
        viewModel.IsPendingSync.Should().BeFalse();
    }

    #endregion

    #region Computed Property Tests

    [Theory]
    [InlineData(SprintStatus.Planned, true, false, false)]
    [InlineData(SprintStatus.Active, false, true, false)]
    [InlineData(SprintStatus.Completed, false, false, true)]
    public void StatusBooleans_ReturnCorrectValues(SprintStatus status, bool isPlanned, bool isActive, bool isCompleted)
    {
        // Arrange
        var sprint = CreateTestSprint() with { Status = status };

        // Act
        var viewModel = new SprintViewModel(sprint);

        // Assert
        viewModel.IsPlanned.Should().Be(isPlanned);
        viewModel.IsActive.Should().Be(isActive);
        viewModel.IsCompleted.Should().Be(isCompleted);
    }

    [Theory]
    [InlineData(SprintStatus.Planned, "Planned")]
    [InlineData(SprintStatus.Active, "Active")]
    [InlineData(SprintStatus.Completed, "Completed")]
    public void StatusDisplayName_ReturnsCorrectValue(SprintStatus status, string expected)
    {
        // Arrange
        var sprint = CreateTestSprint() with { Status = status };

        // Act
        var viewModel = new SprintViewModel(sprint);

        // Assert
        viewModel.StatusDisplayName.Should().Be(expected);
    }

    [Fact]
    public void DateRangeDisplay_FormatsCorrectly()
    {
        // Arrange
        var sprint = CreateTestSprint() with
        {
            StartDate = new DateTime(2024, 1, 15),
            EndDate = new DateTime(2024, 1, 29)
        };

        // Act
        var viewModel = new SprintViewModel(sprint);

        // Assert
        viewModel.DateRangeDisplay.Should().Be("Jan 15 - Jan 29");
    }

    [Fact]
    public void DurationDays_CalculatesCorrectly()
    {
        // Arrange
        var sprint = CreateTestSprint() with
        {
            StartDate = new DateTime(2024, 1, 1),
            EndDate = new DateTime(2024, 1, 15)
        };

        // Act
        var viewModel = new SprintViewModel(sprint);

        // Assert
        viewModel.DurationDays.Should().Be(14);
    }

    [Fact]
    public void DaysRemaining_ReturnsNull_WhenNotActive()
    {
        // Arrange
        var sprint = CreateTestSprint() with { Status = SprintStatus.Planned };

        // Act
        var viewModel = new SprintViewModel(sprint);

        // Assert
        viewModel.DaysRemaining.Should().BeNull();
    }

    [Fact]
    public void DaysRemaining_ReturnsValue_WhenActive()
    {
        // Arrange
        var sprint = CreateTestSprint() with
        {
            Status = SprintStatus.Active,
            StartDate = DateTime.UtcNow.Date.AddDays(-7),
            EndDate = DateTime.UtcNow.Date.AddDays(7)
        };

        // Act
        var viewModel = new SprintViewModel(sprint);

        // Assert
        viewModel.DaysRemaining.Should().Be(7);
    }

    [Fact]
    public void DaysRemaining_ReturnsZero_WhenPastEndDate()
    {
        // Arrange
        var sprint = CreateTestSprint() with
        {
            Status = SprintStatus.Active,
            StartDate = DateTime.UtcNow.Date.AddDays(-14),
            EndDate = DateTime.UtcNow.Date.AddDays(-1)
        };

        // Act
        var viewModel = new SprintViewModel(sprint);

        // Assert
        viewModel.DaysRemaining.Should().Be(0);
    }

    [Fact]
    public void ProgressPercent_ReturnsZero_WhenPlanned()
    {
        // Arrange
        var sprint = CreateTestSprint() with { Status = SprintStatus.Planned };

        // Act
        var viewModel = new SprintViewModel(sprint);

        // Assert
        viewModel.ProgressPercent.Should().Be(0);
    }

    [Fact]
    public void ProgressPercent_Returns100_WhenCompleted()
    {
        // Arrange
        var sprint = CreateTestSprint() with { Status = SprintStatus.Completed };

        // Act
        var viewModel = new SprintViewModel(sprint);

        // Assert
        viewModel.ProgressPercent.Should().Be(100);
    }

    [Fact]
    public void ProgressPercent_CalculatesCorrectly_WhenActive()
    {
        // Arrange
        var sprint = CreateTestSprint() with
        {
            Status = SprintStatus.Active,
            StartDate = DateTime.UtcNow.Date.AddDays(-5),
            EndDate = DateTime.UtcNow.Date.AddDays(5)
        };

        // Act
        var viewModel = new SprintViewModel(sprint);

        // Assert
        viewModel.ProgressPercent.Should().BeApproximately(50, 10); // ~50% with some tolerance
    }

    [Fact]
    public void IsOverdue_ReturnsFalse_WhenNotActive()
    {
        // Arrange
        var sprint = CreateTestSprint() with
        {
            Status = SprintStatus.Completed,
            EndDate = DateTime.UtcNow.Date.AddDays(-1)
        };

        // Act
        var viewModel = new SprintViewModel(sprint);

        // Assert
        viewModel.IsOverdue.Should().BeFalse();
    }

    [Fact]
    public void IsOverdue_ReturnsTrue_WhenActiveAndPastEndDate()
    {
        // Arrange
        var sprint = CreateTestSprint() with
        {
            Status = SprintStatus.Active,
            StartDate = DateTime.UtcNow.Date.AddDays(-14),
            EndDate = DateTime.UtcNow.Date.AddDays(-1)
        };

        // Act
        var viewModel = new SprintViewModel(sprint);

        // Assert
        viewModel.IsOverdue.Should().BeTrue();
    }

    #endregion

    #region Equality Tests

    [Fact]
    public void Equals_ReturnsTrue_WhenSameIdStatusAndPendingState()
    {
        // Arrange
        var sprint = CreateTestSprint();
        var viewModel1 = new SprintViewModel(sprint);
        var viewModel2 = new SprintViewModel(sprint);

        // Act & Assert
        viewModel1.Equals(viewModel2).Should().BeTrue();
    }

    [Fact]
    public void Equals_ReturnsFalse_WhenDifferentStatus()
    {
        // Arrange
        var sprint1 = CreateTestSprint() with { Status = SprintStatus.Planned };
        var sprint2 = sprint1 with { Status = SprintStatus.Active };
        var viewModel1 = new SprintViewModel(sprint1);
        var viewModel2 = new SprintViewModel(sprint2);

        // Act & Assert
        viewModel1.Equals(viewModel2).Should().BeFalse();
    }

    #endregion

    #region Helper Methods

    private static Sprint CreateTestSprint() => new()
    {
        Id = Guid.NewGuid(),
        Name = "Sprint 1",
        Goal = "Complete features",
        ProjectId = Guid.NewGuid(),
        StartDate = DateTime.UtcNow.Date,
        EndDate = DateTime.UtcNow.Date.AddDays(14),
        Status = SprintStatus.Planned
    };

    #endregion
}
