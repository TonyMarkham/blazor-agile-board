using Bunit;
using FluentAssertions;
using Microsoft.Extensions.DependencyInjection;
using Moq;
using ProjectManagement.Components.WorkItems;
using ProjectManagement.Core.Interfaces;
using ProjectManagement.Core.Models;
using ProjectManagement.Core.ViewModels;
using ProjectManagement.Services.State;
using Radzen;
using Radzen.Blazor;

namespace ProjectManagement.Components.Tests.WorkItems;

public class KanbanBoardTests : BunitContext
{
    private readonly Mock<IWorkItemStore> _workItemStoreMock;
    private readonly Mock<ISprintStore> _sprintStoreMock;
    private readonly AppState _appState;
    private readonly Guid _projectId = Guid.NewGuid();

    public KanbanBoardTests()
    {
        Services.AddScoped<DialogService>();
        Services.AddScoped<NotificationService>();
        Services.AddScoped<TooltipService>();
        Services.AddScoped<ContextMenuService>();

        _workItemStoreMock = new Mock<IWorkItemStore>();
        _sprintStoreMock = new Mock<ISprintStore>();
        var projectStoreMock = new Mock<IProjectStore>();

        var mockClient = new Mock<IWebSocketClient>();
        mockClient.Setup(c => c.State).Returns(ConnectionState.Connected);
        mockClient.Setup(c => c.Health).Returns(Mock.Of<IConnectionHealth>());

        _appState = new AppState(
            mockClient.Object,
            _workItemStoreMock.Object,
            _sprintStoreMock.Object,
            projectStoreMock.Object,
            Mock.Of<Microsoft.Extensions.Logging.ILogger<AppState>>());

        Services.AddSingleton(_appState);
        Services.AddSingleton(_workItemStoreMock.Object);
        Services.AddSingleton(_sprintStoreMock.Object);
        Services.AddSingleton<IProjectStore>(projectStoreMock.Object);
        Services.AddScoped<ViewModelFactory>();

        JSInterop.Mode = JSRuntimeMode.Loose;
    }

    #region Column Rendering Tests

    [Fact]
    public async Task KanbanBoard_RendersFiveColumns()
    {
        // Arrange
        SetupEmptyStore();

        // Act
        var cut = Render<KanbanBoard>(parameters => parameters
            .Add(p => p.ProjectId, _projectId));

        // Wait for columns to render
        await cut.WaitForAssertionAsync(() => 
            cut.FindComponents<KanbanColumn>().Should().HaveCount(5),
            timeout: TimeSpan.FromSeconds(5));
    }

    [Fact]
    public async Task KanbanBoard_RendersBacklogColumn()
    {
        // Arrange
        SetupEmptyStore();

        // Act
        var cut = Render<KanbanBoard>(parameters => parameters
            .Add(p => p.ProjectId, _projectId));

        // Assert
        await cut.WaitForAssertionAsync(() => 
            cut.Markup.Should().Contain("Backlog"),
            timeout: TimeSpan.FromSeconds(5));
    }

    [Fact]
    public async Task KanbanBoard_RendersTodoColumn()
    {
        // Arrange
        SetupEmptyStore();

        // Act
        var cut = Render<KanbanBoard>(parameters => parameters
            .Add(p => p.ProjectId, _projectId));

        // Assert
        await cut.WaitForAssertionAsync(() => 
            cut.Markup.Should().Contain("To Do"),
            timeout: TimeSpan.FromSeconds(5));
    }

    [Fact]
    public async Task KanbanBoard_RendersInProgressColumn()
    {
        // Arrange
        SetupEmptyStore();

        // Act
        var cut = Render<KanbanBoard>(parameters => parameters
            .Add(p => p.ProjectId, _projectId));

        // Assert
        await cut.WaitForAssertionAsync(() => 
            cut.Markup.Should().Contain("In Progress"),
            timeout: TimeSpan.FromSeconds(5));
    }

    [Fact]
    public async Task KanbanBoard_RendersReviewColumn()
    {
        // Arrange
        SetupEmptyStore();

        // Act
        var cut = Render<KanbanBoard>(parameters => parameters
            .Add(p => p.ProjectId, _projectId));

        // Assert
        await cut.WaitForAssertionAsync(() => 
            cut.Markup.Should().Contain("Review"),
            timeout: TimeSpan.FromSeconds(5));
    }

    [Fact]
    public async Task KanbanBoard_RendersDoneColumn()
    {
        // Arrange
        SetupEmptyStore();

        // Act
        var cut = Render<KanbanBoard>(parameters => parameters
            .Add(p => p.ProjectId, _projectId));

        // Assert
        await cut.WaitForAssertionAsync(() => 
            cut.Markup.Should().Contain("Done"),
            timeout: TimeSpan.FromSeconds(5));
    }

    #endregion

    #region Filter Tests

    [Fact]
    public async Task KanbanBoard_RendersTypeFilter()
    {
        // Arrange
        SetupEmptyStore();

        // Act
        var cut = Render<KanbanBoard>(parameters => parameters
            .Add(p => p.ProjectId, _projectId));

        // Assert
        await cut.WaitForAssertionAsync(() => 
            cut.Markup.Should().Contain("All Types"),
            timeout: TimeSpan.FromSeconds(5));
    }

    [Fact]
    public async Task KanbanBoard_RendersHideDoneCheckbox()
    {
        // Arrange
        SetupEmptyStore();

        // Act
        var cut = Render<KanbanBoard>(parameters => parameters
            .Add(p => p.ProjectId, _projectId));

        // Assert
        await cut.WaitForAssertionAsync(() => 
        {
            cut.Markup.Should().Contain("Hide Done");
            cut.FindComponents<RadzenCheckBox<bool>>().Should().HaveCount(1);
        }, timeout: TimeSpan.FromSeconds(5));
    }

    [Fact]
    public async Task KanbanBoard_ShowsItemCount()
    {
        // Arrange
        var items = new[]
        {
            CreateWorkItem("Item 1", WorkItemType.Story, "backlog"),
            CreateWorkItem("Item 2", WorkItemType.Story, "todo"),
            CreateWorkItem("Item 3", WorkItemType.Task, "in_progress"),
            CreateWorkItem("Item 4", WorkItemType.Task, "review"),
            CreateWorkItem("Item 5", WorkItemType.Epic, "done")
        };
        SetupStoreWithItems(items);

        // Act
        var cut = Render<KanbanBoard>(parameters => parameters
            .Add(p => p.ProjectId, _projectId));

        // Assert
        await cut.WaitForAssertionAsync(() => 
            cut.Markup.Should().Contain("5 items"),
            timeout: TimeSpan.FromSeconds(5));
    }

    [Fact]
    public async Task KanbanBoard_FiltersByType_WhenTypeSelected()
    {
        // Arrange
        var items = new[]
        {
            CreateWorkItem("Epic 1", WorkItemType.Epic, "backlog"),
            CreateWorkItem("Story 1", WorkItemType.Story, "backlog"),
            CreateWorkItem("Task 1", WorkItemType.Task, "backlog")
        };
        SetupStoreWithItems(items);

        var cut = Render<KanbanBoard>(parameters => parameters
            .Add(p => p.ProjectId, _projectId));

        await cut.WaitForAssertionAsync(() => 
            cut.FindComponents<KanbanCard>().Should().HaveCount(3),
            timeout: TimeSpan.FromSeconds(5));

        // Act - Filter by Story type
        var typeDropdown = cut.FindComponents<RadzenDropDown<WorkItemType?>>()
            .First(d => d.Instance.Placeholder == "All Types");
        
        await cut.InvokeAsync(async () =>
        {
            await typeDropdown.Instance.ValueChanged.InvokeAsync(WorkItemType.Story).ConfigureAwait(false);
            await typeDropdown.Instance.Change.InvokeAsync(WorkItemType.Story).ConfigureAwait(false);
        });

        // Assert
        await cut.WaitForAssertionAsync(() =>
        {
            cut.Markup.Should().Contain("Story 1");
            cut.Markup.Should().NotContain("Epic 1");
            cut.Markup.Should().NotContain("Task 1");
        }, timeout: TimeSpan.FromSeconds(2));
    }

    [Fact]
    public async Task KanbanBoard_HidesDoneColumn_WhenCheckboxChecked()
    {
        // Arrange
        var items = new[]
        {
            CreateWorkItem("Backlog Item", WorkItemType.Story, "backlog"),
            CreateWorkItem("Done Item", WorkItemType.Story, "done")
        };
        SetupStoreWithItems(items);

        var cut = Render<KanbanBoard>(parameters => parameters
            .Add(p => p.ProjectId, _projectId));

        await cut.WaitForAssertionAsync(() => 
                cut.FindComponents<KanbanColumn>().Should().HaveCount(5),
            timeout: TimeSpan.FromSeconds(5));

        // Verify Done item is initially visible
        cut.Markup.Should().Contain("Done Item");

        // Act - Check the "Hide Done" checkbox
        var hideDoneCheckbox = cut.FindComponent<RadzenCheckBox<bool>>();
        await cut.InvokeAsync(async () =>
        {
            await hideDoneCheckbox.Instance.ValueChanged.InvokeAsync(true).ConfigureAwait(false);
            await hideDoneCheckbox.Instance.Change.InvokeAsync(true).ConfigureAwait(false);
        });

        // Assert - Done item should not be visible (or column may have hidden class)
        await cut.WaitForAssertionAsync(() =>
        {
            // The Done item should no longer appear in the markup
            cut.Markup.Should().NotContain("Done Item");
            // Backlog item should still be visible
            cut.Markup.Should().Contain("Backlog Item");
        }, timeout: TimeSpan.FromSeconds(2));
    }

    #endregion

    #region Card Distribution Tests

    [Fact]
    public async Task KanbanBoard_DistributesCards_ToCorrectColumns()
    {
        // Arrange
        var items = new[]
        {
            CreateWorkItem("Backlog Item", WorkItemType.Story, "backlog"),
            CreateWorkItem("Todo Item", WorkItemType.Story, "todo"),
            CreateWorkItem("In Progress Item", WorkItemType.Story, "in_progress"),
            CreateWorkItem("Review Item", WorkItemType.Story, "review"),
            CreateWorkItem("Done Item", WorkItemType.Story, "done")
        };
        SetupStoreWithItems(items);

        // Act
        var cut = Render<KanbanBoard>(parameters => parameters
            .Add(p => p.ProjectId, _projectId));

        // Assert - Each column should have one card
        await cut.WaitForAssertionAsync(() =>
        {
            var columns = cut.FindComponents<KanbanColumn>();
            columns.Should().HaveCount(5);
            
            // Each column should render its respective item
            cut.Markup.Should().Contain("Backlog Item");
            cut.Markup.Should().Contain("Todo Item");
            cut.Markup.Should().Contain("In Progress Item");
            cut.Markup.Should().Contain("Review Item");
            cut.Markup.Should().Contain("Done Item");
        }, timeout: TimeSpan.FromSeconds(5));
    }

    #endregion

    #region Accessibility Tests

    [Fact]
    public async Task KanbanBoard_HasRoleApplication()
    {
        // Arrange
        SetupEmptyStore();

        // Act
        var cut = Render<KanbanBoard>(parameters => parameters
            .Add(p => p.ProjectId, _projectId));

        // Assert
        await cut.WaitForAssertionAsync(() => 
            cut.Markup.Should().Contain("role=\"application\""),
            timeout: TimeSpan.FromSeconds(5));
    }

    [Fact]
    public async Task KanbanBoard_HasAriaLabel()
    {
        // Arrange
        SetupEmptyStore();

        // Act
        var cut = Render<KanbanBoard>(parameters => parameters
            .Add(p => p.ProjectId, _projectId));

        // Assert
        await cut.WaitForAssertionAsync(() => 
            cut.Markup.Should().Contain("aria-label=\"Kanban board\""),
            timeout: TimeSpan.FromSeconds(5));
    }

    [Fact]
    public async Task KanbanBoard_HasScreenReaderInstructions()
    {
        // Arrange
        SetupEmptyStore();

        // Act
        var cut = Render<KanbanBoard>(parameters => parameters
            .Add(p => p.ProjectId, _projectId));

        // Assert
        await cut.WaitForAssertionAsync(() => 
        {
            cut.Markup.Should().Contain("kanban-instructions");
            cut.Markup.Should().Contain("Use arrow keys");
        }, timeout: TimeSpan.FromSeconds(5));
    }

    [Fact]
    public async Task KanbanBoard_HasLiveRegion()
    {
        // Arrange
        SetupEmptyStore();

        // Act
        var cut = Render<KanbanBoard>(parameters => parameters
            .Add(p => p.ProjectId, _projectId));

        // Assert
        await cut.WaitForAssertionAsync(() => 
            cut.Markup.Should().Contain("aria-live=\"polite\""),
            timeout: TimeSpan.FromSeconds(5));
    }

    #endregion

    #region KanbanColumn Tests

    [Fact]
    public void KanbanColumn_RendersTitle()
    {
        // Act
        var cut = Render<KanbanColumn>(parameters => parameters
            .Add(p => p.Status, "backlog")
            .Add(p => p.Title, "Backlog")
            .Add(p => p.Items, Enumerable.Empty<WorkItemViewModel>()));

        // Assert
        cut.Markup.Should().Contain("Backlog");
    }

    [Fact]
    public void KanbanColumn_RendersItemCount()
    {
        // Arrange
        var items = new List<WorkItemViewModel>
        {
            new(CreateWorkItem("Item 1", WorkItemType.Story, "backlog")),
            new(CreateWorkItem("Item 2", WorkItemType.Story, "backlog")),
            new(CreateWorkItem("Item 3", WorkItemType.Story, "backlog"))
        };

        // Act
        var cut = Render<KanbanColumn>(parameters => parameters
            .Add(p => p.Status, "backlog")
            .Add(p => p.Title, "Backlog")
            .Add(p => p.Items, items));

        // Assert
        cut.Markup.Should().Contain("3");
    }

    [Fact]
    public void KanbanColumn_ShowsEmptyMessage_WhenNoItems()
    {
        // Act
        var cut = Render<KanbanColumn>(parameters => parameters
            .Add(p => p.Status, "backlog")
            .Add(p => p.Title, "Backlog")
            .Add(p => p.Items, Enumerable.Empty<WorkItemViewModel>()));

        // Assert
        cut.Markup.Should().Contain("No items");
    }

    [Fact]
    public void KanbanColumn_HasListboxRole()
    {
        // Act
        var cut = Render<KanbanColumn>(parameters => parameters
            .Add(p => p.Status, "backlog")
            .Add(p => p.Title, "Backlog")
            .Add(p => p.Items, Enumerable.Empty<WorkItemViewModel>()));

        // Assert
        cut.Markup.Should().Contain("role=\"listbox\"");
    }

    [Fact]
    public void KanbanColumn_HasAriaLabel()
    {
        // Act
        var cut = Render<KanbanColumn>(parameters => parameters
            .Add(p => p.Status, "backlog")
            .Add(p => p.Title, "Backlog")
            .Add(p => p.Items, Enumerable.Empty<WorkItemViewModel>()));

        // Assert
        cut.Markup.Should().Contain("aria-label=\"Backlog column");
    }

    [Fact]
    public void KanbanColumn_AppliesDragTargetClass_WhenIsDragTarget()
    {
        // Act
        var cut = Render<KanbanColumn>(parameters => parameters
            .Add(p => p.Status, "backlog")
            .Add(p => p.Title, "Backlog")
            .Add(p => p.Items, Enumerable.Empty<WorkItemViewModel>())
            .Add(p => p.IsDragTarget, true));

        // Assert
        cut.Markup.Should().Contain("drag-target");
    }

    #endregion

    #region Helper Methods

    private void SetupEmptyStore()
    {
        _workItemStoreMock.Setup(s => s.GetByProject(It.IsAny<Guid>()))
            .Returns(Array.Empty<WorkItem>());
        _workItemStoreMock.Setup(s => s.GetById(It.IsAny<Guid>()))
            .Returns<Guid>(_ => null);
    }

    private void SetupStoreWithItems(WorkItem[] items)
    {
        _workItemStoreMock.Setup(s => s.GetByProject(_projectId))
            .Returns(items);
        _workItemStoreMock.Setup(s => s.GetById(It.IsAny<Guid>()))
            .Returns<Guid>(id => items.FirstOrDefault(i => i.Id == id));
    }

    private WorkItem CreateWorkItem(string title, WorkItemType type, string status)
    {
        return new WorkItem
        {
            Id = Guid.NewGuid(),
            ProjectId = _projectId,
            ItemType = type,
            Title = title,
            Status = status,
            Priority = "medium",
            Position = 0,
            Version = 1,
            CreatedAt = DateTime.UtcNow,
            UpdatedAt = DateTime.UtcNow,
            CreatedBy = Guid.NewGuid(),
            UpdatedBy = Guid.NewGuid()
        };
    }

    #endregion
}
