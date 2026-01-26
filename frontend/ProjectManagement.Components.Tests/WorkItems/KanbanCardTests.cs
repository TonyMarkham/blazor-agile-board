using Bunit;
using FluentAssertions;
using Microsoft.AspNetCore.Components;
using Microsoft.AspNetCore.Components.Web;
using Microsoft.Extensions.DependencyInjection;
using ProjectManagement.Components.WorkItems;
using ProjectManagement.Core.Models;
using ProjectManagement.Core.ViewModels;
using Radzen;
using Radzen.Blazor;

namespace ProjectManagement.Components.Tests.WorkItems;

public class KanbanCardTests : BunitContext
{
    public KanbanCardTests()
    {
        // Register Radzen services
        Services.AddScoped<DialogService>();
        Services.AddScoped<NotificationService>();
        Services.AddScoped<TooltipService>();
        Services.AddScoped<ContextMenuService>();

        // Set JSInterop to loose mode
        JSInterop.Mode = JSRuntimeMode.Loose;
    }

    #region Rendering Tests

    [Fact]
    public void KanbanCard_RendersTitle()
    {
        // Arrange
        var viewModel = CreateTestViewModel();

        // Act
        var cut = Render<KanbanCard>(parameters => parameters
            .Add(p => p.Item, viewModel));

        // Assert
        cut.Markup.Should().Contain("Test Work Item");
    }

    [Fact]
    public void KanbanCard_RendersTypeIcon()
    {
        // Arrange
        var viewModel = CreateTestViewModel();

        // Act
        var cut = Render<KanbanCard>(parameters => parameters
            .Add(p => p.Item, viewModel));

        // Assert
        cut.FindComponents<WorkItemTypeIcon>().Should().HaveCount(1);
    }

    [Fact]
    public void KanbanCard_RendersPriorityBadge_WithoutLabel()
    {
        // Arrange
        var viewModel = CreateTestViewModel();

        // Act
        var cut = Render<KanbanCard>(parameters => parameters
            .Add(p => p.Item, viewModel));

        // Assert
        var priorityBadge = cut.FindComponent<PriorityBadge>();
        priorityBadge.Instance.ShowLabel.Should().BeFalse();
    }

    [Fact]
    public void KanbanCard_RendersStoryPoints_WhenPresent()
    {
        // Arrange
        var workItem = CreateTestWorkItem() with { StoryPoints = 5 };
        var viewModel = new WorkItemViewModel(workItem);

        // Act
        var cut = Render<KanbanCard>(parameters => parameters
            .Add(p => p.Item, viewModel));

        // Assert
        cut.Markup.Should().Contain("5");
    }

    [Fact]
    public void KanbanCard_ShowsPendingIndicator_WhenPendingSync()
    {
        // Arrange
        var workItem = CreateTestWorkItem();
        var viewModel = new WorkItemViewModel(workItem, isPendingSync: true);

        // Act
        var cut = Render<KanbanCard>(parameters => parameters
            .Add(p => p.Item, viewModel));

        // Assert
        cut.Markup.Should().Contain("pending-sync");
        cut.FindComponents<RadzenProgressBarCircular>().Should().HaveCount(1);
    }

    [Fact]
    public void KanbanCard_HasKanbanCardClass()
    {
        // Arrange
        var viewModel = CreateTestViewModel();

        // Act
        var cut = Render<KanbanCard>(parameters => parameters
            .Add(p => p.Item, viewModel));

        // Assert
        cut.Markup.Should().Contain("kanban-card");
    }

    #endregion

    #region Click Handling Tests

    [Fact]
    public async Task KanbanCard_InvokesOnClick_WhenClicked()
    {
        // Arrange
        var viewModel = CreateTestViewModel();
        WorkItemViewModel? clickedItem = null;

        var cut = Render<KanbanCard>(parameters => parameters
            .Add(p => p.Item, viewModel)
            .Add(p => p.OnClick, EventCallback.Factory.Create<WorkItemViewModel>(
                this, item => clickedItem = item)));

        // Act
        var card = cut.Find(".kanban-card");
        await cut.InvokeAsync(() => card.Click());

        // Assert
        clickedItem.Should().Be(viewModel);
    }

    [Fact]
    public async Task KanbanCard_InvokesOnEdit_WhenEditButtonClicked()
    {
        // Arrange
        var viewModel = CreateTestViewModel();
        WorkItemViewModel? editedItem = null;

        var cut = Render<KanbanCard>(parameters => parameters
            .Add(p => p.Item, viewModel)
            .Add(p => p.IsConnected, true)
            .Add(p => p.OnEdit, EventCallback.Factory.Create<WorkItemViewModel>(
                this, item => editedItem = item)));

        // Act
        var editButton = cut.FindComponent<RadzenButton>();
        await cut.InvokeAsync(() => editButton.Instance.Click.InvokeAsync(new MouseEventArgs()));

        // Assert
        editedItem.Should().Be(viewModel);
    }

    #endregion

    #region Disabled States Tests

    [Fact]
    public void KanbanCard_DisablesEditButton_WhenDisconnected()
    {
        // Arrange
        var viewModel = CreateTestViewModel();

        // Act
        var cut = Render<KanbanCard>(parameters => parameters
            .Add(p => p.Item, viewModel)
            .Add(p => p.IsConnected, false));

        // Assert
        var button = cut.FindComponent<RadzenButton>();
        button.Instance.Disabled.Should().BeTrue();
    }

    [Fact]
    public void KanbanCard_HidesEditButton_WhenPendingSync()
    {
        // Arrange
        var workItem = CreateTestWorkItem();
        var viewModel = new WorkItemViewModel(workItem, isPendingSync: true);

        // Act
        var cut = Render<KanbanCard>(parameters => parameters
            .Add(p => p.Item, viewModel)
            .Add(p => p.IsConnected, true));

        // Assert
        // When pending, spinner is shown instead of edit button
        cut.FindComponents<RadzenProgressBarCircular>().Should().HaveCount(1);
    }

    #endregion

    #region Accessibility Tests

    [Fact]
    public void KanbanCard_HasRoleListItem()
    {
        // Arrange
        var viewModel = CreateTestViewModel();

        // Act
        var cut = Render<KanbanCard>(parameters => parameters
            .Add(p => p.Item, viewModel));

        // Assert
        cut.Markup.Should().Contain("role=\"listitem\"");
    }

    [Fact]
    public void KanbanCard_HasAriaLabel()
    {
        // Arrange
        var workItem = CreateTestWorkItem() with
        {
            Title = "My Task",
            ItemType = WorkItemType.Task,
            Priority = "high"
        };
        var viewModel = new WorkItemViewModel(workItem);

        // Act
        var cut = Render<KanbanCard>(parameters => parameters
            .Add(p => p.Item, viewModel));

        // Assert
        cut.Markup.Should().Contain("aria-label=");
        cut.Markup.Should().Contain("Task");
        cut.Markup.Should().Contain("My Task");
        cut.Markup.Should().Contain("Priority: High");
    }

    [Fact]
    public void KanbanCard_HasTabIndex()
    {
        // Arrange
        var viewModel = CreateTestViewModel();

        // Act
        var cut = Render<KanbanCard>(parameters => parameters
            .Add(p => p.Item, viewModel));

        // Assert
        cut.Markup.Should().Contain("tabindex=\"0\"");
    }

    [Fact]
    public void KanbanCard_EditButtonHasAriaLabel()
    {
        // Arrange
        var workItem = CreateTestWorkItem() with { Title = "My Task" };
        var viewModel = new WorkItemViewModel(workItem);

        // Act
        var cut = Render<KanbanCard>(parameters => parameters
            .Add(p => p.Item, viewModel)
            .Add(p => p.IsConnected, true));

        // Assert
        cut.Markup.Should().Contain("aria-label=\"Edit My Task\"");
    }

    #endregion

    #region Progress Bar Tests

    [Fact]
    public void KanbanCard_ShowsTaskProgress_ForStoryWithChildren()
    {
        // Arrange
        var workItem = CreateTestWorkItem() with { ItemType = WorkItemType.Story };
        var progress = new ChildProgress
        {
            ByStatus = new Dictionary<string, int> { ["done"] = 2, ["todo"] = 3 },
            Total = 5,
            Completed = 2
        };
        var viewModel = new WorkItemViewModel(workItem, isPendingSync: false)
        {
            TaskProgress = progress
        };

        // Act
        var cut = Render<KanbanCard>(parameters => parameters
            .Add(p => p.Item, viewModel));

        // Assert
        cut.FindComponents<ChildProgressBar>().Should().HaveCount(1);
        cut.Markup.Should().Contain("2/5");
    }

    [Fact]
    public void KanbanCard_ShowsBothProgressBars_ForEpicWithStoriesAndTasks()
    {
        // Arrange
        var workItem = CreateTestWorkItem() with { ItemType = WorkItemType.Epic };
        var storyProgress = new ChildProgress
        {
            ByStatus = new Dictionary<string, int> { ["done"] = 1, ["in_progress"] = 2 },
            Total = 3,
            Completed = 1
        };
        var taskProgress = new ChildProgress
        {
            ByStatus = new Dictionary<string, int> { ["done"] = 5, ["todo"] = 5 },
            Total = 10,
            Completed = 5
        };
        var viewModel = new WorkItemViewModel(workItem, isPendingSync: false)
        {
            StoryProgress = storyProgress,
            TaskProgress = taskProgress
        };

        // Act
        var cut = Render<KanbanCard>(parameters => parameters
            .Add(p => p.Item, viewModel));

        // Assert
        cut.FindComponents<ChildProgressBar>().Should().HaveCount(2);
        cut.Markup.Should().Contain("Stories");
        cut.Markup.Should().Contain("Tasks");
    }

    [Fact]
    public void KanbanCard_HidesProgressBar_ForTaskItems()
    {
        // Arrange
        var workItem = CreateTestWorkItem() with { ItemType = WorkItemType.Task };
        var viewModel = new WorkItemViewModel(workItem, isPendingSync: false);

        // Act
        var cut = Render<KanbanCard>(parameters => parameters
            .Add(p => p.Item, viewModel));

        // Assert
        cut.FindComponents<ChildProgressBar>().Should().BeEmpty();
    }

    [Fact]
    public void KanbanCard_HidesProgressBar_ForStoryWithNoChildren()
    {
        // Arrange
        var workItem = CreateTestWorkItem() with { ItemType = WorkItemType.Story };
        var viewModel = new WorkItemViewModel(workItem, isPendingSync: false)
        {
            TaskProgress = ChildProgress.Empty
        };

        // Act
        var cut = Render<KanbanCard>(parameters => parameters
            .Add(p => p.Item, viewModel));

        // Assert
        cut.FindComponents<ChildProgressBar>().Should().BeEmpty();
    }

    #endregion

    #region Helper Methods

    private static WorkItemViewModel CreateTestViewModel()
    {
        return new WorkItemViewModel(CreateTestWorkItem());
    }

    private static WorkItem CreateTestWorkItem() => new()
    {
        Id = Guid.NewGuid(),
        Title = "Test Work Item",
        Description = "Test Description",
        ItemType = WorkItemType.Story,
        ProjectId = Guid.NewGuid(),
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
