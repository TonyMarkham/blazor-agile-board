using Bunit;
using FluentAssertions;
using Microsoft.AspNetCore.Components;
using Microsoft.Extensions.DependencyInjection;
using Moq;
using ProjectManagement.Components.Shared;
using ProjectManagement.Components.WorkItems;
using ProjectManagement.Core.Interfaces;
using ProjectManagement.Core.Models;
using ProjectManagement.Core.ViewModels;
using ProjectManagement.Services.State;
using Radzen;
using Radzen.Blazor;

namespace ProjectManagement.Components.Tests.WorkItems;

public class WorkItemListTests : BunitContext
{
    private readonly Mock<IWorkItemStore> _workItemStoreMock;
    private readonly Mock<ISprintStore> _sprintStoreMock;
    private readonly AppState _appState;
    private readonly Guid _projectId = Guid.NewGuid();

    public WorkItemListTests()
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
            Mock.Of<ICommentStore>(),
            Mock.Of<Microsoft.Extensions.Logging.ILogger<AppState>>());

        Services.AddSingleton(_appState);
        Services.AddSingleton(_workItemStoreMock.Object);
        Services.AddSingleton(_sprintStoreMock.Object);
        Services.AddSingleton<IProjectStore>(projectStoreMock.Object);
        Services.AddScoped<ViewModelFactory>();

        JSInterop.Mode = JSRuntimeMode.Loose;
    }

    #region Empty State Tests

    [Fact]
    public async Task WorkItemList_ShowsEmptyState_WhenNoItems()
    {
        // Arrange
        SetupEmptyStore();

        // Act
        var cut = Render<WorkItemList>(parameters => parameters
            .Add(p => p.ProjectId, _projectId));

        // Wait for loading to complete by checking for actual content
        await cut.WaitForAssertionAsync(() => 
            cut.Markup.Should().Contain("No work items yet"), 
            timeout: TimeSpan.FromSeconds(5));

        // Assert
        cut.Markup.Should().Contain("Create your first work item");
    }

    [Fact]
    public async Task WorkItemList_ShowsCreateButton_WhenNoItems()
    {
        // Arrange
        SetupEmptyStore();

        // Act
        var cut = Render<WorkItemList>(parameters => parameters
            .Add(p => p.ProjectId, _projectId));

        // Wait for content to appear
        await cut.WaitForAssertionAsync(() => 
            cut.Markup.Should().Contain("Create Work Item"),
            timeout: TimeSpan.FromSeconds(5));
    }

    #endregion

    #region Work Item Rendering Tests

    [Fact]
    public async Task WorkItemList_RendersWorkItems_WhenItemsExist()
    {
        // Arrange
        var items = new[]
        {
            CreateWorkItem("Epic 1", WorkItemType.Epic),
            CreateWorkItem("Story 1", WorkItemType.Story),
            CreateWorkItem("Task 1", WorkItemType.Task)
        };
        SetupStoreWithItems(items);

        // Act
        var cut = Render<WorkItemList>(parameters => parameters
            .Add(p => p.ProjectId, _projectId));

        // Wait for work items to render
        await cut.WaitForAssertionAsync(() => 
            cut.FindComponents<WorkItemRow>().Should().HaveCount(3),
            timeout: TimeSpan.FromSeconds(5));

        // Assert
        cut.Markup.Should().Contain("Epic 1");
        cut.Markup.Should().Contain("Story 1");
        cut.Markup.Should().Contain("Task 1");
    }

    [Fact]
    public async Task WorkItemList_ShowsCorrectCount_InAnnouncement()
    {
        // Arrange
        var items = new[]
        {
            CreateWorkItem("Item 1", WorkItemType.Story),
            CreateWorkItem("Item 2", WorkItemType.Task)
        };
        SetupStoreWithItems(items);

        // Act
        var cut = Render<WorkItemList>(parameters => parameters
            .Add(p => p.ProjectId, _projectId));

        // Wait for count announcement
        await cut.WaitForAssertionAsync(() => 
            cut.Markup.Should().Contain("2 work items found"),
            timeout: TimeSpan.FromSeconds(5));
    }

    #endregion

    #region Search Filter Tests

    [Fact]
    public async Task WorkItemList_FiltersBySearch_WhenTextEntered()
    {
        // Arrange
        var items = new[]
        {
            CreateWorkItem("Login Feature", WorkItemType.Story),
            CreateWorkItem("Registration Feature", WorkItemType.Story),
            CreateWorkItem("Dashboard Task", WorkItemType.Task)
        };
        SetupStoreWithItems(items);

        var cut = Render<WorkItemList>(parameters => parameters
            .Add(p => p.ProjectId, _projectId));

        await cut.WaitForAssertionAsync(() => 
            cut.FindComponents<WorkItemRow>().Should().HaveCount(3),
            timeout: TimeSpan.FromSeconds(5));

        // Act - Trigger search via the debounced textbox
        var searchBox = cut.FindComponent<DebouncedTextBox>();
        await cut.InvokeAsync(() => searchBox.Instance.ValueChanged.InvokeAsync("Feature"));

        // Assert
        await cut.WaitForAssertionAsync(() => 
        {
            cut.Markup.Should().Contain("2 work items found");
            cut.Markup.Should().Contain("Login Feature");
            cut.Markup.Should().Contain("Registration Feature");
            cut.Markup.Should().NotContain("Dashboard Task");
        }, timeout: TimeSpan.FromSeconds(2));
    }

    [Fact]
    public async Task WorkItemList_SearchIsCaseInsensitive()
    {
        // Arrange
        var items = new[]
        {
            CreateWorkItem("Login Feature", WorkItemType.Story)
        };
        SetupStoreWithItems(items);

        var cut = Render<WorkItemList>(parameters => parameters
            .Add(p => p.ProjectId, _projectId));

        await cut.WaitForAssertionAsync(() => 
            cut.Markup.Should().Contain("Login Feature"),
            timeout: TimeSpan.FromSeconds(5));

        // Act - Search with different case
        var searchBox = cut.FindComponent<DebouncedTextBox>();
        await cut.InvokeAsync(() => searchBox.Instance.ValueChanged.InvokeAsync("LOGIN"));

        // Assert
        await cut.WaitForAssertionAsync(() => 
            cut.Markup.Should().Contain("Login Feature"),
            timeout: TimeSpan.FromSeconds(2));
    }

    #endregion

    #region Type Filter Tests

    [Fact]
    public async Task WorkItemList_FiltersByType_WhenTypeSelected()
    {
        // Arrange
        var items = new[]
        {
            CreateWorkItem("Epic 1", WorkItemType.Epic),
            CreateWorkItem("Story 1", WorkItemType.Story),
            CreateWorkItem("Task 1", WorkItemType.Task)
        };
        SetupStoreWithItems(items);

        var cut = Render<WorkItemList>(parameters => parameters
            .Add(p => p.ProjectId, _projectId));

        await cut.WaitForAssertionAsync(() => 
                cut.FindComponents<WorkItemRow>().Should().HaveCount(3),
            timeout: TimeSpan.FromSeconds(5));

        // Act - Simulate type filter change via the bound value AND trigger Change event
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
            cut.Markup.Should().Contain("1 work items found");
        }, timeout: TimeSpan.FromSeconds(2));
    }

    #endregion

    #region Status Filter Tests

    [Fact]
    public async Task WorkItemList_FiltersByStatus_WhenStatusSelected()
    {
        // Arrange
        var items = new[]
        {
            CreateWorkItem("Backlog Item", WorkItemType.Story, status: "backlog"),
            CreateWorkItem("In Progress Item", WorkItemType.Story, status: "in_progress"),
            CreateWorkItem("Done Item", WorkItemType.Story, status: "done")
        };
        SetupStoreWithItems(items);

        var cut = Render<WorkItemList>(parameters => parameters
            .Add(p => p.ProjectId, _projectId));

        await cut.WaitForAssertionAsync(() => 
                cut.FindComponents<WorkItemRow>().Should().HaveCount(3),
            timeout: TimeSpan.FromSeconds(5));

        // Act - Simulate status filter change via the bound value AND trigger Change event
        var statusDropdown = cut.FindComponents<RadzenDropDown<string>>()
            .First(d => d.Instance.Placeholder == "All Statuses");
        
        await cut.InvokeAsync(async () =>
        {
            await statusDropdown.Instance.ValueChanged.InvokeAsync("in_progress").ConfigureAwait(false);
            await statusDropdown.Instance.Change.InvokeAsync("in_progress").ConfigureAwait(false);
        });

        // Assert
        await cut.WaitForAssertionAsync(() =>
        {
            cut.Markup.Should().Contain("In Progress Item");
            cut.Markup.Should().NotContain("Backlog Item");
            cut.Markup.Should().NotContain("Done Item");
        }, timeout: TimeSpan.FromSeconds(2));
    }

    #endregion

    #region No Matches Tests

    [Fact]
    public async Task WorkItemList_ShowsNoMatches_WhenFiltersExcludeAll()
    {
        // Arrange
        var items = new[]
        {
            CreateWorkItem("Story 1", WorkItemType.Story),
            CreateWorkItem("Task 1", WorkItemType.Task)
        };
        SetupStoreWithItems(items);

        var cut = Render<WorkItemList>(parameters => parameters
            .Add(p => p.ProjectId, _projectId));

        await cut.WaitForAssertionAsync(() => 
            cut.FindComponents<WorkItemRow>().Should().HaveCount(2),
            timeout: TimeSpan.FromSeconds(5));

        // Act - Search for non-existent term
        var searchBox = cut.FindComponent<DebouncedTextBox>();
        await cut.InvokeAsync(() => searchBox.Instance.ValueChanged.InvokeAsync("NonExistentTerm"));

        // Assert
        await cut.WaitForAssertionAsync(() =>
        {
            cut.Markup.Should().Contain("No matches found");
            cut.Markup.Should().Contain("Try adjusting your search or filters");
            cut.Markup.Should().NotContain("Create Work Item"); // No create button when filtering
        }, timeout: TimeSpan.FromSeconds(2));
    }

    #endregion

    #region Interaction Tests

    [Fact]
    public async Task WorkItemList_CallsOnWorkItemSelected_WhenRowClicked()
    {
        // Arrange
        var items = new[] { CreateWorkItem("Test Item", WorkItemType.Story) };
        SetupStoreWithItems(items);

        WorkItemViewModel? selectedItem = null;

        var cut = Render<WorkItemList>(parameters => parameters
            .Add(p => p.ProjectId, _projectId)
            .Add(p => p.OnWorkItemSelected, EventCallback.Factory.Create<WorkItemViewModel>(
                this, vm => selectedItem = vm)));

        await cut.WaitForAssertionAsync(() => 
            cut.FindComponents<WorkItemRow>().Should().HaveCount(1),
            timeout: TimeSpan.FromSeconds(5));

        // Act - Click the work item row
        var row = cut.FindComponent<WorkItemRow>();
        await row.Find(".work-item-row").ClickAsync();

        // Assert
        selectedItem.Should().NotBeNull();
        selectedItem!.Title.Should().Be("Test Item");
    }

    [Fact]
    public async Task WorkItemList_UpdatesWhenStoreChanges()
    {
        // Arrange
        SetupEmptyStore();

        var cut = Render<WorkItemList>(parameters => parameters
            .Add(p => p.ProjectId, _projectId));

        await cut.WaitForAssertionAsync(() => 
            cut.Markup.Should().Contain("No work items yet"),
            timeout: TimeSpan.FromSeconds(5));

        // Act - Add items to store and trigger state change
        var newItem = CreateWorkItem("New Item", WorkItemType.Story);
        _workItemStoreMock.Setup(s => s.GetByProject(_projectId))
            .Returns(new[] { newItem });

        // Trigger the OnChanged event on the mock to notify the component
        _workItemStoreMock.Raise(m => m.OnChanged += null);

        // Assert - Component should refresh and show the new item
        await cut.WaitForAssertionAsync(() =>
        {
            cut.Markup.Should().Contain("New Item");
            cut.Markup.Should().NotContain("No work items yet");
        }, timeout: TimeSpan.FromSeconds(5));
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

    private WorkItem CreateWorkItem(string title, WorkItemType type, string status = "backlog")
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
