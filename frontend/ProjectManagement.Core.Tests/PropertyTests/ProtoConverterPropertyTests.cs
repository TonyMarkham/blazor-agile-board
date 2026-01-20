  using FsCheck.Xunit;
  using ProjectManagement.Core.Converters;
  using ProjectManagement.Core.Models;

  namespace ProjectManagement.Core.Tests.PropertyTests;

  public class ProtoConverterPropertyTests
  {
      [Property]
      public bool WorkItem_RoundTrip_PreservesId(Guid id)
      {
          var original = CreateWorkItem(id);
          var proto = ProtoConverter.ToProto(original);
          var roundTripped = ProtoConverter.ToDomain(proto);

          return roundTripped.Id == original.Id;
      }

      [Property]
      public bool WorkItem_RoundTrip_PreservesVersion(int version)
      {
          if (version < 0) return true;

          var original = CreateWorkItem(Guid.NewGuid()) with { Version = version };
          var proto = ProtoConverter.ToProto(original);
          var roundTripped = ProtoConverter.ToDomain(proto);

          return roundTripped.Version == original.Version;
      }

      [Property]
      public bool WorkItem_RoundTrip_PreservesItemType(int typeValue)
      {
          if (typeValue < 1 || typeValue > 4) return true;

          var itemType = (WorkItemType)typeValue;
          var original = CreateWorkItem(Guid.NewGuid()) with { ItemType = itemType };
          var proto = ProtoConverter.ToProto(original);
          var roundTripped = ProtoConverter.ToDomain(proto);

          return roundTripped.ItemType == original.ItemType;
      }

      [Property]
      public bool Timestamp_RoundTrip_WithinOneSecond(int secondsSinceEpoch)
      {
          if (secondsSinceEpoch < 0 || secondsSinceEpoch > int.MaxValue / 2) return true;

          var timestamp = DateTime.UnixEpoch.AddSeconds(secondsSinceEpoch);
          var original = CreateWorkItem(Guid.NewGuid()) with
          {
              CreatedAt = timestamp,
              UpdatedAt = timestamp
          };

          var proto = ProtoConverter.ToProto(original);
          var roundTripped = ProtoConverter.ToDomain(proto);

          var diff = Math.Abs((roundTripped.CreatedAt - timestamp).TotalSeconds);
          return diff <= 1;
      }

      private static WorkItem CreateWorkItem(Guid id) => new()
      {
          Id = id,
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
  }