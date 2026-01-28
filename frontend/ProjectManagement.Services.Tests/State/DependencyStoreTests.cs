using FluentAssertions;
using Microsoft.Extensions.Logging;
using Moq;
using ProjectManagement.Core.Interfaces;
using ProjectManagement.Core.Models;
using ProjectManagement.Services.State;

namespace ProjectManagement.Services.Tests.State;

public class DependencyStoreTests
{
    private readonly Mock<IWebSocketClient> _mockClient;
    private readonly Mock<ILogger<DependencyStore>> _mockLogger;
    private readonly DependencyStore _store;
    private readonly Guid _currentUserId = Guid.NewGuid();

    public DependencyStoreTests()
    {
        _mockClient = new Mock<IWebSocketClient>();
        _mockLogger = new Mock<ILogger<DependencyStore>>();

        // Create real AppState with mocked dependencies
        var mockWorkItems = new Mock<IWorkItemStore>();
        var mockSprints = new Mock<ISprintStore>();
        var mockProjects = new Mock<IProjectStore>();
        var mockComments = new Mock<ICommentStore>();
        var mockAppStateLogger = new Mock<ILogger<AppState>>();

        var appState = new AppState(
            _mockClient.Object,
            mockWorkItems.Object,
            mockSprints.Object,
            mockProjects.Object,
            mockComments.Object,
            mockAppStateLogger.Object
        );

        // Set current user
        var currentUser = new UserIdentity
        {
            Id = _currentUserId,
            Name = "Test User",
            Email = "test@example.com",
            CreatedAt = DateTime.UtcNow
        };
        appState.SetCurrentUser(currentUser);

        _store = new DependencyStore(_mockClient.Object, appState, _mockLogger.Object);
    }

    [Fact]
    public async Task CreateAsync_AddsToStore()
    {
        // Given
        var blockingItemId = Guid.NewGuid();
        var blockedItemId = Guid.NewGuid();
        var request = new CreateDependencyRequest
        {
            BlockingItemId = blockingItemId,
            BlockedItemId = blockedItemId,
            Type = DependencyType.Blocks
        };

        var confirmedDep = new Dependency
        {
            Id = Guid.NewGuid(),
            BlockingItemId = blockingItemId,
            BlockedItemId = blockedItemId,
            Type = DependencyType.Blocks,
            CreatedAt = DateTime.UtcNow,
            CreatedBy = _currentUserId
        };

        _mockClient
            .Setup(c => c.CreateDependencyAsync(request, It.IsAny<CancellationToken>()))
            .ReturnsAsync(confirmedDep);

        // When
        var result = await _store.CreateAsync(request, CancellationToken.None);

        // Then
        result.Should().NotBeNull();
        result.Id.Should().Be(confirmedDep.Id);
        _store.GetBlocking(blockedItemId).Should().ContainSingle();
        _store.GetBlocked(blockingItemId).Should().ContainSingle();
    }

    [Fact]
    public async Task CreateAsync_WhenServerRejects_RollsBack()
    {
        // Given
        var request = new CreateDependencyRequest
        {
            BlockingItemId = Guid.NewGuid(),
            BlockedItemId = Guid.NewGuid(),
            Type = DependencyType.Blocks
        };

        _mockClient
            .Setup(c => c.CreateDependencyAsync(request, It.IsAny<CancellationToken>()))
            .ThrowsAsync(new Exception("Circular dependency detected"));

        // When
        var act = async () => await _store.CreateAsync(request, CancellationToken.None);

        // Then
        await act.Should().ThrowAsync<Exception>();
        _store.GetBlocking(request.BlockedItemId).Should().BeEmpty();
    }

    [Fact]
    public void GetBlocking_ReturnsCorrectItems()
    {
        // Given
        var itemA = Guid.NewGuid();
        var itemB = Guid.NewGuid();
        var itemC = Guid.NewGuid();

        // A blocks B, C blocks B
        var dep1 = new Dependency
        {
            Id = Guid.NewGuid(),
            BlockingItemId = itemA,
            BlockedItemId = itemB,
            Type = DependencyType.Blocks,
            CreatedAt = DateTime.UtcNow,
            CreatedBy = _currentUserId
        };

        var dep2 = new Dependency
        {
            Id = Guid.NewGuid(),
            BlockingItemId = itemC,
            BlockedItemId = itemB,
            Type = DependencyType.Blocks,
            CreatedAt = DateTime.UtcNow,
            CreatedBy = _currentUserId
        };

        _mockClient.Raise(c => c.OnDependencyCreated += null, dep1);
        _mockClient.Raise(c => c.OnDependencyCreated += null, dep2);

        // When
        var blocking = _store.GetBlocking(itemB);

        // Then
        blocking.Should().HaveCount(2);
        blocking.Should().Contain(d => d.BlockingItemId == itemA);
        blocking.Should().Contain(d => d.BlockingItemId == itemC);
    }

    [Fact]
    public void GetBlocked_ReturnsCorrectItems()
    {
        // Given
        var itemA = Guid.NewGuid();
        var itemB = Guid.NewGuid();
        var itemC = Guid.NewGuid();

        // A blocks B and C
        var dep1 = new Dependency
        {
            Id = Guid.NewGuid(),
            BlockingItemId = itemA,
            BlockedItemId = itemB,
            Type = DependencyType.Blocks,
            CreatedAt = DateTime.UtcNow,
            CreatedBy = _currentUserId
        };

        var dep2 = new Dependency
        {
            Id = Guid.NewGuid(),
            BlockingItemId = itemA,
            BlockedItemId = itemC,
            Type = DependencyType.Blocks,
            CreatedAt = DateTime.UtcNow,
            CreatedBy = _currentUserId
        };

        _mockClient.Raise(c => c.OnDependencyCreated += null, dep1);
        _mockClient.Raise(c => c.OnDependencyCreated += null, dep2);

        // When
        var blocked = _store.GetBlocked(itemA);

        // Then
        blocked.Should().HaveCount(2);
        blocked.Should().Contain(d => d.BlockedItemId == itemB);
        blocked.Should().Contain(d => d.BlockedItemId == itemC);
    }

    [Fact]
    public void IsBlocked_TrueWhenHasBlockingDeps()
    {
        // Given
        var itemA = Guid.NewGuid();
        var itemB = Guid.NewGuid();

        var dep = new Dependency
        {
            Id = Guid.NewGuid(),
            BlockingItemId = itemA,
            BlockedItemId = itemB,
            Type = DependencyType.Blocks,
            CreatedAt = DateTime.UtcNow,
            CreatedBy = _currentUserId
        };

        _mockClient.Raise(c => c.OnDependencyCreated += null, dep);

        // When/Then
        _store.IsBlocked(itemB).Should().BeTrue();
        _store.IsBlocked(itemA).Should().BeFalse();
    }

    [Fact]
    public void IsBlocked_FalseForRelatesTo()
    {
        // Given
        var itemA = Guid.NewGuid();
        var itemB = Guid.NewGuid();

        var dep = new Dependency
        {
            Id = Guid.NewGuid(),
            BlockingItemId = itemA,
            BlockedItemId = itemB,
            Type = DependencyType.RelatesTo,
            CreatedAt = DateTime.UtcNow,
            CreatedBy = _currentUserId
        };

        _mockClient.Raise(c => c.OnDependencyCreated += null, dep);

        // When/Then
        _store.IsBlocked(itemB).Should().BeFalse();
    }

    [Fact]
    public void IsBlocked_FalseWhenNoBlockingDeps()
    {
        // Given
        var itemId = Guid.NewGuid();

        // When/Then
        _store.IsBlocked(itemId).Should().BeFalse();
    }

    [Fact]
    public void GetBlockingCount_ReturnsCorrectCount()
    {
        // Given
        var itemB = Guid.NewGuid();

        for (int i = 0; i < 3; i++)
        {
            var dep = new Dependency
            {
                Id = Guid.NewGuid(),
                BlockingItemId = Guid.NewGuid(),
                BlockedItemId = itemB,
                Type = DependencyType.Blocks,
                CreatedAt = DateTime.UtcNow,
                CreatedBy = _currentUserId
            };
            _mockClient.Raise(c => c.OnDependencyCreated += null, dep);
        }

        // When
        var count = _store.GetBlockingCount(itemB);

        // Then
        count.Should().Be(3);
    }

    [Fact]
    public async Task DeleteAsync_RemovesFromStore()
    {
        // Given
        var itemA = Guid.NewGuid();
        var itemB = Guid.NewGuid();
        var depId = Guid.NewGuid();

        var dep = new Dependency
        {
            Id = depId,
            BlockingItemId = itemA,
            BlockedItemId = itemB,
            Type = DependencyType.Blocks,
            CreatedAt = DateTime.UtcNow,
            CreatedBy = _currentUserId
        };

        _mockClient.Raise(c => c.OnDependencyCreated += null, dep);
        _store.GetBlocking(itemB).Should().ContainSingle();

        _mockClient
            .Setup(c => c.DeleteDependencyAsync(depId, It.IsAny<CancellationToken>()))
            .Returns(Task.CompletedTask);

        // When
        await _store.DeleteAsync(depId, CancellationToken.None);

        // Then
        _store.GetBlocking(itemB).Should().BeEmpty();
        _store.IsBlocked(itemB).Should().BeFalse();
    }

    [Fact]
    public void Dispose_UnsubscribesFromEvents()
    {
        // Given
        var dep = new Dependency
        {
            Id = Guid.NewGuid(),
            BlockingItemId = Guid.NewGuid(),
            BlockedItemId = Guid.NewGuid(),
            Type = DependencyType.Blocks,
            CreatedAt = DateTime.UtcNow,
            CreatedBy = _currentUserId
        };

        _mockClient.Raise(c => c.OnDependencyCreated += null, dep);
        _store.GetBlocking(dep.BlockedItemId).Should().ContainSingle();

        // When
        _store.Dispose();

        // Then - No crash when event fires
        var dep2 = new Dependency
        {
            Id = Guid.NewGuid(),
            BlockingItemId = Guid.NewGuid(),
            BlockedItemId = dep.BlockedItemId,
            Type = DependencyType.Blocks,
            CreatedAt = DateTime.UtcNow,
            CreatedBy = _currentUserId
        };
        _mockClient.Raise(c => c.OnDependencyCreated += null, dep2);

        // Store still has only first dep
        _store.GetBlocking(dep.BlockedItemId).Should().ContainSingle();
    }
}