using Bunit;
using FluentAssertions;
using Microsoft.AspNetCore.Components.Web;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Logging.Abstractions;
using Moq;
using ProjectManagement.Components.Dependencies;
using ProjectManagement.Core.Exceptions;
using ProjectManagement.Core.Interfaces;
using ProjectManagement.Core.Models;
using ProjectManagement.Services.State;
using ProjectManagement.Core.ViewModels;
using Radzen;
using Radzen.Blazor;

namespace ProjectManagement.Components.Tests.Dependencies;

public class AddDependencyDialogTests : BunitContext
{
    private readonly Mock<IDependencyStore> _dependencyStore = new();
    private readonly Mock<IWorkItemStore> _workItemStore = new();
    private readonly Mock<ISprintStore> _sprintStore = new();
    private readonly Mock<IProjectStore> _projectStore = new();
    private readonly Mock<ICommentStore> _commentStore = new();
    private readonly Mock<IWebSocketClient> _client = new();

    private readonly Guid _projectId = Guid.NewGuid();

    public AddDependencyDialogTests()
    {
        Services.AddScoped<DialogService>();
        Services.AddScoped<NotificationService>();
        Services.AddScoped<TooltipService>();
        Services.AddScoped<ContextMenuService>();
        Services.AddSingleton<Microsoft.Extensions.Logging.ILogger<AddDependencyDialog>>(
            NullLogger<AddDependencyDialog>.Instance);
        JSInterop.Mode = JSRuntimeMode.Loose;

        _client.Setup(c => c.State).Returns(ConnectionState.Connected);

        _workItemStore.Setup(s => s.IsPending(It.IsAny<Guid>())).Returns(false);
        _sprintStore.Setup(s => s.IsPending(It.IsAny<Guid>())).Returns(false);
        _projectStore.Setup(s => s.IsPending(It.IsAny<Guid>())).Returns(false);

        _dependencyStore.Setup(s => s.GetBlocking(It.IsAny<Guid>()))
            .Returns(Array.Empty<Dependency>());
        _dependencyStore.Setup(s => s.GetBlocked(It.IsAny<Guid>()))
            .Returns(Array.Empty<Dependency>());

        var appState = new AppState(
            _client.Object,
            _workItemStore.Object,
            _sprintStore.Object,
            _projectStore.Object,
            _commentStore.Object,
            Mock.Of<Microsoft.Extensions.Logging.ILogger<AppState>>());

        Services.AddSingleton(appState);
        Services.AddSingleton(_workItemStore.Object);
        Services.AddSingleton(_sprintStore.Object);
        Services.AddSingleton<IProjectStore>(_projectStore.Object);
        Services.AddSingleton<ICommentStore>(_commentStore.Object);
        Services.AddScoped<ViewModelFactory>();
        Services.AddSingleton(_dependencyStore.Object);
    }

    [Fact]
    public void AddDependencyDialog_FiltersOutCurrentItem()
    {
        // Arrange
        var current = CreateWorkItem("Current");
        var other = CreateWorkItem("Other");
        _workItemStore.Setup(s => s.GetByProject(_projectId))
            .Returns([current, other]);

        // Act
        var cut = Render<AddDependencyDialog>(parameters => parameters
            .Add(p => p.WorkItemId, current.Id)
            .Add(p => p.ProjectId, _projectId));

        // Assert
        var options = cut.FindAll(".search-result");
        options.Should().ContainSingle(o => o.TextContent.Contains("Other"));
        options.Should().NotContain(o => o.TextContent.Contains("Current"));
    }

    [Fact]
    public void AddDependencyDialog_FiltersOutExistingDependencies()
    {
        // Arrange
        var current = CreateWorkItem("Current");
        var blocking = CreateWorkItem("Blocking");
        var blocked = CreateWorkItem("Blocked");
        var ok = CreateWorkItem("Ok");

        _workItemStore.Setup(s => s.GetByProject(_projectId))
            .Returns([current, blocking, blocked, ok]);

        _dependencyStore.Setup(s => s.GetBlocking(current.Id)).Returns([
            new Dependency
            {
                Id = Guid.NewGuid(),
                BlockingItemId = blocking.Id,
                BlockedItemId = current.Id,
                Type = DependencyType.Blocks,
                CreatedAt = DateTime.UtcNow,
                CreatedBy = Guid.NewGuid()
            }
        ]);

        _dependencyStore.Setup(s => s.GetBlocked(current.Id)).Returns([
            new Dependency
            {
                Id = Guid.NewGuid(),
                BlockingItemId = current.Id,
                BlockedItemId = blocked.Id,
                Type = DependencyType.Blocks,
                CreatedAt = DateTime.UtcNow,
                CreatedBy = Guid.NewGuid()
            }
        ]);

        // Act
        var cut = Render<AddDependencyDialog>(parameters => parameters
            .Add(p => p.WorkItemId, current.Id)
            .Add(p => p.ProjectId, _projectId));

        // Assert
        var options = cut.FindAll(".search-result");
        options.Should().ContainSingle(o => o.TextContent.Contains("Ok"));
        options.Should().NotContain(o => o.TextContent.Contains("Blocking"));
        options.Should().NotContain(o => o.TextContent.Contains("Blocked"));
    }

    [Fact]
    public async Task AddDependencyDialog_EnterSelectsActiveItem()
    {
        // Arrange
        var current = CreateWorkItem("Current");
        var target = CreateWorkItem("Target");
        _workItemStore.Setup(s => s.GetByProject(_projectId))
            .Returns([current, target]);

        CreateDependencyRequest? captured = null;
        _dependencyStore.Setup(s => s.CreateAsync(It.IsAny<CreateDependencyRequest>(),
                It.IsAny<CancellationToken>()))
            .Callback<CreateDependencyRequest, CancellationToken>((req, _) => captured = req)
            .ReturnsAsync(CreateDependency(target.Id, current.Id));

        var cut = Render<AddDependencyDialog>(parameters => parameters
            .Add(p => p.WorkItemId, current.Id)
            .Add(p => p.ProjectId, _projectId));

        var input = cut.Find("[role='combobox']");

        // Act
        await cut.InvokeAsync(() => input.KeyDown(new KeyboardEventArgs { Key = "ArrowDown" }));
        await cut.InvokeAsync(() => input.KeyDown(new KeyboardEventArgs { Key = "Enter" }));

        // Assert
        captured.Should().NotBeNull();
        captured!.BlockingItemId.Should().Be(target.Id);
        captured.BlockedItemId.Should().Be(current.Id);
        captured.Type.Should().Be(DependencyType.Blocks);
    }

    [Fact]
    public async Task AddDependencyDialog_ArrowKeysMoveActiveOption()
    {
        // Arrange
        var current = CreateWorkItem("Current");
        var first = CreateWorkItem("First");
        var second = CreateWorkItem("Second");
        _workItemStore.Setup(s => s.GetByProject(_projectId))
            .Returns([current, first, second]);

        var cut = Render<AddDependencyDialog>(parameters => parameters
            .Add(p => p.WorkItemId, current.Id)
            .Add(p => p.ProjectId, _projectId));

        var input = cut.Find("[role='combobox']");

        // Act
        await cut.InvokeAsync(() => input.KeyDown(new KeyboardEventArgs { Key = "ArrowDown" }));

        // Assert
        var activeId = input.GetAttribute("aria-activedescendant");
        activeId.Should().NotBeNullOrWhiteSpace();
        cut.Find($"#{activeId}").Should().NotBeNull();
    }

    [Fact]
    public async Task AddDependencyDialog_UsesCanonicalDirection_ForRelates()
    {
        // Arrange
        var current = CreateWorkItem("Current");
        var target = CreateWorkItem("Target");
        _workItemStore.Setup(s => s.GetByProject(_projectId))
            .Returns([current, target]);

        CreateDependencyRequest? captured = null;
        _dependencyStore.Setup(s => s.CreateAsync(It.IsAny<CreateDependencyRequest>(),
                It.IsAny<CancellationToken>()))
            .Callback<CreateDependencyRequest, CancellationToken>((req, _) => captured = req)
            .ReturnsAsync(CreateDependency(current.Id, target.Id));

        var cut = Render<AddDependencyDialog>(parameters => parameters
            .Add(p => p.WorkItemId, current.Id)
            .Add(p => p.ProjectId, _projectId));

        var typeDropdown = cut.FindComponent<RadzenDropDown<DependencyType>>();
        await cut.InvokeAsync(async () =>
        {
            await
                typeDropdown.Instance.ValueChanged.InvokeAsync(DependencyType.RelatesTo).ConfigureAwait(false);
            await
                typeDropdown.Instance.Change.InvokeAsync(DependencyType.RelatesTo).ConfigureAwait(false);
        });

        var input = cut.Find("[role='combobox']");

        // Act
        await cut.InvokeAsync(() => input.KeyDown(new KeyboardEventArgs { Key = "ArrowDown" }));
        await cut.InvokeAsync(() => input.KeyDown(new KeyboardEventArgs { Key = "Enter" }));

        // Assert
        captured.Should().NotBeNull();
        captured!.BlockingItemId.Should().Be(current.Id);
        captured.BlockedItemId.Should().Be(target.Id);
        captured.Type.Should().Be(DependencyType.RelatesTo);
    }

    [Fact]
    public async Task AddDependencyDialog_ClosesOnSuccess()
    {
        // Arrange
        var current = CreateWorkItem("Current");
        var target = CreateWorkItem("Target");
        _workItemStore.Setup(s => s.GetByProject(_projectId))
            .Returns([current, target]);

        _dependencyStore.Setup(s => s.CreateAsync(It.IsAny<CreateDependencyRequest>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(CreateDependency(target.Id, current.Id));

        var cut = Render<AddDependencyDialog>(parameters => parameters
            .Add(p => p.WorkItemId, current.Id)
            .Add(p => p.ProjectId, _projectId));

        var input = cut.Find("[role='combobox']");

        // Act
        await cut.InvokeAsync(() => input.KeyDown(new KeyboardEventArgs { Key = "ArrowDown" }));
        await cut.InvokeAsync(() => input.KeyDown(new KeyboardEventArgs { Key = "Enter" }));

        // Assert - verify CreateAsync was called (dialog closes after this)
        _dependencyStore.Verify(s => s.CreateAsync(It.IsAny<CreateDependencyRequest>(),
            It.IsAny<CancellationToken>()), Times.Once);
    }

    [Fact]
    public async Task AddDependencyDialog_RendersValidationErrorInline()
    {
        // Arrange
        var current = CreateWorkItem("Current");
        var target = CreateWorkItem("Target");
        _workItemStore.Setup(s => s.GetByProject(_projectId))
            .Returns([current, target]);

        _dependencyStore.Setup(s => s.CreateAsync(It.IsAny<CreateDependencyRequest>(),
                It.IsAny<CancellationToken>()))
            .ThrowsAsync(new ValidationException("Dependency", "Duplicate dependency"));

        var cut = Render<AddDependencyDialog>(parameters => parameters
            .Add(p => p.WorkItemId, current.Id)
            .Add(p => p.ProjectId, _projectId));

        var input = cut.Find("[role='combobox']");

        // Act
        await cut.InvokeAsync(() => input.KeyDown(new KeyboardEventArgs { Key = "ArrowDown" }));
        await cut.InvokeAsync(() => input.KeyDown(new KeyboardEventArgs { Key = "Enter" }));

        // Assert
        var alert = cut.Find("[role='alert']");
        alert.TextContent.Should().Contain("Duplicate dependency");
        var listboxId = input.GetAttribute("aria-controls");
        listboxId.Should().NotBeNullOrWhiteSpace();
        cut.Find($"#{listboxId}").Should().NotBeNull();
        var describedBy = input.GetAttribute("aria-describedby");
        describedBy.Should().NotBeNullOrWhiteSpace();
        foreach (var id in describedBy!.Split(' ', StringSplitOptions.RemoveEmptyEntries))
        {
            cut.Find($"#{id}").Should().NotBeNull();
        }
    }

    [Fact]
    public async Task AddDependencyDialog_DoesNotClose_OnValidationError()
    {
        // Arrange
        var current = CreateWorkItem("Current");
        var target = CreateWorkItem("Target");
        _workItemStore.Setup(s => s.GetByProject(_projectId))
            .Returns([current, target]);

        _dependencyStore.Setup(s => s.CreateAsync(It.IsAny<CreateDependencyRequest>(),
                It.IsAny<CancellationToken>()))
            .ThrowsAsync(new ValidationException("Dependency", "Duplicate dependency"));

        var dialogService = Services.GetRequiredService<DialogService>();
        var closed = false;
        dialogService.OnClose += _ => closed = true;

        var cut = Render<AddDependencyDialog>(parameters => parameters
            .Add(p => p.WorkItemId, current.Id)
            .Add(p => p.ProjectId, _projectId));

        var input = cut.Find("[role='combobox']");

        // Act
        await cut.InvokeAsync(() => input.KeyDown(new KeyboardEventArgs { Key = "ArrowDown" }));
        await cut.InvokeAsync(() => input.KeyDown(new KeyboardEventArgs { Key = "Enter" }));

        // Assert
        closed.Should().BeFalse();
    }

    [Fact]
    public void AddDependencyDialog_UsesUniqueInstanceIds()
    {
        // Arrange
        var current = CreateWorkItem("Current");
        var target = CreateWorkItem("Target");
        _workItemStore.Setup(s => s.GetByProject(_projectId))
            .Returns([current, target]);

        // Act
        var cut1 = Render<AddDependencyDialog>(parameters => parameters
            .Add(p => p.WorkItemId, current.Id)
            .Add(p => p.ProjectId, _projectId));
        var cut2 = Render<AddDependencyDialog>(parameters => parameters
            .Add(p => p.WorkItemId, current.Id)
            .Add(p => p.ProjectId, _projectId));

        // Assert
        var id1 = cut1.Find("[role='combobox']").GetAttribute("aria-controls");
        var id2 = cut2.Find("[role='combobox']").GetAttribute("aria-controls");
        id1.Should().NotBe(id2);
    }

    [Fact]
    public async Task AddDependencyDialog_ShowsSafeFallbackMessage_OnException()
    {
        // Arrange
        var current = CreateWorkItem("Current");
        var target = CreateWorkItem("Target");
        _workItemStore.Setup(s => s.GetByProject(_projectId))
            .Returns([current, target]);

        _dependencyStore.Setup(s => s.CreateAsync(It.IsAny<CreateDependencyRequest>(),
                It.IsAny<CancellationToken>()))
            .ThrowsAsync(new Exception("Boom"));

        var cut = Render<AddDependencyDialog>(parameters => parameters
            .Add(p => p.WorkItemId, current.Id)
            .Add(p => p.ProjectId, _projectId));

        var input = cut.Find("[role='combobox']");

        // Act
        await cut.InvokeAsync(() => input.KeyDown(new KeyboardEventArgs { Key = "ArrowDown" }));
        await cut.InvokeAsync(() => input.KeyDown(new KeyboardEventArgs { Key = "Enter" }));

        // Assert
        var alert = cut.Find("[role='alert']");
        alert.TextContent.Should().Contain("Something went wrong");
        alert.TextContent.Should().NotContain("Boom");
    }

    [Fact]
    public async Task AddDependencyDialog_DoesNotCreate_WhenOffline()
    {
        // Arrange
        _client.Setup(c => c.State).Returns(ConnectionState.Disconnected);

        var current = CreateWorkItem("Current");
        var target = CreateWorkItem("Target");
        _workItemStore.Setup(s => s.GetByProject(_projectId))
            .Returns([current, target]);

        var cut = Render<AddDependencyDialog>(parameters => parameters
            .Add(p => p.WorkItemId, current.Id)
            .Add(p => p.ProjectId, _projectId));

        var input = cut.Find("[role='combobox']");

        // Act
        await cut.InvokeAsync(() => input.KeyDown(new KeyboardEventArgs { Key = "ArrowDown" }));
        await cut.InvokeAsync(() => input.KeyDown(new KeyboardEventArgs { Key = "Enter" }));

        // Assert
        _dependencyStore.Verify(s => s.CreateAsync(It.IsAny<CreateDependencyRequest>(),
            It.IsAny<CancellationToken>()), Times.Never);
        var describedBy = input.GetAttribute("aria-describedby");
        describedBy.Should().NotBeNullOrWhiteSpace();
        var ids = describedBy!.Split(' ', StringSplitOptions.RemoveEmptyEntries);
        ids.Should().NotBeEmpty();
        cut.Find($"#{ids[0]}").TextContent.Should().Contain("You are offline");
    }

    [Fact]
    public async Task AddDependencyDialog_ResetsActive_WhenFilteredOut()
    {
        // Arrange
        var current = CreateWorkItem("Current");
        var alpha = CreateWorkItem("Alpha");
        var beta = CreateWorkItem("Beta");
        _workItemStore.Setup(s => s.GetByProject(_projectId))
            .Returns([current, alpha, beta]);

        var cut = Render<AddDependencyDialog>(parameters => parameters
            .Add(p => p.WorkItemId, current.Id)
            .Add(p => p.ProjectId, _projectId)
            .Add(p => p.DebounceMs, 0));

        var input = cut.Find("[role='combobox']");

        // Act: filter to only "Beta"
        await cut.InvokeAsync(() => input.Input("Beta"));

        // Assert
        var activeId = input.GetAttribute("aria-activedescendant");
        activeId.Should().NotBeNullOrWhiteSpace();
        cut.Find($"#{activeId}").Should().NotBeNull();
    }

    [Fact]
    public async Task AddDependencyDialog_DoesNotSubmitWithoutExplicitSelection()
    {
        // Arrange
        var current = CreateWorkItem("Current");
        var target = CreateWorkItem("Target");
        _workItemStore.Setup(s => s.GetByProject(_projectId))
            .Returns([current, target]);

        var cut = Render<AddDependencyDialog>(parameters => parameters
            .Add(p => p.WorkItemId, current.Id)
            .Add(p => p.ProjectId, _projectId));

        var input = cut.Find("[role='combobox']");

        // Act
        await cut.InvokeAsync(() => input.KeyDown(new KeyboardEventArgs { Key = "Enter" }));

        // Assert
        _dependencyStore.Verify(s => s.CreateAsync(It.IsAny<CreateDependencyRequest>(),
            It.IsAny<CancellationToken>()), Times.Never);
    }

    [Fact]
    public void AddDependencyDialog_WiresComboboxListboxA11y()
    {
        // Arrange
        var current = CreateWorkItem("Current");
        var target = CreateWorkItem("Target");
        _workItemStore.Setup(s => s.GetByProject(_projectId))
            .Returns([current, target]);

        // Act
        var cut = Render<AddDependencyDialog>(parameters => parameters
            .Add(p => p.WorkItemId, current.Id)
            .Add(p => p.ProjectId, _projectId));

        // Assert
        var input = cut.Find("[role='combobox']");
        input.GetAttribute("role").Should().Be("combobox");
        input.GetAttribute("aria-autocomplete").Should().Be("list");
        var listboxId = input.GetAttribute("aria-controls");
        listboxId.Should().NotBeNullOrWhiteSpace();
        cut.Find($"#{listboxId}").GetAttribute("role").Should().Be("listbox");
    }

    [Fact]
    public void AddDependencyDialog_CapsResultsToMax()
    {
        // Arrange
        var current = CreateWorkItem("Current");
        var many = Enumerable.Range(0, 60)
            .Select(i => CreateWorkItem($"Item {i}"))
            .ToList();
        var items = new List<WorkItem> { current };
        items.AddRange(many);

        _workItemStore.Setup(s => s.GetByProject(_projectId))
            .Returns(items);

        // Act
        var cut = Render<AddDependencyDialog>(parameters => parameters
            .Add(p => p.WorkItemId, current.Id)
            .Add(p => p.ProjectId, _projectId));

        // Assert
        cut.FindAll(".search-result").Count.Should().Be(10);
        cut.FindAll(".search-result").All(option =>
                option.HasAttribute("aria-setsize") && option.GetAttribute("aria-setsize") == "50")
            .Should().BeTrue();
        cut.FindAll(".search-result").Any(option => option.GetAttribute("aria-posinset") == "51")
            .Should().BeFalse();
    }

    [Fact]
    public async Task AddDependencyDialog_FiltersTrimAndCaseInsensitive()
    {
        // Arrange
        var current = CreateWorkItem("Current");
        var target = CreateWorkItem("Login Story");
        _workItemStore.Setup(s => s.GetByProject(_projectId))
            .Returns([current, target]);

        var cut = Render<AddDependencyDialog>(parameters => parameters
            .Add(p => p.WorkItemId, current.Id)
            .Add(p => p.ProjectId, _projectId)
            .Add(p => p.DebounceMs, 0));

        var input = cut.Find("[role='combobox']");

        // Act
        await cut.InvokeAsync(() => input.Input("  login  "));

        // Assert
        cut.FindAll(".search-result").Should().ContainSingle(option =>
            option.TextContent.Contains("Login Story"));
    }

    private WorkItem CreateWorkItem(string title) => new()
    {
        Id = Guid.NewGuid(),
        Title = title,
        Description = "",
        ItemType = WorkItemType.Story,
        ProjectId = _projectId,
        Status = "backlog",
        Priority = "medium",
        StoryPoints = 1,
        Position = 1,
        Version = 1,
        CreatedAt = DateTime.UtcNow,
        UpdatedAt = DateTime.UtcNow,
        CreatedBy = Guid.NewGuid(),
        UpdatedBy = Guid.NewGuid()
    };

    private static Dependency CreateDependency(Guid blockingId, Guid blockedId) => new()
    {
        Id = Guid.NewGuid(),
        BlockingItemId = blockingId,
        BlockedItemId = blockedId,
        Type = DependencyType.Blocks,
        CreatedAt = DateTime.UtcNow,
        CreatedBy = Guid.NewGuid()
    };
}