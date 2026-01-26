  using FluentAssertions;
  using Microsoft.Extensions.Logging;
  using Moq;
  using ProjectManagement.Core.Interfaces;
  using ProjectManagement.Core.Models;
  using ProjectManagement.Services.State;
  using Xunit;

  namespace ProjectManagement.Services.Tests.State;

  public class WorkItemStoreTests
  {
      private readonly WorkItemStore _sut;
      private readonly Mock<IWebSocketClient> _client;
      private readonly Mock<ILogger<WorkItemStore>> _logger;

      public WorkItemStoreTests()
      {
          _client = new Mock<IWebSocketClient>();
          _logger = new Mock<ILogger<WorkItemStore>>();
          _sut = new WorkItemStore(_client.Object, _logger.Object);
      }

      [Fact]
      public void GetByProject_Empty_ReturnsEmptyList()
      {
          var result = _sut.GetByProject(Guid.NewGuid());

          result.Should().BeEmpty();
      }

      [Fact]
      public async Task CreateAsync_AppliesOptimistically()
      {
          var projectId = Guid.NewGuid();
          var confirmedId = Guid.NewGuid();
          var request = new CreateWorkItemRequest
          {
              ItemType = WorkItemType.Task,
              Title = "Test Task",
              ProjectId = projectId
          };

          _client.Setup(c => c.CreateWorkItemAsync(request, It.IsAny<CancellationToken>()))
              .ReturnsAsync(new WorkItem
              {
                  Id = confirmedId,
                  ItemType = WorkItemType.Task,
                  Title = "Test Task",
                  ProjectId = projectId,
                  Status = "backlog",
                  Priority = "medium",
                  CreatedAt = DateTime.UtcNow,
                  UpdatedAt = DateTime.UtcNow
              });

          var changedCount = 0;
          _sut.OnChanged += () => changedCount++;

          var result = await _sut.CreateAsync(request);

          result.Id.Should().Be(confirmedId);
          changedCount.Should().BeGreaterOrEqualTo(1);
          _sut.GetById(confirmedId).Should().NotBeNull();
      }

      [Fact]
      public async Task CreateAsync_RollsBackOnFailure()
      {
          var projectId = Guid.NewGuid();
          var request = new CreateWorkItemRequest
          {
              ItemType = WorkItemType.Task,
              Title = "Test Task",
              ProjectId = projectId
          };

          _client.Setup(c => c.CreateWorkItemAsync(request, It.IsAny<CancellationToken>()))
              .ThrowsAsync(new Exception("Server error"));

          var act = () => _sut.CreateAsync(request);

          await act.Should().ThrowAsync<Exception>();
          _sut.GetByProject(projectId).Should().BeEmpty();
      }

      [Fact]
      public async Task UpdateAsync_AppliesOptimistically()
      {
          var workItemId = Guid.NewGuid();
          var projectId = Guid.NewGuid();

          var existing = new WorkItem
          {
              Id = workItemId,
              ItemType = WorkItemType.Task,
              Title = "Original Title",
              ProjectId = projectId,
              Status = "backlog",
              Priority = "medium",
              Version = 1,
              CreatedAt = DateTime.UtcNow,
              UpdatedAt = DateTime.UtcNow
          };

          _client.Raise(c => c.OnWorkItemCreated += null, existing);

          var request = new UpdateWorkItemRequest
          {
              WorkItemId = workItemId,
              ExpectedVersion = 1,
              Title = "Updated Title"
          };

          _client.Setup(c => c.UpdateWorkItemAsync(request, It.IsAny<CancellationToken>()))
              .ReturnsAsync(existing with
              {
                  Title = "Updated Title",
                  Version = 2,
                  UpdatedAt = DateTime.UtcNow
              });

          var result = await _sut.UpdateAsync(request);

          result.Title.Should().Be("Updated Title");
          result.Version.Should().Be(2);
      }

      [Fact]
      public async Task UpdateAsync_RollsBackOnFailure()
      {
          var workItemId = Guid.NewGuid();
          var projectId = Guid.NewGuid();

          var existing = new WorkItem
          {
              Id = workItemId,
              ItemType = WorkItemType.Task,
              Title = "Original Title",
              ProjectId = projectId,
              Status = "backlog",
              Priority = "medium",
              Version = 1,
              CreatedAt = DateTime.UtcNow,
              UpdatedAt = DateTime.UtcNow
          };

          _client.Raise(c => c.OnWorkItemCreated += null, existing);

          var request = new UpdateWorkItemRequest
          {
              WorkItemId = workItemId,
              ExpectedVersion = 1,
              Title = "Updated Title"
          };

          _client.Setup(c => c.UpdateWorkItemAsync(request, It.IsAny<CancellationToken>()))
              .ThrowsAsync(new Exception("Server error"));

          var act = () => _sut.UpdateAsync(request);

          await act.Should().ThrowAsync<Exception>();

          var item = _sut.GetById(workItemId);
          item.Should().NotBeNull();
          item!.Title.Should().Be("Original Title");
      }

      [Fact]
      public async Task DeleteAsync_AppliesOptimisticSoftDelete()
      {
          var workItemId = Guid.NewGuid();
          var projectId = Guid.NewGuid();

          var existing = new WorkItem
          {
              Id = workItemId,
              ItemType = WorkItemType.Task,
              Title = "Test Task",
              ProjectId = projectId,
              Status = "backlog",
              Priority = "medium",
              CreatedAt = DateTime.UtcNow,
              UpdatedAt = DateTime.UtcNow
          };

          _client.Raise(c => c.OnWorkItemCreated += null, existing);
          _client.Setup(c => c.DeleteWorkItemAsync(workItemId, It.IsAny<CancellationToken>()))
              .Returns(Task.CompletedTask);

          await _sut.DeleteAsync(workItemId);

          _sut.GetById(workItemId).Should().BeNull();
          _sut.GetByProject(projectId).Should().BeEmpty();
      }

      [Fact]
      public void HandleBroadcast_WorkItemCreated_AddsToStore()
      {
          var workItem = new WorkItem
          {
              Id = Guid.NewGuid(),
              ItemType = WorkItemType.Task,
              Title = "Broadcast Task",
              ProjectId = Guid.NewGuid(),
              Status = "backlog",
              Priority = "medium",
              CreatedAt = DateTime.UtcNow,
              UpdatedAt = DateTime.UtcNow
          };

          _client.Raise(c => c.OnWorkItemCreated += null, workItem);

          _sut.GetById(workItem.Id).Should().NotBeNull();
      }

      [Fact]
      public void HandleBroadcast_WorkItemDeleted_SoftDeletes()
      {
          var workItemId = Guid.NewGuid();
          var projectId = Guid.NewGuid();

          var existing = new WorkItem
          {
              Id = workItemId,
              ItemType = WorkItemType.Task,
              Title = "Test Task",
              ProjectId = projectId,
              Status = "backlog",
              Priority = "medium",
              CreatedAt = DateTime.UtcNow,
              UpdatedAt = DateTime.UtcNow
          };

          _client.Raise(c => c.OnWorkItemCreated += null, existing);
          _client.Raise(c => c.OnWorkItemDeleted += null, workItemId);

          _sut.GetById(workItemId).Should().BeNull();
      }

      [Fact]
      public void GetBySprint_FiltersCorrectly()
      {
          var sprintId = Guid.NewGuid();
          var projectId = Guid.NewGuid();

          var inSprint = new WorkItem
          {
              Id = Guid.NewGuid(),
              ItemType = WorkItemType.Task,
              Title = "In Sprint",
              ProjectId = projectId,
              SprintId = sprintId,
              Status = "backlog",
              Priority = "medium",
              CreatedAt = DateTime.UtcNow,
              UpdatedAt = DateTime.UtcNow
          };

          var notInSprint = new WorkItem
          {
              Id = Guid.NewGuid(),
              ItemType = WorkItemType.Task,
              Title = "Not In Sprint",
              ProjectId = projectId,
              SprintId = null,
              Status = "backlog",
              Priority = "medium",
              CreatedAt = DateTime.UtcNow,
              UpdatedAt = DateTime.UtcNow
          };

          _client.Raise(c => c.OnWorkItemCreated += null, inSprint);
          _client.Raise(c => c.OnWorkItemCreated += null, notInSprint);

          var result = _sut.GetBySprint(sprintId);

          result.Should().HaveCount(1);
          result[0].Title.Should().Be("In Sprint");
      }

      [Fact]
      public void GetChildren_FiltersCorrectly()
      {
          var parentId = Guid.NewGuid();
          var projectId = Guid.NewGuid();

          var parent = new WorkItem
          {
              Id = parentId,
              ItemType = WorkItemType.Story,
              Title = "Parent Story",
              ProjectId = projectId,
              Status = "backlog",
              Priority = "medium",
              CreatedAt = DateTime.UtcNow,
              UpdatedAt = DateTime.UtcNow
          };

          var child = new WorkItem
          {
              Id = Guid.NewGuid(),
              ItemType = WorkItemType.Task,
              Title = "Child Task",
              ProjectId = projectId,
              ParentId = parentId,
              Status = "backlog",
              Priority = "medium",
              CreatedAt = DateTime.UtcNow,
              UpdatedAt = DateTime.UtcNow
          };

          _client.Raise(c => c.OnWorkItemCreated += null, parent);
          _client.Raise(c => c.OnWorkItemCreated += null, child);

          var result = _sut.GetChildren(parentId);

          result.Should().HaveCount(1);
          result[0].Title.Should().Be("Child Task");
      }

      [Fact]
      public void OnChanged_FiresOnStateChanges()
      {
          var changedCount = 0;
          _sut.OnChanged += () => changedCount++;

          var workItem = new WorkItem
          {
              Id = Guid.NewGuid(),
              ItemType = WorkItemType.Task,
              Title = "Test Task",
              ProjectId = Guid.NewGuid(),
              Status = "backlog",
              Priority = "medium",
              CreatedAt = DateTime.UtcNow,
              UpdatedAt = DateTime.UtcNow
          };

          _client.Raise(c => c.OnWorkItemCreated += null, workItem);

          changedCount.Should().BeGreaterThan(0);
      }
  }