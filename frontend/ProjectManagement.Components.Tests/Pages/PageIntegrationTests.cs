using Bunit;
using FluentAssertions;
using Microsoft.Extensions.DependencyInjection;
using Moq;
using ProjectManagement.Components.Shared;
using ProjectManagement.Components.WorkItems;
using ProjectManagement.Core.Interfaces;
using ProjectManagement.Core.Models;
using ProjectManagement.Core.ViewModels;
using ProjectManagement.Services.State;
using ProjectManagement.Wasm.Pages;
using Radzen;

namespace ProjectManagement.Components.Tests.Pages;

public class PageIntegrationTests : BunitContext
{
    private readonly Mock<IWorkItemStore> _workItemStoreMock;
    private readonly Mock<ISprintStore> _sprintStoreMock;
    private readonly AppState _appState;

    public PageIntegrationTests()
    {
        Services.AddScoped<DialogService>();
        Services.AddScoped<NotificationService>();
        Services.AddScoped<TooltipService>();
        Services.AddScoped<ContextMenuService>();

        _workItemStoreMock = new Mock<IWorkItemStore>();
        _sprintStoreMock = new Mock<ISprintStore>();

        var mockClient = new Mock<IWebSocketClient>();
        mockClient.Setup(c => c.State).Returns(ConnectionState.Connected);
        mockClient.Setup(c => c.Health).Returns(Mock.Of<IConnectionHealth>());

        _appState = new AppState(
            mockClient.Object,
            _workItemStoreMock.Object,
            _sprintStoreMock.Object,
            Mock.Of<Microsoft.Extensions.Logging.ILogger<AppState>>());

        Services.AddSingleton(_appState);
        Services.AddSingleton(_workItemStoreMock.Object);
        Services.AddSingleton(_sprintStoreMock.Object);
        Services.AddScoped<ViewModelFactory>();

        JSInterop.Mode = JSRuntimeMode.Loose;
    }

    #region Home Page Tests

    [Fact]
    public async Task HomePage_RendersPageTitle()
    {
        // Arrange
        SetupEmptyStore();

        // Act
        var cut = Render<Home>();

        // Assert
        await cut.WaitForAssertionAsync(() => 
            cut.Markup.Should().Contain("Welcome to Agile Board"),
            timeout: TimeSpan.FromSeconds(5));
    }

    [Fact]
    public async Task HomePage_ShowsEmptyState_WhenNoProjects()
    {
        // Arrange
        SetupEmptyStore();

        // Act
        var cut = Render<Home>();

        // Assert
        await cut.WaitForAssertionAsync(() => 
        {
            cut.FindComponents<EmptyState>().Should().HaveCount(1);
            cut.Markup.Should().Contain("Get Started");
        }, timeout: TimeSpan.FromSeconds(5));
    }

    [Fact]
    public async Task HomePage_ShowsCreateProjectButton_WhenNoProjects()
    {
        // Arrange
        SetupEmptyStore();

        // Act
        var cut = Render<Home>();

        // Assert
        await cut.WaitForAssertionAsync(() => 
            cut.Markup.Should().Contain("Create Project"),
            timeout: TimeSpan.FromSeconds(5));
    }

    #endregion

    #region ProjectDetail Page Tests

    [Fact]
    public async Task ProjectDetailPage_ShowsNotFound_WhenProjectDoesNotExist()
    {
        // Arrange
        _workItemStoreMock.Setup(s => s.GetById(It.IsAny<Guid>()))
            .Returns((WorkItem?)null);

        // Act
        var cut = Render<ProjectDetail>(parameters => parameters
            .Add(p => p.ProjectId, Guid.NewGuid()));

        // Assert
        await cut.WaitForAssertionAsync(() => 
        {
            cut.FindComponents<EmptyState>().Should().HaveCount(1);
            cut.Markup.Should().Contain("Project Not Found");
        }, timeout: TimeSpan.FromSeconds(5));
    }

    [Fact]
    public async Task ProjectDetailPage_RendersBreadcrumbs()
    {
        // Arrange
        var projectId = Guid.NewGuid();
        var project = CreateProject("My Project") with { Id = projectId };
        _workItemStoreMock.Setup(s => s.GetById(projectId)).Returns(project);
        _workItemStoreMock.Setup(s => s.GetByProject(projectId)).Returns(Array.Empty<WorkItem>());

        // Act
        var cut = Render<ProjectDetail>(parameters => parameters
            .Add(p => p.ProjectId, projectId));

        // Assert
        await cut.WaitForAssertionAsync(() => 
        {
            cut.Markup.Should().Contain("breadcrumbs");
            cut.Markup.Should().Contain("Home");
        }, timeout: TimeSpan.FromSeconds(5));
    }

    [Fact]
    public async Task ProjectDetailPage_RendersProjectTitle()
    {
        // Arrange
        var projectId = Guid.NewGuid();
        var project = CreateProject("My Project") with { Id = projectId };
        _workItemStoreMock.Setup(s => s.GetById(projectId)).Returns(project);
        _workItemStoreMock.Setup(s => s.GetByProject(projectId)).Returns(Array.Empty<WorkItem>());

        // Act
        var cut = Render<ProjectDetail>(parameters => parameters
            .Add(p => p.ProjectId, projectId));

        // Assert
        await cut.WaitForAssertionAsync(() => 
            cut.Markup.Should().Contain("My Project"),
            timeout: TimeSpan.FromSeconds(5));
    }

    [Fact]
    public async Task ProjectDetailPage_RendersViewTabs()
    {
        // Arrange
        var projectId = Guid.NewGuid();
        var project = CreateProject("My Project") with { Id = projectId };
        _workItemStoreMock.Setup(s => s.GetById(projectId)).Returns(project);
        _workItemStoreMock.Setup(s => s.GetByProject(projectId)).Returns(Array.Empty<WorkItem>());

        // Act
        var cut = Render<ProjectDetail>(parameters => parameters
            .Add(p => p.ProjectId, projectId));

        // Assert
        await cut.WaitForAssertionAsync(() => 
        {
            cut.Markup.Should().Contain("List");
            cut.Markup.Should().Contain("Board");
        }, timeout: TimeSpan.FromSeconds(5));
    }

    [Fact]
    public async Task ProjectDetailPage_DefaultsToKanbanView()
    {
        // Arrange
        var projectId = Guid.NewGuid();
        var project = CreateProject("My Project") with { Id = projectId };
        _workItemStoreMock.Setup(s => s.GetById(projectId)).Returns(project);
        _workItemStoreMock.Setup(s => s.GetByProject(projectId)).Returns(Array.Empty<WorkItem>());

        // Act
        var cut = Render<ProjectDetail>(parameters => parameters
            .Add(p => p.ProjectId, projectId));

        // Assert
        await cut.WaitForAssertionAsync(() => 
            cut.FindComponents<KanbanBoard>().Should().HaveCount(1),
            timeout: TimeSpan.FromSeconds(5));
    }

    [Fact]
    public async Task ProjectDetailPage_RendersNewWorkItemButton()
    {
        // Arrange
        var projectId = Guid.NewGuid();
        var project = CreateProject("My Project") with { Id = projectId };
        _workItemStoreMock.Setup(s => s.GetById(projectId)).Returns(project);
        _workItemStoreMock.Setup(s => s.GetByProject(projectId)).Returns(Array.Empty<WorkItem>());

        // Act
        var cut = Render<ProjectDetail>(parameters => parameters
            .Add(p => p.ProjectId, projectId));

        // Assert
        await cut.WaitForAssertionAsync(() => 
            cut.Markup.Should().Contain("New Work Item"),
            timeout: TimeSpan.FromSeconds(5));
    }

    #endregion

    #region WorkItemDetail Page Tests

    [Fact]
    public async Task WorkItemDetailPage_ShowsNotFound_WhenItemDoesNotExist()
    {
        // Arrange
        _workItemStoreMock.Setup(s => s.GetById(It.IsAny<Guid>()))
            .Returns((WorkItem?)null);

        // Act
        var cut = Render<WorkItemDetail>(parameters => parameters
            .Add(p => p.WorkItemId, Guid.NewGuid()));

        // Assert
        await cut.WaitForAssertionAsync(() => 
        {
            cut.FindComponents<EmptyState>().Should().HaveCount(1);
            cut.Markup.Should().Contain("Work Item Not Found");
        }, timeout: TimeSpan.FromSeconds(5));
    }

    [Fact]
    public async Task WorkItemDetailPage_RendersBreadcrumbs()
    {
        // Arrange
        var workItemId = Guid.NewGuid();
        var projectId = Guid.NewGuid();
        var workItem = CreateWorkItem("My Task", WorkItemType.Task) with
        {
            Id = workItemId,
            ProjectId = projectId
        };
        var project = CreateProject("My Project") with { Id = projectId };

        _workItemStoreMock.Setup(s => s.GetById(workItemId)).Returns(workItem);
        _workItemStoreMock.Setup(s => s.GetById(projectId)).Returns(project);
        _workItemStoreMock.Setup(s => s.GetChildren(workItemId)).Returns(Array.Empty<WorkItem>());

        // Act
        var cut = Render<WorkItemDetail>(parameters => parameters
            .Add(p => p.WorkItemId, workItemId));

        // Assert
        await cut.WaitForAssertionAsync(() => 
        {
            cut.Markup.Should().Contain("breadcrumbs");
            cut.Markup.Should().Contain("Home");
            cut.Markup.Should().Contain("My Project");
        }, timeout: TimeSpan.FromSeconds(5));
    }

    [Fact]
    public async Task WorkItemDetailPage_RendersWorkItemTitle()
    {
        // Arrange
        var workItemId = Guid.NewGuid();
        var workItem = CreateWorkItem("My Task", WorkItemType.Task) with { Id = workItemId };

        _workItemStoreMock.Setup(s => s.GetById(workItemId)).Returns(workItem);
        _workItemStoreMock.Setup(s => s.GetById(workItem.ProjectId)).Returns((WorkItem?)null);
        _workItemStoreMock.Setup(s => s.GetChildren(workItemId)).Returns(Array.Empty<WorkItem>());

        // Act
        var cut = Render<WorkItemDetail>(parameters => parameters
            .Add(p => p.WorkItemId, workItemId));

        // Assert
        await cut.WaitForAssertionAsync(() => 
            cut.Markup.Should().Contain("My Task"),
            timeout: TimeSpan.FromSeconds(5));
    }

    [Fact]
    public async Task WorkItemDetailPage_RendersStatusBadge()
    {
        // Arrange
        var workItemId = Guid.NewGuid();
        var workItem = CreateWorkItem("My Task", WorkItemType.Task) with 
        { 
            Id = workItemId,
            Status = "in_progress" 
        };

        _workItemStoreMock.Setup(s => s.GetById(workItemId)).Returns(workItem);
        _workItemStoreMock.Setup(s => s.GetById(workItem.ProjectId)).Returns((WorkItem?)null);
        _workItemStoreMock.Setup(s => s.GetChildren(workItemId)).Returns(Array.Empty<WorkItem>());

        // Act
        var cut = Render<WorkItemDetail>(parameters => parameters
            .Add(p => p.WorkItemId, workItemId));

        // Assert
        await cut.WaitForAssertionAsync(() => 
            cut.FindComponents<WorkItemStatusBadge>().Should().HaveCount(1),
            timeout: TimeSpan.FromSeconds(5));
    }

    [Fact]
    public async Task WorkItemDetailPage_RendersPriorityBadge()
    {
        // Arrange
        var workItemId = Guid.NewGuid();
        var workItem = CreateWorkItem("My Task", WorkItemType.Task) with { Id = workItemId };

        _workItemStoreMock.Setup(s => s.GetById(workItemId)).Returns(workItem);
        _workItemStoreMock.Setup(s => s.GetById(workItem.ProjectId)).Returns((WorkItem?)null);
        _workItemStoreMock.Setup(s => s.GetChildren(workItemId)).Returns(Array.Empty<WorkItem>());

        // Act
        var cut = Render<WorkItemDetail>(parameters => parameters
            .Add(p => p.WorkItemId, workItemId));

        // Assert
        await cut.WaitForAssertionAsync(() => 
            cut.FindComponents<PriorityBadge>().Should().HaveCount(1),
            timeout: TimeSpan.FromSeconds(5));
    }

    [Fact]
    public async Task WorkItemDetailPage_RendersEditButton()
    {
        // Arrange
        var workItemId = Guid.NewGuid();
        var workItem = CreateWorkItem("My Task", WorkItemType.Task) with { Id = workItemId };

        _workItemStoreMock.Setup(s => s.GetById(workItemId)).Returns(workItem);
        _workItemStoreMock.Setup(s => s.GetById(workItem.ProjectId)).Returns((WorkItem?)null);
        _workItemStoreMock.Setup(s => s.GetChildren(workItemId)).Returns(Array.Empty<WorkItem>());

        // Act
        var cut = Render<WorkItemDetail>(parameters => parameters
            .Add(p => p.WorkItemId, workItemId));

        // Assert
        await cut.WaitForAssertionAsync(() => 
            cut.Markup.Should().Contain("Edit"),
            timeout: TimeSpan.FromSeconds(5));
    }

    [Fact]
    public async Task WorkItemDetailPage_RendersDeleteButton()
    {
        // Arrange
        var workItemId = Guid.NewGuid();
        var workItem = CreateWorkItem("My Task", WorkItemType.Task) with { Id = workItemId };

        _workItemStoreMock.Setup(s => s.GetById(workItemId)).Returns(workItem);
        _workItemStoreMock.Setup(s => s.GetById(workItem.ProjectId)).Returns((WorkItem?)null);
        _workItemStoreMock.Setup(s => s.GetChildren(workItemId)).Returns(Array.Empty<WorkItem>());

        // Act
        var cut = Render<WorkItemDetail>(parameters => parameters
            .Add(p => p.WorkItemId, workItemId));

        // Assert
        await cut.WaitForAssertionAsync(() => 
            cut.Markup.Should().Contain("Delete"),
            timeout: TimeSpan.FromSeconds(5));
    }

    [Fact]
    public async Task WorkItemDetailPage_RendersDescription()
    {
        // Arrange
        var workItemId = Guid.NewGuid();
        var workItem = CreateWorkItem("My Task", WorkItemType.Task) with
        {
            Id = workItemId,
            Description = "This is a detailed description"
        };

        _workItemStoreMock.Setup(s => s.GetById(workItemId)).Returns(workItem);
        _workItemStoreMock.Setup(s => s.GetById(workItem.ProjectId)).Returns((WorkItem?)null);
        _workItemStoreMock.Setup(s => s.GetChildren(workItemId)).Returns(Array.Empty<WorkItem>());

        // Act
        var cut = Render<WorkItemDetail>(parameters => parameters
            .Add(p => p.WorkItemId, workItemId));

        // Assert
        await cut.WaitForAssertionAsync(() => 
        {
            cut.Markup.Should().Contain("Description");
            cut.Markup.Should().Contain("This is a detailed description");
        }, timeout: TimeSpan.FromSeconds(5));
    }

    [Fact]
    public async Task WorkItemDetailPage_RendersChildItems_WhenPresent()
    {
        // Arrange
        var workItemId = Guid.NewGuid();
        var workItem = CreateWorkItem("Parent Epic", WorkItemType.Epic) with { Id = workItemId };
        var children = new[]
        {
            CreateWorkItem("Child Story 1", WorkItemType.Story) with { ParentId = workItemId },
            CreateWorkItem("Child Story 2", WorkItemType.Story) with { ParentId = workItemId }
        };

        _workItemStoreMock.Setup(s => s.GetById(workItemId)).Returns(workItem);
        _workItemStoreMock.Setup(s => s.GetById(workItem.ProjectId)).Returns((WorkItem?)null);
        _workItemStoreMock.Setup(s => s.GetChildren(workItemId)).Returns(children);

        // Act
        var cut = Render<WorkItemDetail>(parameters => parameters
            .Add(p => p.WorkItemId, workItemId));

        // Assert
        await cut.WaitForAssertionAsync(() => 
        {
            cut.Markup.Should().Contain("Child Items");
            cut.FindComponents<WorkItemRow>().Should().HaveCount(2);
        }, timeout: TimeSpan.FromSeconds(5));
    }

    [Fact]
    public async Task WorkItemDetailPage_RendersDetailsSidebar()
    {
        // Arrange
        var workItemId = Guid.NewGuid();
        var workItem = CreateWorkItem("My Task", WorkItemType.Task) with { Id = workItemId };

        _workItemStoreMock.Setup(s => s.GetById(workItemId)).Returns(workItem);
        _workItemStoreMock.Setup(s => s.GetById(workItem.ProjectId)).Returns((WorkItem?)null);
        _workItemStoreMock.Setup(s => s.GetChildren(workItemId)).Returns(Array.Empty<WorkItem>());

        // Act
        var cut = Render<WorkItemDetail>(parameters => parameters
            .Add(p => p.WorkItemId, workItemId));

        // Assert
        await cut.WaitForAssertionAsync(() => 
        {
            cut.Markup.Should().Contain("Details");
            cut.Markup.Should().Contain("Type");
            cut.Markup.Should().Contain("Status");
            cut.Markup.Should().Contain("Priority");
            cut.Markup.Should().Contain("Created");
            cut.Markup.Should().Contain("Updated");
            cut.Markup.Should().Contain("Version");
        }, timeout: TimeSpan.FromSeconds(5));
    }

    #endregion

    #region Helper Methods

    private void SetupEmptyStore()
    {
        _workItemStoreMock.Setup(s => s.GetByProject(It.IsAny<Guid>()))
            .Returns(Array.Empty<WorkItem>());
        _workItemStoreMock.Setup(s => s.GetById(It.IsAny<Guid>()))
            .Returns((WorkItem?)null);
        _workItemStoreMock.Setup(s => s.GetChildren(It.IsAny<Guid>()))
            .Returns(Array.Empty<WorkItem>());
    }

    private static WorkItem CreateProject(string title) => new()
    {
        Id = Guid.NewGuid(),
        Title = title,
        Description = "Test Description",
        ProjectId = Guid.Empty,
        Status = "active",
        Priority = "medium",
        Position = 0,
        Version = 1,
        CreatedAt = DateTime.UtcNow,
        UpdatedAt = DateTime.UtcNow,
        CreatedBy = Guid.NewGuid(),
        UpdatedBy = Guid.NewGuid()
    };

    private static WorkItem CreateWorkItem(string title, WorkItemType type) => new()
    {
        Id = Guid.NewGuid(),
        Title = title,
        Description = "Test Description",
        ItemType = type,
        ProjectId = Guid.NewGuid(),
        Status = "backlog",
        Priority = "medium",
        StoryPoints = 5,
        Position = 0,
        Version = 1,
        CreatedAt = DateTime.UtcNow,
        UpdatedAt = DateTime.UtcNow,
        CreatedBy = Guid.NewGuid(),
        UpdatedBy = Guid.NewGuid()
    };

    #endregion
}
