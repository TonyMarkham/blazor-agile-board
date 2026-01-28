using FluentAssertions;
using Microsoft.Extensions.Logging;
using Moq;
using ProjectManagement.Core.Interfaces;
using ProjectManagement.Core.Models;
using ProjectManagement.Services.State;

namespace ProjectManagement.Services.Tests.State;

public class TimeEntryStoreTests
{
    private readonly Mock<IWebSocketClient> _mockClient;
    private readonly Mock<ILogger<TimeEntryStore>> _mockLogger;
    private readonly TimeEntryStore _store;
    private readonly Guid _currentUserId = Guid.NewGuid();

    public TimeEntryStoreTests()
    {
        _mockClient = new Mock<IWebSocketClient>();
        _mockLogger = new Mock<ILogger<TimeEntryStore>>();

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

        _store = new TimeEntryStore(_mockClient.Object, appState, _mockLogger.Object);
    }

    [Fact]
    public async Task StartTimerAsync_CreatesOptimisticThenConfirms()
    {
        // Given
        var workItemId = Guid.NewGuid();
        var request = new StartTimerRequest
        {
            WorkItemId = workItemId,
            Description = "Working on task"
        };

        var confirmedEntry = new TimeEntry
        {
            Id = Guid.NewGuid(),
            WorkItemId = workItemId,
            UserId = _currentUserId,
            StartedAt = DateTime.UtcNow,
            Description = "Working on task",
            CreatedAt = DateTime.UtcNow,
            UpdatedAt = DateTime.UtcNow
        };

        _mockClient
            .Setup(c => c.StartTimerAsync(request, It.IsAny<CancellationToken>()))
            .ReturnsAsync((confirmedEntry, (TimeEntry?)null));

        // When
        var result = await _store.StartTimerAsync(request, CancellationToken.None);

        // Then
        result.Should().NotBeNull();
        result.Id.Should().Be(confirmedEntry.Id);
        result.WorkItemId.Should().Be(workItemId);
        _store.GetByWorkItem(workItemId).Should().ContainSingle();
    }

    [Fact]
    public async Task StartTimerAsync_WhenServerFails_RollsBack()
    {
        // Given
        var workItemId = Guid.NewGuid();
        var request = new StartTimerRequest { WorkItemId = workItemId };

        _mockClient
            .Setup(c => c.StartTimerAsync(request, It.IsAny<CancellationToken>()))
            .ThrowsAsync(new Exception("Server error"));

        // When
        var act = async () => await _store.StartTimerAsync(request, CancellationToken.None);

        // Then
        await act.Should().ThrowAsync<Exception>();
        _store.GetByWorkItem(workItemId).Should().BeEmpty();
    }

    [Fact]
    public async Task StopTimerAsync_UpdatesEntry()
    {
        // Given - Create running entry first
        var workItemId = Guid.NewGuid();
        var entryId = Guid.NewGuid();

        var runningEntry = new TimeEntry
        {
            Id = entryId,
            WorkItemId = workItemId,
            UserId = _currentUserId,
            StartedAt = DateTime.UtcNow.AddHours(-1),
            CreatedAt = DateTime.UtcNow,
            UpdatedAt = DateTime.UtcNow
        };

        _mockClient
            .Setup(c => c.StartTimerAsync(It.IsAny<StartTimerRequest>(), It.IsAny<CancellationToken>()))
            .ReturnsAsync((runningEntry, (TimeEntry?)null));

        await _store.StartTimerAsync(new StartTimerRequest { WorkItemId = workItemId }, CancellationToken.None);

        // Setup stop response
        var stoppedEntry = runningEntry with
        {
            EndedAt = DateTime.UtcNow,
            DurationSeconds = 3600
        };

        _mockClient
            .Setup(c => c.StopTimerAsync(entryId, It.IsAny<CancellationToken>()))
            .ReturnsAsync(stoppedEntry);

        // When
        var result = await _store.StopTimerAsync(entryId, CancellationToken.None);

        // Then
        result.EndedAt.Should().NotBeNull();
        result.DurationSeconds.Should().Be(3600);
        _store.GetRunningTimer().Should().BeNull();
    }

    [Fact]
    public void GetByWorkItem_FiltersDeletedEntries()
    {
        // Given
        var workItemId = Guid.NewGuid();

        var activeEntry = new TimeEntry
        {
            Id = Guid.NewGuid(),
            WorkItemId = workItemId,
            UserId = _currentUserId,
            StartedAt = DateTime.UtcNow,
            CreatedAt = DateTime.UtcNow,
            UpdatedAt = DateTime.UtcNow,
            DeletedAt = null
        };

        var deletedEntry = new TimeEntry
        {
            Id = Guid.NewGuid(),
            WorkItemId = workItemId,
            UserId = _currentUserId,
            StartedAt = DateTime.UtcNow,
            CreatedAt = DateTime.UtcNow,
            UpdatedAt = DateTime.UtcNow,
            DeletedAt = DateTime.UtcNow
        };

        // Trigger events
        _mockClient.Raise(c => c.OnTimeEntryCreated += null, activeEntry);
        _mockClient.Raise(c => c.OnTimeEntryCreated += null, deletedEntry);

        // When
        var entries = _store.GetByWorkItem(workItemId);

        // Then
        entries.Should().ContainSingle();
        entries[0].Id.Should().Be(activeEntry.Id);
    }

    [Fact]
    public void GetByWorkItem_OrdersByStartedAtDescending()
    {
        // Given
        var workItemId = Guid.NewGuid();

        var entry1 = new TimeEntry
        {
            Id = Guid.NewGuid(),
            WorkItemId = workItemId,
            UserId = _currentUserId,
            StartedAt = DateTime.UtcNow.AddHours(-3),
            CreatedAt = DateTime.UtcNow,
            UpdatedAt = DateTime.UtcNow
        };

        var entry2 = new TimeEntry
        {
            Id = Guid.NewGuid(),
            WorkItemId = workItemId,
            UserId = _currentUserId,
            StartedAt = DateTime.UtcNow.AddHours(-1), // Most recent
            CreatedAt = DateTime.UtcNow,
            UpdatedAt = DateTime.UtcNow
        };

        // Add in random order
        _mockClient.Raise(c => c.OnTimeEntryCreated += null, entry1);
        _mockClient.Raise(c => c.OnTimeEntryCreated += null, entry2);

        // When
        var entries = _store.GetByWorkItem(workItemId).ToList();

        // Then
        entries[0].Id.Should().Be(entry2.Id); // Most recent first
        entries[1].Id.Should().Be(entry1.Id);
    }

    [Fact]
    public async Task IsPending_ReturnsTrueForOptimisticUpdates()
    {
        // Given
        var workItemId = Guid.NewGuid();
        var tempId = Guid.NewGuid();

        var request = new StartTimerRequest { WorkItemId = workItemId };
        var tcs = new TaskCompletionSource<(TimeEntry, TimeEntry?)>();

        _mockClient
            .Setup(c => c.StartTimerAsync(request, It.IsAny<CancellationToken>()))
            .Returns(tcs.Task);

        // When
        var startTask = _store.StartTimerAsync(request, CancellationToken.None);
        await Task.Delay(50); // Let optimistic update happen

        var runningTimer = _store.GetRunningTimer();
        runningTimer.Should().NotBeNull();
        _store.IsPending(runningTimer!.Id).Should().BeTrue();

        // Complete server response
        var confirmed = new TimeEntry
        {
            Id = Guid.NewGuid(),
            WorkItemId = workItemId,
            UserId = _currentUserId,
            StartedAt = DateTime.UtcNow,
            CreatedAt = DateTime.UtcNow,
            UpdatedAt = DateTime.UtcNow
        };
        tcs.SetResult((confirmed, null));

        await startTask;
        _store.IsPending(runningTimer.Id).Should().BeFalse();
    }

    [Fact]
    public void Dispose_UnsubscribesFromEvents()
    {
        // Given
        var entry = new TimeEntry
        {
            Id = Guid.NewGuid(),
            WorkItemId = Guid.NewGuid(),
            UserId = _currentUserId,
            StartedAt = DateTime.UtcNow,
            CreatedAt = DateTime.UtcNow,
            UpdatedAt = DateTime.UtcNow
        };

        _mockClient.Raise(c => c.OnTimeEntryCreated += null, entry);
        _store.GetByWorkItem(entry.WorkItemId).Should().ContainSingle();

        // When
        _store.Dispose();

        // Then - No crash when event fires
        var entry2 = new TimeEntry
        {
            Id = Guid.NewGuid(),
            WorkItemId = entry.WorkItemId,
            UserId = _currentUserId,
            StartedAt = DateTime.UtcNow,
            CreatedAt = DateTime.UtcNow,
            UpdatedAt = DateTime.UtcNow
        };
        _mockClient.Raise(c => c.OnTimeEntryCreated += null, entry2);

        // Store still has only first entry
        _store.GetByWorkItem(entry.WorkItemId).Should().ContainSingle();
    }
}