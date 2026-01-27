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

public class DialogTests : BunitContext
{
    private readonly Mock<DialogService> _dialogServiceMock;

    public DialogTests()
    {
        Services.AddScoped<DialogService>();
        Services.AddScoped<NotificationService>();
        Services.AddScoped<TooltipService>();
        Services.AddScoped<ContextMenuService>();
        
        _dialogServiceMock = new Mock<DialogService>();
        
        JSInterop.Mode = JSRuntimeMode.Loose;
    }

    #region VersionConflictDialog Tests

    [Fact]
    public void VersionConflictDialog_RendersItemTitle()
    {
        // Act
        var cut = Render<VersionConflictDialog>(parameters => parameters
            .Add(p => p.ItemTitle, "My Work Item"));

        // Assert
        cut.Markup.Should().Contain("My Work Item");
    }

    [Fact]
    public void VersionConflictDialog_RendersConflictHeader()
    {
        // Act
        var cut = Render<VersionConflictDialog>(parameters => parameters
            .Add(p => p.ItemTitle, "Test"));

        // Assert
        cut.Markup.Should().Contain("Conflict Detected");
        cut.Markup.Should().Contain("Version mismatch");
    }

    [Fact]
    public void VersionConflictDialog_RendersThreeButtons()
    {
        // Act
        var cut = Render<VersionConflictDialog>(parameters => parameters
            .Add(p => p.ItemTitle, "Test"));

        // Assert
        var buttons = cut.FindComponents<RadzenButton>();
        buttons.Should().HaveCount(3);
    }

    [Fact]
    public void VersionConflictDialog_RendersReloadButton()
    {
        // Act
        var cut = Render<VersionConflictDialog>(parameters => parameters
            .Add(p => p.ItemTitle, "Test"));

        // Assert
        cut.Markup.Should().Contain("Reload");
        cut.Markup.Should().Contain("discard my changes");
    }

    [Fact]
    public void VersionConflictDialog_RendersOverwriteButton()
    {
        // Act
        var cut = Render<VersionConflictDialog>(parameters => parameters
            .Add(p => p.ItemTitle, "Test"));

        // Assert
        cut.Markup.Should().Contain("Overwrite");
        cut.Markup.Should().Contain("keep my changes");
    }

    [Fact]
    public void VersionConflictDialog_RendersCancelButton()
    {
        // Act
        var cut = Render<VersionConflictDialog>(parameters => parameters
            .Add(p => p.ItemTitle, "Test"));

        // Assert
        var cancelButton = cut.FindComponents<RadzenButton>()
            .FirstOrDefault(b => b.Instance.Text == "Cancel");
        cancelButton.Should().NotBeNull();
    }

    [Fact]
    public void VersionConflictDialog_RendersOptionsExplanation()
    {
        // Act
        var cut = Render<VersionConflictDialog>(parameters => parameters
            .Add(p => p.ItemTitle, "Test"));

        // Assert
        cut.Markup.Should().Contain("Choose how to resolve this conflict");
        cut.FindComponents<RadzenAlert>().Should().HaveCount(1);
    }

    [Fact]
    public void VersionConflictDialog_RendersWarningIcon()
    {
        // Act
        var cut = Render<VersionConflictDialog>(parameters => parameters
            .Add(p => p.ItemTitle, "Test"));

        // Assert
        cut.Markup.Should().Contain("warning");
    }

    #endregion

    #region WorkItemDialog Tests

    [Fact]
    public void WorkItemDialog_RendersTypeDropdown()
    {
        // Arrange
        SetupWorkItemDialogServices();

        // Act
        var cut = Render<WorkItemDialog>(parameters => parameters
            .Add(p => p.ProjectId, Guid.NewGuid()));

        // Assert
        cut.Markup.Should().Contain("Type");
        var dropdowns = cut.FindComponents<RadzenDropDown<WorkItemType>>();
        dropdowns.Should().HaveCount(1);
    }

    [Fact]
    public void WorkItemDialog_RendersTitle()
    {
        // Arrange
        SetupWorkItemDialogServices();

        // Act
        var cut = Render<WorkItemDialog>(parameters => parameters
            .Add(p => p.ProjectId, Guid.NewGuid()));

        // Assert
        cut.Markup.Should().Contain("Title");
        cut.FindComponents<RadzenTextBox>().Should().HaveCountGreaterThanOrEqualTo(1);
    }

    [Fact]
    public void WorkItemDialog_RendersDescription()
    {
        // Arrange
        SetupWorkItemDialogServices();

        // Act
        var cut = Render<WorkItemDialog>(parameters => parameters
            .Add(p => p.ProjectId, Guid.NewGuid()));

        // Assert
        cut.Markup.Should().Contain("Description");
        cut.FindComponents<RadzenTextArea>().Should().HaveCount(1);
    }

    [Fact]
    public void WorkItemDialog_RendersStatusDropdown()
    {
        // Arrange
        SetupWorkItemDialogServices();

        // Act
        var cut = Render<WorkItemDialog>(parameters => parameters
            .Add(p => p.ProjectId, Guid.NewGuid()));

        // Assert
        cut.Markup.Should().Contain("Status");
    }

    [Fact]
    public void WorkItemDialog_RendersPriorityDropdown()
    {
        // Arrange
        SetupWorkItemDialogServices();

        // Act
        var cut = Render<WorkItemDialog>(parameters => parameters
            .Add(p => p.ProjectId, Guid.NewGuid()));

        // Assert
        cut.Markup.Should().Contain("Priority");
    }

    [Fact]
    public void WorkItemDialog_RendersStoryPointsInput()
    {
        // Arrange
        SetupWorkItemDialogServices();

        // Act
        var cut = Render<WorkItemDialog>(parameters => parameters
            .Add(p => p.ProjectId, Guid.NewGuid()));

        // Assert
        cut.Markup.Should().Contain("Story Points");
        cut.FindComponents<RadzenNumeric<int?>>().Should().HaveCount(1);
    }

    [Fact]
    public void WorkItemDialog_RendersSprintDropdown()
    {
        // Arrange
        SetupWorkItemDialogServices();

        // Act
        var cut = Render<WorkItemDialog>(parameters => parameters
            .Add(p => p.ProjectId, Guid.NewGuid()));

        // Assert
        cut.Markup.Should().Contain("Sprint");
    }

    [Fact]
    public void WorkItemDialog_RendersCreateButton_ForNewItem()
    {
        // Arrange
        SetupWorkItemDialogServices();

        // Act
        var cut = Render<WorkItemDialog>(parameters => parameters
            .Add(p => p.ProjectId, Guid.NewGuid()));

        // Assert
        cut.Markup.Should().Contain("Create");
    }

    [Fact]
    public void WorkItemDialog_RendersSaveButton_ForEditItem()
    {
        // Arrange
        SetupWorkItemDialogServices();
        var viewModel = CreateTestViewModel();

        // Act
        var cut = Render<WorkItemDialog>(parameters => parameters
            .Add(p => p.WorkItem, viewModel)
            .Add(p => p.ProjectId, viewModel.ProjectId));

        // Assert
        cut.Markup.Should().Contain("Save Changes");
    }

    [Fact]
    public void WorkItemDialog_RendersCancelButton()
    {
        // Arrange
        SetupWorkItemDialogServices();

        // Act
        var cut = Render<WorkItemDialog>(parameters => parameters
            .Add(p => p.ProjectId, Guid.NewGuid()));

        // Assert
        var cancelButton = cut.FindComponents<RadzenButton>()
            .FirstOrDefault(b => b.Instance.Text == "Cancel");
        cancelButton.Should().NotBeNull();
    }

    [Fact]
    public void WorkItemDialog_DisablesTypeDropdown_ForEditItem()
    {
        // Arrange
        SetupWorkItemDialogServices();
        var viewModel = CreateTestViewModel();

        // Act
        var cut = Render<WorkItemDialog>(parameters => parameters
            .Add(p => p.WorkItem, viewModel)
            .Add(p => p.ProjectId, viewModel.ProjectId));

        // Assert
        var typeDropdown = cut.FindComponents<RadzenDropDown<WorkItemType>>().First();
        typeDropdown.Instance.Disabled.Should().BeTrue();
    }

    [Fact]
    public void WorkItemDialog_ShowsCharacterCount_ForTitle()
    {
        // Arrange
        SetupWorkItemDialogServices();

        // Act
        var cut = Render<WorkItemDialog>(parameters => parameters
            .Add(p => p.ProjectId, Guid.NewGuid()));

        // Assert
        cut.Markup.Should().Contain("/200");
    }

    [Fact]
    public void WorkItemDialog_ShowsCharacterCount_ForDescription()
    {
        // Arrange
        SetupWorkItemDialogServices();

        // Act
        var cut = Render<WorkItemDialog>(parameters => parameters
            .Add(p => p.ProjectId, Guid.NewGuid()));

        // Assert
        cut.Markup.Should().Contain("/5000");
    }

    [Fact]
    public void WorkItemDialog_PopulatesFields_ForEditItem()
    {
        // Arrange
        SetupWorkItemDialogServices();
        var workItem = CreateTestWorkItem() with
        {
            Title = "Existing Title",
            Description = "Existing Description"
        };
        var viewModel = new WorkItemViewModel(workItem);

        // Act
        var cut = Render<WorkItemDialog>(parameters => parameters
            .Add(p => p.WorkItem, viewModel)
            .Add(p => p.ProjectId, viewModel.ProjectId));

        // Assert
        cut.Markup.Should().Contain("Existing Title");
    }

    #endregion

    #region Helper Methods

    private void SetupWorkItemDialogServices()
    {
        var workItemStore = new Mock<IWorkItemStore>();
        var sprintStore = new Mock<ISprintStore>();
        var projectStore = new Mock<IProjectStore>();
        sprintStore.Setup(s => s.GetByProject(It.IsAny<Guid>()))
            .Returns(Array.Empty<Sprint>());
        workItemStore.Setup(w => w.GetByProject(It.IsAny<Guid>()))
            .Returns(Array.Empty<WorkItem>());

        var appState = new AppState(
            Mock.Of<IWebSocketClient>(),
            workItemStore.Object,
            sprintStore.Object,
            projectStore.Object,
            Mock.Of<ICommentStore>(),
            Mock.Of<Microsoft.Extensions.Logging.ILogger<AppState>>());
        
        Services.AddSingleton(appState);
        Services.AddSingleton(workItemStore.Object);
        Services.AddSingleton(sprintStore.Object);
        Services.AddSingleton<IProjectStore>(projectStore.Object);
        Services.AddScoped<ViewModelFactory>();
    }

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
