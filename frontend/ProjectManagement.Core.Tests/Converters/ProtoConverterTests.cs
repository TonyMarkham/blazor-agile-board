  using FluentAssertions;
  using ProjectManagement.Core.Converters;
  using ProjectManagement.Core.Models;
  using Xunit;

  namespace ProjectManagement.Core.Tests.Converters;

  using Pm = ProjectManagement.Core.Proto;

  public class ProtoConverterTests
  {
      [Fact]
      public void ToProto_WorkItem_ConvertsAllFields()
      {
          var workItem = new WorkItem
          {
              Id = Guid.NewGuid(),
              ItemType = WorkItemType.Story,
              ProjectId = Guid.NewGuid(),
              ParentId = Guid.NewGuid(),
              Title = "Test Story",
              Description = "Description here",
              Status = "in_progress",
              Priority = "high",
              AssigneeId = Guid.NewGuid(),
              StoryPoints = 5,
              SprintId = Guid.NewGuid(),
              Position = 10,
              Version = 3,
              CreatedAt = new DateTime(2024, 1, 15, 12, 0, 0, DateTimeKind.Utc),
              UpdatedAt = new DateTime(2024, 1, 16, 14, 30, 0, DateTimeKind.Utc),
              CreatedBy = Guid.NewGuid(),
              UpdatedBy = Guid.NewGuid()
          };

          var proto = ProtoConverter.ToProto(workItem);

          proto.Id.Should().Be(workItem.Id.ToString());
          proto.ItemType.Should().Be(Pm.WorkItemType.Story);
          proto.ProjectId.Should().Be(workItem.ProjectId.ToString());
          proto.ParentId.Should().Be(workItem.ParentId.ToString());
          proto.Title.Should().Be("Test Story");
          proto.Description.Should().Be("Description here");
          proto.Status.Should().Be("in_progress");
          proto.Priority.Should().Be("high");
          proto.AssigneeId.Should().Be(workItem.AssigneeId.ToString());
          proto.StoryPoints.Should().Be(5);
          proto.SprintId.Should().Be(workItem.SprintId.ToString());
          proto.Position.Should().Be(10);
          proto.Version.Should().Be(3);
      }

      [Fact]
      public void ToDomain_WorkItem_ConvertsAllFields()
      {
          var proto = new Pm.WorkItem
          {
              Id = Guid.NewGuid().ToString(),
              ItemType = Pm.WorkItemType.Task,
              ProjectId = Guid.NewGuid().ToString(),
              Title = "Test Task",
              Status = "backlog",
              Priority = "low",
              Position = 5,
              Version = 1,
              CreatedAt = DateTimeOffset.UtcNow.ToUnixTimeSeconds(),
              UpdatedAt = DateTimeOffset.UtcNow.ToUnixTimeSeconds(),
              CreatedBy = Guid.NewGuid().ToString(),
              UpdatedBy = Guid.NewGuid().ToString()
          };

          var domain = ProtoConverter.ToDomain(proto);

          domain.Id.Should().Be(Guid.Parse(proto.Id));
          domain.ItemType.Should().Be(WorkItemType.Task);
          domain.ProjectId.Should().Be(Guid.Parse(proto.ProjectId));
          domain.Title.Should().Be("Test Task");
          domain.Status.Should().Be("backlog");
          domain.Priority.Should().Be("low");
          domain.Position.Should().Be(5);
          domain.Version.Should().Be(1);
      }

      [Fact]
      public void RoundTrip_WorkItem_PreservesData()
      {
          var original = new WorkItem
          {
              Id = Guid.NewGuid(),
              ItemType = WorkItemType.Epic,
              ProjectId = Guid.NewGuid(),
              Title = "Round Trip Test",
              Status = "done",
              Priority = "medium",
              Version = 5,
              CreatedAt = DateTime.UtcNow,
              UpdatedAt = DateTime.UtcNow,
              CreatedBy = Guid.NewGuid(),
              UpdatedBy = Guid.NewGuid()
          };

          var proto = ProtoConverter.ToProto(original);
          var roundTripped = ProtoConverter.ToDomain(proto);

          roundTripped.Id.Should().Be(original.Id);
          roundTripped.ItemType.Should().Be(original.ItemType);
          roundTripped.ProjectId.Should().Be(original.ProjectId);
          roundTripped.Title.Should().Be(original.Title);
          roundTripped.Status.Should().Be(original.Status);
          roundTripped.Priority.Should().Be(original.Priority);
          roundTripped.Version.Should().Be(original.Version);
      }

      [Theory]
      [InlineData(WorkItemType.Epic, Pm.WorkItemType.Epic)]
      [InlineData(WorkItemType.Story, Pm.WorkItemType.Story)]
      [InlineData(WorkItemType.Task, Pm.WorkItemType.Task)]
      public void ToProto_WorkItemType_MapsCorrectly(WorkItemType domain, Pm.WorkItemType expected)
      {
          var result = ProtoConverter.ToProto(domain);
          result.Should().Be(expected);
      }

      [Fact]
      public void ToDomain_NullOptionalFields_HandlesGracefully()
      {
          var proto = new Pm.WorkItem
          {
              Id = Guid.NewGuid().ToString(),
              ItemType = Pm.WorkItemType.Task,
              ProjectId = Guid.NewGuid().ToString(),
              Title = "Minimal Task",
              Status = "backlog",
              Priority = "medium",
              CreatedAt = DateTimeOffset.UtcNow.ToUnixTimeSeconds(),
              UpdatedAt = DateTimeOffset.UtcNow.ToUnixTimeSeconds(),
              CreatedBy = Guid.NewGuid().ToString(),
              UpdatedBy = Guid.NewGuid().ToString()
          };

          var domain = ProtoConverter.ToDomain(proto);

          domain.ParentId.Should().BeNull();
          domain.Description.Should().BeNull();
          domain.AssigneeId.Should().BeNull();
          domain.SprintId.Should().BeNull();
          domain.StoryPoints.Should().BeNull();
      }

      [Fact]
      public void ToDomain_Timestamp_ConvertsCorrectly()
      {
          var timestamp = new DateTimeOffset(2024, 6, 15, 10, 30, 0, TimeSpan.Zero);
          var proto = new Pm.WorkItem
          {
              Id = Guid.NewGuid().ToString(),
              ItemType = Pm.WorkItemType.Task,
              ProjectId = Guid.NewGuid().ToString(),
              Title = "Timestamp Test",
              Status = "backlog",
              Priority = "medium",
              CreatedAt = timestamp.ToUnixTimeSeconds(),
              UpdatedAt = timestamp.ToUnixTimeSeconds(),
              CreatedBy = Guid.NewGuid().ToString(),
              UpdatedBy = Guid.NewGuid().ToString()
          };

          var domain = ProtoConverter.ToDomain(proto);

          var diff = Math.Abs((domain.CreatedAt - timestamp.UtcDateTime).TotalSeconds);
          diff.Should().BeLessThan(1);
      }

      [Fact]
      public void ToProto_WithOptionalFields_IncludesThem()
      {
          var workItem = new WorkItem
          {
              Id = Guid.NewGuid(),
              ItemType = WorkItemType.Task,
              ProjectId = Guid.NewGuid(),
              ParentId = Guid.NewGuid(),
              Title = "Test",
              Description = "Has description",
              Status = "backlog",
              Priority = "medium",
              AssigneeId = Guid.NewGuid(),
              SprintId = Guid.NewGuid(),
              StoryPoints = 3,
              CreatedAt = DateTime.UtcNow,
              UpdatedAt = DateTime.UtcNow,
              CreatedBy = Guid.NewGuid(),
              UpdatedBy = Guid.NewGuid()
          };

          var proto = ProtoConverter.ToProto(workItem);

          proto.ParentId.Should().NotBeNullOrEmpty();
          proto.Description.Should().Be("Has description");
          proto.AssigneeId.Should().NotBeNullOrEmpty();
          proto.SprintId.Should().NotBeNullOrEmpty();
          proto.StoryPoints.Should().Be(3);
      }

      [Fact]
      public void ToProto_WithoutOptionalFields_OmitsThem()
      {
          var workItem = new WorkItem
          {
              Id = Guid.NewGuid(),
              ItemType = WorkItemType.Task,
              ProjectId = Guid.NewGuid(),
              Title = "Test",
              Status = "backlog",
              Priority = "medium",
              CreatedAt = DateTime.UtcNow,
              UpdatedAt = DateTime.UtcNow,
              CreatedBy = Guid.NewGuid(),
              UpdatedBy = Guid.NewGuid()
          };

          var proto = ProtoConverter.ToProto(workItem);

          proto.ParentId.Should().BeNullOrEmpty();
          proto.Description.Should().BeNullOrEmpty();
          proto.AssigneeId.Should().BeNullOrEmpty();
          proto.SprintId.Should().BeNullOrEmpty();
          proto.StoryPoints.Should().Be(0);
      }

      [Fact]
      public void ToDomain_ActivityLogEntry_ConvertsAllFields()
      {
          var entry = new Pm.ActivityLogEntry
          {
              Id = Guid.NewGuid().ToString(),
              EntityType = "work_item",
              EntityId = Guid.NewGuid().ToString(),
              Action = "updated",
              FieldName = "title",
              OldValue = "Old",
              NewValue = "New",
              UserId = Guid.NewGuid().ToString(),
              Timestamp = DateTimeOffset.UtcNow.ToUnixTimeSeconds(),
              Comment = "Renamed"
          };

          var domain = ProtoConverter.ToDomain(entry);

          domain.Id.Should().Be(Guid.Parse(entry.Id));
          domain.EntityType.Should().Be("work_item");
          domain.EntityId.Should().Be(Guid.Parse(entry.EntityId));
          domain.Action.Should().Be("updated");
          domain.FieldName.Should().Be("title");
          domain.OldValue.Should().Be("Old");
          domain.NewValue.Should().Be("New");
          domain.UserId.Should().Be(Guid.Parse(entry.UserId));
          domain.Comment.Should().Be("Renamed");
      }

      [Fact]
      public void ToDomain_ActivityLogList_ConvertsPagination()
      {
          var list = new Pm.ActivityLogList
          {
              TotalCount = 10,
              HasMore = true
          };
          list.Entries.Add(new Pm.ActivityLogEntry
          {
              Id = Guid.NewGuid().ToString(),
              EntityType = "project",
              EntityId = Guid.NewGuid().ToString(),
              Action = "created",
              UserId = Guid.NewGuid().ToString(),
              Timestamp = DateTimeOffset.UtcNow.ToUnixTimeSeconds()
          });

          var page = ProtoConverter.ToDomain(list);

          page.TotalCount.Should().Be(10);
          page.HasMore.Should().BeTrue();
          page.Entries.Should().HaveCount(1);
      }
  }