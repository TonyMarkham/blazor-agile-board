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

public class WorkItemRowTests : BunitContext
{
    public WorkItemRowTests()
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
    public void WorkItemRow_RendersTitle()
    {
        // Arrange
        var viewModel = CreateTestViewModel();

        // Act
        var cut = Render<WorkItemRow>(parameters => parameters
            .Add(p => p.Item, viewModel));

        // Assert
        cut.Markup.Should().Contain("Test Work Item");
    }

    [Fact]
    public void WorkItemRow_RendersTypeIcon()
    {
        // Arrange
        var viewModel = CreateTestViewModel(WorkItemType.Story);

        // Act
        var cut = Render<WorkItemRow>(parameters => parameters
            .Add(p => p.Item, viewModel));

        // Assert
        cut.FindComponents<WorkItemTypeIcon>().Should().HaveCount(1);
    }

    [Fact]
    public void WorkItemRow_RendersStatusBadge()
    {
        // Arrange
        var viewModel = CreateTestViewModel();

        // Act
        var cut = Render<WorkItemRow>(parameters => parameters
            .Add(p => p.Item, viewModel));

        // Assert
        cut.FindComponents<WorkItemStatusBadge>().Should().HaveCount(1);
    }

    [Fact]
    public void WorkItemRow_RendersPriorityBadge()
    {
        // Arrange
        var viewModel = CreateTestViewModel();

        // Act
        var cut = Render<WorkItemRow>(parameters => parameters
            .Add(p => p.Item, viewModel));

        // Assert
        cut.FindComponents<PriorityBadge>().Should().HaveCount(1);
    }

    [Fact]
    public void WorkItemRow_RendersStoryPoints_WhenPresent()
    {
        // Arrange
        var workItem = CreateTestWorkItem() with { StoryPoints = 8 };
        var viewModel = new WorkItemViewModel(workItem);

        // Act
        var cut = Render<WorkItemRow>(parameters => parameters
            .Add(p => p.Item, viewModel));

        // Assert
        cut.Markup.Should().Contain("8");
    }

    [Fact]
    public void WorkItemRow_DoesNotRenderStoryPoints_WhenNull()
    {
        // Arrange
        var workItem = CreateTestWorkItem() with { StoryPoints = null };
        var viewModel = new WorkItemViewModel(workItem);

        // Act
        var cut = Render<WorkItemRow>(parameters => parameters
            .Add(p => p.Item, viewModel));

        // Assert
        var pointsCell = cut.Find(".points-cell");
        pointsCell.InnerHtml.Should().BeEmpty();
    }

    [Fact]
    public void WorkItemRow_AppliesIndent_WhenIndentLevelSet()
    {
        // Arrange
        var viewModel = CreateTestViewModel();

        // Act
        var cut = Render<WorkItemRow>(parameters => parameters
            .Add(p => p.Item, viewModel)
            .Add(p => p.IndentLevel, 2));

        // Assert
        cut.Markup.Should().Contain("width: 40px"); // 2 * 20px
    }

    [Fact]
    public void WorkItemRow_ShowsPendingIndicator_WhenPendingSync()
    {
        // Arrange
        var workItem = CreateTestWorkItem();
        var viewModel = new WorkItemViewModel(workItem, isPendingSync: true);

        // Act
        var cut = Render<WorkItemRow>(parameters => parameters
            .Add(p => p.Item, viewModel));

        // Assert
        cut.Markup.Should().Contain("pending-sync");
        cut.FindComponents<RadzenProgressBarCircular>().Should().HaveCount(1);
    }

    [Fact]
    public void WorkItemRow_HasCorrectAriaLabel()
    {
        // Arrange
        var workItem = CreateTestWorkItem() with
        {
            Title = "My Task",
            ItemType = WorkItemType.Task,
            Status = "in_progress",
            Priority = "high",
            StoryPoints = 3
        };
        var viewModel = new WorkItemViewModel(workItem);

        // Act
        var cut = Render<WorkItemRow>(parameters => parameters
            .Add(p => p.Item, viewModel));

        // Assert
        cut.Markup.Should().Contain("aria-label=");
        cut.Markup.Should().Contain("Task");
        cut.Markup.Should().Contain("My Task");
        cut.Markup.Should().Contain("Status: In Progress");
        cut.Markup.Should().Contain("Priority: High");
    }

    [Fact]
    public void WorkItemRow_AppliesDoneClass_WhenCompleted()
    {
        // Arrange
        var workItem = CreateTestWorkItem() with { Status = "done" };
        var viewModel = new WorkItemViewModel(workItem);

        // Act
        var cut = Render<WorkItemRow>(parameters => parameters
            .Add(p => p.Item, viewModel));

        // Assert
        cut.Markup.Should().Contain("status-done");
    }

    #endregion

    #region Click Handling Tests

    [Fact]
    public async Task WorkItemRow_InvokesOnSelect_WhenClicked()
    {
        // Arrange
        var viewModel = CreateTestViewModel();
        WorkItemViewModel? selectedItem = null;

        var cut = Render<WorkItemRow>(parameters => parameters
            .Add(p => p.Item, viewModel)
            .Add(p => p.OnSelect, EventCallback.Factory.Create<WorkItemViewModel>(
                this, item => selectedItem = item)));

        // Act
        var row = cut.Find(".work-item-row");
        await cut.InvokeAsync(() => row.Click());

        // Assert
        selectedItem.Should().Be(viewModel);
    }

    [Fact]
    public async Task WorkItemRow_InvokesOnEdit_WhenEditButtonClicked()
    {
        // Arrange
        var viewModel = CreateTestViewModel();
        WorkItemViewModel? editedItem = null;

        var cut = Render<WorkItemRow>(parameters => parameters
            .Add(p => p.Item, viewModel)
            .Add(p => p.IsConnected, true)
            .Add(p => p.OnEdit, EventCallback.Factory.Create<WorkItemViewModel>(
                this, item => editedItem = item)));

        // Act
        var editButton = cut.FindComponents<RadzenButton>()
            .First(b => b.Instance.Icon == "edit");
        await cut.InvokeAsync(() => editButton.Instance.Click.InvokeAsync(new MouseEventArgs()));

        // Assert
        editedItem.Should().Be(viewModel);
    }

    [Fact]
    public async Task WorkItemRow_InvokesOnDelete_WhenDeleteButtonClicked()
    {
        // Arrange
        var viewModel = CreateTestViewModel();
        WorkItemViewModel? deletedItem = null;

        var cut = Render<WorkItemRow>(parameters => parameters
            .Add(p => p.Item, viewModel)
            .Add(p => p.IsConnected, true)
            .Add(p => p.OnDelete, EventCallback.Factory.Create<WorkItemViewModel>(
                this, item => deletedItem = item)));

        // Act
        var deleteButton = cut.FindComponents<RadzenButton>()
            .First(b => b.Instance.Icon == "delete");
        await cut.InvokeAsync(() => deleteButton.Instance.Click.InvokeAsync(new MouseEventArgs()));

        // Assert
        deletedItem.Should().Be(viewModel);
    }

    #endregion

    #region Disabled States Tests

    [Fact]
    public void WorkItemRow_DisablesActions_WhenDisconnected()
    {
        // Arrange
        var viewModel = CreateTestViewModel();

        // Act
        var cut = Render<WorkItemRow>(parameters => parameters
            .Add(p => p.Item, viewModel)
            .Add(p => p.IsConnected, false));

        // Assert
        var buttons = cut.FindComponents<RadzenButton>();
        buttons.Should().AllSatisfy(b => b.Instance.Disabled.Should().BeTrue());
    }

    [Fact]
    public void WorkItemRow_DisablesActions_WhenPendingSync()
    {
        // Arrange
        var workItem = CreateTestWorkItem();
        var viewModel = new WorkItemViewModel(workItem, isPendingSync: true);

        // Act
        var cut = Render<WorkItemRow>(parameters => parameters
            .Add(p => p.Item, viewModel)
            .Add(p => p.IsConnected, true));

        // Assert
        var buttons = cut.FindComponents<RadzenButton>();
        buttons.Should().AllSatisfy(b => b.Instance.Disabled.Should().BeTrue());
    }

    [Fact]
    public void WorkItemRow_EnablesActions_WhenConnectedAndNotPending()
    {
        // Arrange
        var viewModel = CreateTestViewModel();

        // Act
        var cut = Render<WorkItemRow>(parameters => parameters
            .Add(p => p.Item, viewModel)
            .Add(p => p.IsConnected, true));

        // Assert
        var buttons = cut.FindComponents<RadzenButton>();
        buttons.Should().AllSatisfy(b => b.Instance.Disabled.Should().BeFalse());
    }

    #endregion

    #region Keyboard Navigation Tests

    [Fact]
    public void WorkItemRow_HasTabIndex()
    {
        // Arrange
        var viewModel = CreateTestViewModel();

        // Act
        var cut = Render<WorkItemRow>(parameters => parameters
            .Add(p => p.Item, viewModel));

        // Assert
        cut.Markup.Should().Contain("tabindex=\"0\"");
    }

    [Fact]
    public void WorkItemRow_HasRoleRow()
    {
        // Arrange
        var viewModel = CreateTestViewModel();

        // Act
        var cut = Render<WorkItemRow>(parameters => parameters
            .Add(p => p.Item, viewModel));

        // Assert
        cut.Markup.Should().Contain("role=\"row\"");
    }

    [Fact]
    public void WorkItemRow_EditButtonHasAriaLabel()
    {
        // Arrange
        var workItem = CreateTestWorkItem() with { Title = "My Task" };
        var viewModel = new WorkItemViewModel(workItem);

        // Act
        var cut = Render<WorkItemRow>(parameters => parameters
            .Add(p => p.Item, viewModel));

        // Assert
        cut.Markup.Should().Contain("aria-label=\"Edit My Task\"");
    }

    [Fact]
    public void WorkItemRow_DeleteButtonHasAriaLabel()
    {
        // Arrange
        var workItem = CreateTestWorkItem() with { Title = "My Task" };
        var viewModel = new WorkItemViewModel(workItem);

        // Act
        var cut = Render<WorkItemRow>(parameters => parameters
            .Add(p => p.Item, viewModel));

        // Assert
        cut.Markup.Should().Contain("aria-label=\"Delete My Task\"");
    }

    #endregion

    #region Helper Methods

    private static WorkItemViewModel CreateTestViewModel(WorkItemType type = WorkItemType.Story)
    {
        return new WorkItemViewModel(CreateTestWorkItem() with { ItemType = type });
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
