using FluentAssertions;
using Moq;
using ProjectManagement.Core.Interfaces;
using ProjectManagement.Core.Models;
using ProjectManagement.Core.ViewModels;

namespace ProjectManagement.Components.Tests.ViewModels;

public class ViewModelFactoryTests
{
    private readonly Mock<IWorkItemStore> _workItemStore;
    private readonly Mock<ISprintStore> _sprintStore;
    private readonly Mock<IProjectStore> _projectStore;
    private readonly ViewModelFactory _factory;

    public ViewModelFactoryTests()
    {
        _workItemStore = new Mock<IWorkItemStore>();
        _sprintStore = new Mock<ISprintStore>();
        _projectStore = new Mock<IProjectStore>();
        _factory = new ViewModelFactory(_workItemStore.Object, _sprintStore.Object, _projectStore.Object);
    }

    #region Constructor Tests

    [Fact]
    public void Constructor_ThrowsArgumentNullException_WhenWorkItemStoreIsNull()
    {
        // Act
        var act = () => new ViewModelFactory(null!, _sprintStore.Object, _projectStore.Object);

        // Assert
        act.Should().Throw<ArgumentNullException>()
            .WithParameterName("workItemStore");
    }

    [Fact]
    public void Constructor_ThrowsArgumentNullException_WhenSprintStoreIsNull()
    {
        // Act
        var act = () => new ViewModelFactory(_workItemStore.Object, null!, _projectStore.Object);

        // Assert
        act.Should().Throw<ArgumentNullException>()
            .WithParameterName("sprintStore");
    }
    
    [Fact]
    public void Constructor_ThrowsArgumentNullException_WhenProjectStoreIsNull()
    {
        // Act
        var act = () => new ViewModelFactory(_workItemStore.Object, _sprintStore.Object, null!);

        // Assert
        act.Should().Throw<ArgumentNullException>()
            .WithParameterName("projectStore");
    }

    #endregion

    #region Create WorkItem Tests

    [Fact]
    public void Create_WorkItem_ThrowsArgumentNullException_WhenItemIsNull()
    {
        // Act
        var act = () => _factory.Create((WorkItem)null!);

        // Assert
        act.Should().Throw<ArgumentNullException>();
    }

    [Fact]
    public void Create_WorkItem_ReturnsViewModelWithCorrectModel()
    {
        // Arrange
        var workItem = CreateTestWorkItem();
        _workItemStore.Setup(s => s.IsPending(workItem.Id)).Returns(false);

        // Act
        var viewModel = _factory.Create(workItem);

        // Assert
        viewModel.Model.Should().BeSameAs(workItem);
        viewModel.Id.Should().Be(workItem.Id);
        viewModel.Title.Should().Be(workItem.Title);
    }

    [Fact]
    public void Create_WorkItem_SetsIsPendingSyncFalse_WhenNotPending()
    {
        // Arrange
        var workItem = CreateTestWorkItem();
        _workItemStore.Setup(s => s.IsPending(workItem.Id)).Returns(false);

        // Act
        var viewModel = _factory.Create(workItem);

        // Assert
        viewModel.IsPendingSync.Should().BeFalse();
    }

    [Fact]
    public void Create_WorkItem_SetsIsPendingSyncTrue_WhenPending()
    {
        // Arrange
        var workItem = CreateTestWorkItem();
        _workItemStore.Setup(s => s.IsPending(workItem.Id)).Returns(true);

        // Act
        var viewModel = _factory.Create(workItem);

        // Assert
        viewModel.IsPendingSync.Should().BeTrue();
    }

    [Fact]
    public void Create_WorkItem_CallsIsPendingWithCorrectId()
    {
        // Arrange
        var workItem = CreateTestWorkItem();
        _workItemStore.Setup(s => s.IsPending(workItem.Id)).Returns(false);

        // Act
        _factory.Create(workItem);

        // Assert
        _workItemStore.Verify(s => s.IsPending(workItem.Id), Times.Once);
    }

    #endregion

    #region Create Sprint Tests

    [Fact]
    public void Create_Sprint_ThrowsArgumentNullException_WhenSprintIsNull()
    {
        // Act
        var act = () => _factory.Create((Sprint)null!);

        // Assert
        act.Should().Throw<ArgumentNullException>();
    }

    [Fact]
    public void Create_Sprint_ReturnsViewModelWithCorrectModel()
    {
        // Arrange
        var sprint = CreateTestSprint();
        _sprintStore.Setup(s => s.IsPending(sprint.Id)).Returns(false);

        // Act
        var viewModel = _factory.Create(sprint);

        // Assert
        viewModel.Model.Should().BeSameAs(sprint);
        viewModel.Id.Should().Be(sprint.Id);
        viewModel.Name.Should().Be(sprint.Name);
    }

    [Fact]
    public void Create_Sprint_SetsIsPendingSyncFalse_WhenNotPending()
    {
        // Arrange
        var sprint = CreateTestSprint();
        _sprintStore.Setup(s => s.IsPending(sprint.Id)).Returns(false);

        // Act
        var viewModel = _factory.Create(sprint);

        // Assert
        viewModel.IsPendingSync.Should().BeFalse();
    }

    [Fact]
    public void Create_Sprint_SetsIsPendingSyncTrue_WhenPending()
    {
        // Arrange
        var sprint = CreateTestSprint();
        _sprintStore.Setup(s => s.IsPending(sprint.Id)).Returns(true);

        // Act
        var viewModel = _factory.Create(sprint);

        // Assert
        viewModel.IsPendingSync.Should().BeTrue();
    }

    #endregion

    #region CreateMany Tests

    [Fact]
    public void CreateMany_WorkItems_ThrowsArgumentNullException_WhenItemsIsNull()
    {
        // Act
        var act = () => _factory.CreateMany((IEnumerable<WorkItem>)null!);

        // Assert
        act.Should().Throw<ArgumentNullException>();
    }

    [Fact]
    public void CreateMany_WorkItems_ReturnsEmptyList_WhenItemsIsEmpty()
    {
        // Act
        var viewModels = _factory.CreateMany(Enumerable.Empty<WorkItem>());

        // Assert
        viewModels.Should().BeEmpty();
    }

    [Fact]
    public void CreateMany_WorkItems_ReturnsCorrectNumberOfViewModels()
    {
        // Arrange
        var items = new List<WorkItem>
        {
            CreateTestWorkItem(),
            CreateTestWorkItem(),
            CreateTestWorkItem()
        };
        _workItemStore.Setup(s => s.IsPending(It.IsAny<Guid>())).Returns(false);

        // Act
        var viewModels = _factory.CreateMany(items);

        // Assert
        viewModels.Should().HaveCount(3);
    }

    [Fact]
    public void CreateMany_WorkItems_ChecksPendingStateForEachItem()
    {
        // Arrange
        var item1 = CreateTestWorkItem();
        var item2 = CreateTestWorkItem();
        _workItemStore.Setup(s => s.IsPending(item1.Id)).Returns(true);
        _workItemStore.Setup(s => s.IsPending(item2.Id)).Returns(false);

        // Act
        var viewModels = _factory.CreateMany(new[] { item1, item2 });

        // Assert
        viewModels[0].IsPendingSync.Should().BeTrue();
        viewModels[1].IsPendingSync.Should().BeFalse();
    }

    [Fact]
    public void CreateMany_Sprints_ThrowsArgumentNullException_WhenSprintsIsNull()
    {
        // Act
        var act = () => _factory.CreateMany((IEnumerable<Sprint>)null!);

        // Assert
        act.Should().Throw<ArgumentNullException>();
    }

    [Fact]
    public void CreateMany_Sprints_ReturnsCorrectNumberOfViewModels()
    {
        // Arrange
        var sprints = new List<Sprint>
        {
            CreateTestSprint(),
            CreateTestSprint()
        };
        _sprintStore.Setup(s => s.IsPending(It.IsAny<Guid>())).Returns(false);

        // Act
        var viewModels = _factory.CreateMany(sprints);

        // Assert
        viewModels.Should().HaveCount(2);
    }

    #endregion

    #region CreateWithPendingState Tests

    [Fact]
    public void CreateWithPendingState_WorkItem_SetsExplicitPendingState()
    {
        // Arrange
        var workItem = CreateTestWorkItem();

        // Act
        var viewModel = _factory.CreateWithPendingState(workItem, true);

        // Assert
        viewModel.IsPendingSync.Should().BeTrue();
        _workItemStore.Verify(s => s.IsPending(It.IsAny<Guid>()), Times.Never);
    }

    [Fact]
    public void CreateWithPendingState_Sprint_SetsExplicitPendingState()
    {
        // Arrange
        var sprint = CreateTestSprint();

        // Act
        var viewModel = _factory.CreateWithPendingState(sprint, false);

        // Assert
        viewModel.IsPendingSync.Should().BeFalse();
        _sprintStore.Verify(s => s.IsPending(It.IsAny<Guid>()), Times.Never);
    }

    #endregion

    #region Helper Methods

    private static WorkItem CreateTestWorkItem() => new()
    {
        Id = Guid.NewGuid(),
        Title = "Test Work Item",
        ItemType = WorkItemType.Story,
        ProjectId = Guid.NewGuid(),
        Status = "backlog",
        Priority = "medium",
        Position = 1,
        Version = 1,
        CreatedAt = DateTime.UtcNow,
        UpdatedAt = DateTime.UtcNow,
        CreatedBy = Guid.NewGuid(),
        UpdatedBy = Guid.NewGuid()
    };

    private static Sprint CreateTestSprint() => new()
    {
        Id = Guid.NewGuid(),
        Name = "Sprint 1",
        ProjectId = Guid.NewGuid(),
        StartDate = DateTime.UtcNow,
        EndDate = DateTime.UtcNow.AddDays(14),
        Status = SprintStatus.Planned
    };

    #endregion
}
