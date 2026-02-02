using Bunit;
using FluentAssertions;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Logging.Abstractions;
using Moq;
using ProjectManagement.Components.Dependencies;
using ProjectManagement.Core.Interfaces;
using ProjectManagement.Core.Models;
using ProjectManagement.Core.ViewModels;
using ProjectManagement.Services.State;
using Radzen;

namespace ProjectManagement.Components.Tests.Dependencies;

public class DependencyManagerTests : BunitContext
{
    private readonly Mock<IDependencyStore> _mockStore = new();
    private readonly Mock<IWorkItemStore> _mockWorkItemStore = new();
    private readonly Mock<ISprintStore> _mockSprintStore = new();
    private readonly Mock<IProjectStore> _mockProjectStore = new();
    private readonly Mock<ICommentStore> _mockCommentStore = new();
    private readonly Mock<IWebSocketClient> _mockClient = new();
    private readonly AppState _appState;

    public DependencyManagerTests()
    {
        // Register Radzen services (let DI create them)
        Services.AddScoped<DialogService>();
        Services.AddScoped<NotificationService>();
        Services.AddScoped<TooltipService>();
        Services.AddScoped<ContextMenuService>();
        Services.AddSingleton<Microsoft.Extensions.Logging.ILogger<DependencyManager>>(
            NullLogger<DependencyManager>.Instance);

        JSInterop.Mode = JSRuntimeMode.Loose;

        // Default setup
        _mockClient.Setup(c => c.State).Returns(ConnectionState.Connected);
        _mockStore.Setup(s => s.GetBlocking(It.IsAny<Guid>())).Returns(Array.Empty<Dependency>());
        _mockStore.Setup(s => s.GetBlocked(It.IsAny<Guid>())).Returns(Array.Empty<Dependency>());

        // Create AppState
        _appState = new AppState(
            _mockClient.Object,
            _mockWorkItemStore.Object,
            _mockSprintStore.Object,
            _mockProjectStore.Object,
            _mockCommentStore.Object,
            NullLogger<AppState>.Instance);

        Services.AddSingleton(_appState);
        Services.AddSingleton<IDependencyStore>(_mockStore.Object);
        Services.AddSingleton<IWorkItemStore>(_mockWorkItemStore.Object);
        Services.AddSingleton<ISprintStore>(_mockSprintStore.Object);
        Services.AddSingleton<IProjectStore>(_mockProjectStore.Object);
        Services.AddScoped<ViewModelFactory>();
    }

    [Fact]
    public void DependencyManager_ThrowsOnEmptyWorkItemId()
    {
        // Arrange
        var projectId = Guid.NewGuid();

        // Act & Assert
        var act = () => Render<DependencyManager>(parameters => parameters
            .Add(p => p.WorkItemId, Guid.Empty)
            .Add(p => p.ProjectId, projectId));

        act.Should().Throw<ArgumentException>()
            .WithMessage("*WorkItemId cannot be empty*");
    }

    [Fact]
    public void DependencyManager_ThrowsOnEmptyProjectId()
    {
        // Arrange
        var workItemId = Guid.NewGuid();

        // Act & Assert
        var act = () => Render<DependencyManager>(parameters => parameters
            .Add(p => p.WorkItemId, workItemId)
            .Add(p => p.ProjectId, Guid.Empty));

        act.Should().Throw<ArgumentException>()
            .WithMessage("*ProjectId cannot be empty*");
    }

    [Fact]
    public void DependencyManager_RendersBlockingSection()
    {
        // Arrange
        var workItemId = Guid.NewGuid();
        var projectId = Guid.NewGuid();
        _mockStore.Setup(s => s.GetBlocking(workItemId)).Returns(Array.Empty<Dependency>());
        _mockStore.Setup(s => s.GetBlocked(workItemId)).Returns(Array.Empty<Dependency>());

        // Act
        var cut = Render<DependencyManager>(parameters => parameters
            .Add(p => p.WorkItemId, workItemId)
            .Add(p => p.ProjectId, projectId));

        // Assert
        cut.Markup.Should().Contain("Blocking this item");
        cut.Markup.Should().Contain("Blocked by this item");
    }

    [Fact]
    public void DependencyManager_ShowsEmptyState_WhenNoDependencies()
    {
        // Arrange
        var workItemId = Guid.NewGuid();
        var projectId = Guid.NewGuid();
        _mockStore.Setup(s => s.GetBlocking(workItemId)).Returns(Array.Empty<Dependency>());
        _mockStore.Setup(s => s.GetBlocked(workItemId)).Returns(Array.Empty<Dependency>());

        // Act
        var cut = Render<DependencyManager>(parameters => parameters
            .Add(p => p.WorkItemId, workItemId)
            .Add(p => p.ProjectId, projectId));

        // Assert
        cut.Markup.Should().Contain("No items are blocking this work item");
        cut.Markup.Should().Contain("This work item is not blocking anything");
    }

    [Fact]
    public void DependencyManager_DisablesAddButton_WhenOffline()
    {
        // Arrange
        var workItemId = Guid.NewGuid();
        var projectId = Guid.NewGuid();
        _mockStore.Setup(s => s.GetBlocking(workItemId)).Returns(Array.Empty<Dependency>());
        _mockStore.Setup(s => s.GetBlocked(workItemId)).Returns(Array.Empty<Dependency>());
        _mockClient.Setup(c => c.State).Returns(ConnectionState.Disconnected);

        // Act
        var cut = Render<DependencyManager>(parameters => parameters
            .Add(p => p.WorkItemId, workItemId)
            .Add(p => p.ProjectId, projectId));

        // Assert
        var addButton = cut.Find("button[aria-label='Add dependency']");
        addButton.HasAttribute("disabled").Should().BeTrue();
        cut.Markup.Should().Contain("You are offline");
    }

    [Fact]
    public async Task DependencyManager_CallsRefreshAsync_OnLoad()
    {
        // Arrange
        var workItemId = Guid.NewGuid();
        var projectId = Guid.NewGuid();
        _mockStore.Setup(s => s.GetBlocking(workItemId)).Returns(Array.Empty<Dependency>());
        _mockStore.Setup(s => s.GetBlocked(workItemId)).Returns(Array.Empty<Dependency>());
        _mockStore.Setup(s => s.RefreshAsync(workItemId, It.IsAny<CancellationToken>()))
            .Returns(Task.CompletedTask);

        // Act
        var cut = Render<DependencyManager>(parameters => parameters
            .Add(p => p.WorkItemId, workItemId)
            .Add(p => p.ProjectId, projectId));

        await Task.Delay(100); // Let async initialization complete

        // Assert
        _mockStore.Verify(s => s.RefreshAsync(workItemId, It.IsAny<CancellationToken>()), Times.Once);
    }

    [Fact]
    public async Task DependencyManager_ShowsErrorState_WhenRefreshFails()
    {
        // Arrange
        var workItemId = Guid.NewGuid();
        var projectId = Guid.NewGuid();
        _mockStore.Setup(s => s.GetBlocking(workItemId)).Returns(Array.Empty<Dependency>());
        _mockStore.Setup(s => s.GetBlocked(workItemId)).Returns(Array.Empty<Dependency>());
        _mockStore.Setup(s => s.RefreshAsync(workItemId, It.IsAny<CancellationToken>()))
            .ThrowsAsync(new Exception("Network error"));

        // Act
        var cut = Render<DependencyManager>(parameters => parameters
            .Add(p => p.WorkItemId, workItemId)
            .Add(p => p.ProjectId, projectId));

        await Task.Delay(100);

        // Assert
        cut.Markup.Should().Contain("Failed to load dependencies");
        cut.Markup.Should().Contain("Network error");
        cut.Markup.Should().Contain("Retry");
    }

    [Fact]
    public async Task DependencyManager_HandleRemove_CallsDeleteAsync()
    {
        // Arrange
        var workItemId = Guid.NewGuid();
        var projectId = Guid.NewGuid();
        var dependencyId = Guid.NewGuid();

        _mockStore.Setup(s => s.GetBlocking(workItemId)).Returns(new List<Dependency>
        {
            new Dependency { Id = dependencyId, BlockingItemId = Guid.NewGuid(), BlockedItemId = workItemId }
        });
        _mockStore.Setup(s => s.GetBlocked(workItemId)).Returns(Array.Empty<Dependency>());
        _mockStore.Setup(s => s.DeleteAsync(dependencyId, It.IsAny<CancellationToken>()))
            .Returns(Task.CompletedTask);

        var cut = Render<DependencyManager>(parameters => parameters
            .Add(p => p.WorkItemId, workItemId)
            .Add(p => p.ProjectId, projectId));

        await Task.Delay(100);

        // Act - Trigger remove via reflection
        var instance = cut.Instance;
        var method = instance.GetType().GetMethod("HandleRemove",
            System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Instance);
        await (Task)method!.Invoke(instance, new object[] { dependencyId })!;

        await Task.Delay(50);

        // Assert
        _mockStore.Verify(s => s.DeleteAsync(dependencyId, It.IsAny<CancellationToken>()), Times.Once);
    }

    [Fact]
    public void DependencyManager_ResolveWorkItem_ReturnsNull_WhenNotFound()
    {
        // Arrange
        var workItemId = Guid.NewGuid();
        var projectId = Guid.NewGuid();
        var missingId = Guid.NewGuid();

        _mockStore.Setup(s => s.GetBlocking(workItemId)).Returns(Array.Empty<Dependency>());
        _mockStore.Setup(s => s.GetBlocked(workItemId)).Returns(Array.Empty<Dependency>());
        _mockWorkItemStore.Setup(s => s.GetById(missingId)).Returns((WorkItem?)null);

        var cut = Render<DependencyManager>(parameters => parameters
            .Add(p => p.WorkItemId, workItemId)
            .Add(p => p.ProjectId, projectId));

        // Act - Call ResolveWorkItem via reflection
        var instance = cut.Instance;
        var method = instance.GetType().GetMethod("ResolveWorkItem",
            System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Instance);
        var result = method!.Invoke(instance, new object[] { missingId });

        // Assert
        result.Should().BeNull();
    }
}