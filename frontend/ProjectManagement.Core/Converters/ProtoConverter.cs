using ProjectManagement.Core.Models;

namespace ProjectManagement.Core.Converters;

using DomainWorkItem = WorkItem;
using DomainWorkItemType = WorkItemType;
using ProtoWorkItem = Proto.WorkItem;
using ProtoWorkItemType = Proto.WorkItemType;

/// <summary>
///     Converts between Protocol Buffer messages and domain models.
///     All conversions are null-safe and validated.
///     NOTE: Timestamps use Unix epoch seconds, losing sub-second precision.
///     Round-trip conversions may differ by up to 1 second.
/// </summary>
public static class ProtoConverter
{
    private static readonly DateTime UnixEpoch = new(1970, 1, 1, 0, 0, 0, DateTimeKind.Utc);

    #region Project Conversions

    /// <summary>
    /// Convert proto Project to domain Project.
    /// </summary>
    public static Project ToDomain(Proto.Project proto)
    {
        ArgumentNullException.ThrowIfNull(proto);

        return new Project
        {
            Id = ParseGuid(proto.Id, "Project.Id"),
            Title = proto.Title ?? string.Empty,
            Description = string.IsNullOrEmpty(proto.Description) ? null : proto.Description,
            Key = proto.Key ?? string.Empty,
            Status = proto.Status switch
            {
                Proto.ProjectStatus.Active => ProjectStatus.Active,
                Proto.ProjectStatus.Archived => ProjectStatus.Archived,
                _ => ProjectStatus.Active
            },
            Version = proto.Version,
            CreatedAt = FromUnixTimestamp(proto.CreatedAt),
            UpdatedAt = FromUnixTimestamp(proto.UpdatedAt),
            CreatedBy = ParseGuid(proto.CreatedBy, "Project.CreatedBy"),
            UpdatedBy = ParseGuid(proto.UpdatedBy, "Project.UpdatedBy"),
            DeletedAt = proto.DeletedAt == 0 ? null : FromUnixTimestamp(proto.DeletedAt),
            NextWorkItemNumber = proto.NextWorkItemNumber,
        };
    }

    /// <summary>
    /// Convert CreateProjectRequest to proto.
    /// </summary>
    public static Proto.CreateProjectRequest ToProto(CreateProjectRequest req)
    {
        ArgumentNullException.ThrowIfNull(req);

        return new Proto.CreateProjectRequest
        {
            Title = req.Title,
            Description = req.Description ?? string.Empty,
            Key = req.Key,
        };
    }

    /// <summary>
    /// Convert UpdateProjectRequest to proto.
    /// </summary>
    public static Proto.UpdateProjectRequest ToProto(UpdateProjectRequest req)
    {
        ArgumentNullException.ThrowIfNull(req);

        var proto = new Proto.UpdateProjectRequest
        {
            ProjectId = req.ProjectId.ToString(),
            ExpectedVersion = req.ExpectedVersion,
        };

        if (req.Title is not null)
            proto.Title = req.Title;

        if (req.Description is not null)
            proto.Description = req.Description;

        if (req.Status.HasValue)
            proto.Status = req.Status.Value switch
            {
                ProjectStatus.Active => Proto.ProjectStatus.Active,
                ProjectStatus.Archived => Proto.ProjectStatus.Archived,
                _ => Proto.ProjectStatus.Active
            };

        return proto;
    }

    /// <summary>
    /// Convert delete request to proto.
    /// </summary>
    public static Proto.DeleteProjectRequest ToDeleteProjectProto(Guid projectId, int expectedVersion)
    {
        return new Proto.DeleteProjectRequest
        {
            ProjectId = projectId.ToString(),
            ExpectedVersion = expectedVersion,
        };
    }

    #endregion

    #region WorkItem Conversions

    public static DomainWorkItem ToDomain(ProtoWorkItem proto)
    {
        ArgumentNullException.ThrowIfNull(proto);

        return new DomainWorkItem
        {
            Id = ParseGuid(proto.Id, "WorkItem.Id"),
            ItemType = ToDomain(proto.ItemType),
            ParentId = string.IsNullOrEmpty(proto.ParentId) ? null : ParseGuid(proto.ParentId, "WorkItem.ParentId"),
            ProjectId = ParseGuid(proto.ProjectId, "WorkItem.ProjectId"),
            Position = proto.Position,
            Title = proto.Title ?? string.Empty,
            Description = string.IsNullOrEmpty(proto.Description) ? null : proto.Description,
            Status = proto.Status ?? "backlog",
            Priority = proto.Priority ?? "medium",
            AssigneeId = string.IsNullOrEmpty(proto.AssigneeId)
                ? null
                : ParseGuid(proto.AssigneeId, "WorkItem.AssigneeId"),
            StoryPoints = proto.StoryPoints == 0 ? null : proto.StoryPoints,
            SprintId = string.IsNullOrEmpty(proto.SprintId) ? null : ParseGuid(proto.SprintId, "WorkItem.SprintId"),
            ItemNumber = proto.ItemNumber,
            Version = proto.Version,
            CreatedAt = FromUnixTimestamp(proto.CreatedAt),
            UpdatedAt = FromUnixTimestamp(proto.UpdatedAt),
            CreatedBy = ParseGuid(proto.CreatedBy, "WorkItem.CreatedBy"),
            UpdatedBy = ParseGuid(proto.UpdatedBy, "WorkItem.UpdatedBy"),
            DeletedAt = proto.DeletedAt == 0 ? null : FromUnixTimestamp(proto.DeletedAt),
            AncestorIds = proto.AncestorIds
                .Where(s => !string.IsNullOrEmpty(s))
                .Select(s => ParseGuid(s, "WorkItem.AncestorIds"))
                .ToList(),
            DescendantIds = proto.DescendantIds
                .Where(s => !string.IsNullOrEmpty(s))
                .Select(s => ParseGuid(s, "WorkItem.DescendantIds"))
                .ToList()
        };
    }

    public static ProtoWorkItem ToProto(DomainWorkItem domain)
    {
        ArgumentNullException.ThrowIfNull(domain);

        var proto = new ProtoWorkItem
        {
            Id = domain.Id.ToString(),
            ItemType = ToProto(domain.ItemType),
            ProjectId = domain.ProjectId.ToString(),
            Position = domain.Position,
            Title = domain.Title,
            Status = domain.Status,
            Priority = domain.Priority,
            ItemNumber = domain.ItemNumber,
            Version = domain.Version,
            CreatedAt = ToUnixTimestamp(domain.CreatedAt),
            UpdatedAt = ToUnixTimestamp(domain.UpdatedAt),
            CreatedBy = domain.CreatedBy.ToString(),
            UpdatedBy = domain.UpdatedBy.ToString()
        };

        if (domain.ParentId.HasValue)
            proto.ParentId = domain.ParentId.Value.ToString();
        if (!string.IsNullOrEmpty(domain.Description))
            proto.Description = domain.Description;
        if (domain.AssigneeId.HasValue)
            proto.AssigneeId = domain.AssigneeId.Value.ToString();
        if (domain.StoryPoints.HasValue)
            proto.StoryPoints = domain.StoryPoints.Value;
        if (domain.SprintId.HasValue)
            proto.SprintId = domain.SprintId.Value.ToString();
        if (domain.DeletedAt.HasValue)
            proto.DeletedAt = ToUnixTimestamp(domain.DeletedAt.Value);

        // Map hierarchy fields
        proto.AncestorIds.AddRange(domain.AncestorIds.Select(id => id.ToString()));
        proto.DescendantIds.AddRange(domain.DescendantIds.Select(id => id.ToString()));

        return proto;
    }
    
    /// <summary>
    /// Convert UpdateWorkItemRequest to proto.
    /// Only includes fields that are set (non-null).
    /// </summary>
    public static Proto.UpdateWorkItemRequest ToProto(UpdateWorkItemRequest req)
    {
        ArgumentNullException.ThrowIfNull(req);

        var proto = new Proto.UpdateWorkItemRequest
        {
            WorkItemId = req.WorkItemId.ToString(),
            ExpectedVersion = req.ExpectedVersion,
            UpdateParent = req.UpdateParent,
        };

        // Only set optional fields if they have values
        if (req.Title is not null)
            proto.Title = req.Title;

        if (req.Description is not null)
            proto.Description = req.Description;

        if (req.Status is not null)
            proto.Status = req.Status;

        if (req.Priority is not null)
            proto.Priority = req.Priority;

        if (req.AssigneeId.HasValue)
            proto.AssigneeId = req.AssigneeId.Value.ToString();

        if (req.SprintId.HasValue)
            proto.SprintId = req.SprintId.Value.ToString();

        if (req.Position.HasValue)
            proto.Position = req.Position.Value;

        if (req.StoryPoints.HasValue)
            proto.StoryPoints = req.StoryPoints.Value;

        // Handle parent assignment (Feature C)
        if (req.UpdateParent)
        {
            // Set parent_id:
            // - Empty string to clear parent (if ParentId is null or Guid.Empty)
            // - UUID string to set parent
            proto.ParentId = req.ParentId == Guid.Empty || req.ParentId == null
                ? string.Empty
                : req.ParentId.Value.ToString();
        }
        // If UpdateParent is false, don't set ParentId field at all

        return proto;
    }

    #endregion

    #region Enum Conversions

    public static DomainWorkItemType ToDomain(ProtoWorkItemType proto)
    {
        return proto switch
        {
            ProtoWorkItemType.Epic => DomainWorkItemType.Epic,
            ProtoWorkItemType.Story => DomainWorkItemType.Story,
            ProtoWorkItemType.Task => DomainWorkItemType.Task,
            _ => throw new ArgumentOutOfRangeException(nameof(proto), $"Unknown WorkItemType: {proto}")
        };
    }

    public static ProtoWorkItemType ToProto(DomainWorkItemType domain)
    {
        return domain switch
        {
            DomainWorkItemType.Epic => ProtoWorkItemType.Epic,
            DomainWorkItemType.Story => ProtoWorkItemType.Story,
            DomainWorkItemType.Task => ProtoWorkItemType.Task,
            _ => throw new ArgumentOutOfRangeException(nameof(domain), $"Unknown WorkItemType: {domain}")
        };
    }

    #endregion

    #region Helper Methods

    private static Guid ParseGuid(string value, string fieldName)
    {
        if (string.IsNullOrEmpty(value))
            throw new ArgumentException($"{fieldName} cannot be empty", fieldName);

        if (!Guid.TryParse(value, out var guid))
            throw new ArgumentException($"{fieldName} is not a valid GUID: {value}", fieldName);

        return guid;
    }

    private static DateTime FromUnixTimestamp(long timestamp)
    {
        return UnixEpoch.AddSeconds(timestamp);
    }

    private static long ToUnixTimestamp(DateTime dateTime)
    {
        return (long)(dateTime.ToUniversalTime() - UnixEpoch).TotalSeconds;
    }

    #endregion

    #region Sprint Conversions

    /// <summary>
    /// Convert protobuf Sprint to domain Sprint.
    /// </summary>
    public static Sprint ToDomain(Proto.Sprint proto)
    {
        ArgumentNullException.ThrowIfNull(proto);

        return new Sprint
        {
            Id = ParseGuid(proto.Id, "Sprint.Id"),
            ProjectId = ParseGuid(proto.ProjectId, "Sprint.ProjectId"),
            Name = proto.Name ?? string.Empty,
            Goal = string.IsNullOrEmpty(proto.Goal) ? null : proto.Goal,
            StartDate = FromUnixTimestamp(proto.StartDate),
            EndDate = FromUnixTimestamp(proto.EndDate),
            Status = ToDomain(proto.Status),
            Version = proto.Version,
            CreatedAt = FromUnixTimestamp(proto.CreatedAt),
            UpdatedAt = FromUnixTimestamp(proto.UpdatedAt),
            CreatedBy = ParseGuid(proto.CreatedBy, "Sprint.CreatedBy"),
            UpdatedBy = ParseGuid(proto.UpdatedBy, "Sprint.UpdatedBy"),
            DeletedAt = proto.DeletedAt == 0 ? null : FromUnixTimestamp(proto.DeletedAt),
        };
    }

    /// <summary>
    /// Convert CreateSprintRequest to protobuf.
    /// </summary>
    public static Proto.CreateSprintRequest ToProto(CreateSprintRequest req)
    {
        ArgumentNullException.ThrowIfNull(req);

        return new Proto.CreateSprintRequest
        {
            ProjectId = req.ProjectId.ToString(),
            Name = req.Name,
            Goal = req.Goal ?? string.Empty,
            StartDate = ToUnixTimestamp(req.StartDate),
            EndDate = ToUnixTimestamp(req.EndDate),
        };
    }

    /// <summary>
    /// Convert UpdateSprintRequest to protobuf.
    /// Only includes fields that are set (non-null).
    /// </summary>
    public static Proto.UpdateSprintRequest ToProto(UpdateSprintRequest req)
    {
        ArgumentNullException.ThrowIfNull(req);

        var proto = new Proto.UpdateSprintRequest
        {
            SprintId = req.SprintId.ToString(),
            ExpectedVersion = req.ExpectedVersion,
        };

        if (req.Name is not null) proto.Name = req.Name;
        if (req.Goal is not null) proto.Goal = req.Goal;
        if (req.StartDate.HasValue) proto.StartDate = ToUnixTimestamp(req.StartDate.Value);
        if (req.EndDate.HasValue) proto.EndDate = ToUnixTimestamp(req.EndDate.Value);
        if (req.Status.HasValue) proto.Status = ToProto(req.Status.Value);

        return proto;
    }

    /// <summary>
    /// Convert protobuf SprintStatus to domain SprintStatus.
    /// </summary>
    public static SprintStatus ToDomain(Proto.SprintStatus proto)
    {
        return proto switch
        {
            Proto.SprintStatus.Planned => SprintStatus.Planned,
            Proto.SprintStatus.Active => SprintStatus.Active,
            Proto.SprintStatus.Completed => SprintStatus.Completed,
            Proto.SprintStatus.Cancelled => SprintStatus.Cancelled,
            _ => SprintStatus.Planned
        };
    }

    /// <summary>
    /// Convert domain SprintStatus to protobuf.
    /// </summary>
    public static Proto.SprintStatus ToProto(SprintStatus domain)
    {
        return domain switch
        {
            SprintStatus.Planned => Proto.SprintStatus.Planned,
            SprintStatus.Active => Proto.SprintStatus.Active,
            SprintStatus.Completed => Proto.SprintStatus.Completed,
            SprintStatus.Cancelled => Proto.SprintStatus.Cancelled,
            _ => Proto.SprintStatus.Planned
        };
    }

    #endregion

    #region Comment Conversions

    /// <summary>
    /// Convert protobuf Comment to domain Comment.
    /// </summary>
    public static Comment ToDomain(Proto.Comment proto)
    {
        ArgumentNullException.ThrowIfNull(proto);

        return new Comment
        {
            Id = ParseGuid(proto.Id, "Comment.Id"),
            WorkItemId = ParseGuid(proto.WorkItemId, "Comment.WorkItemId"),
            Content = proto.Content ?? string.Empty,
            CreatedAt = FromUnixTimestamp(proto.CreatedAt),
            UpdatedAt = FromUnixTimestamp(proto.UpdatedAt),
            CreatedBy = ParseGuid(proto.CreatedBy, "Comment.CreatedBy"),
            UpdatedBy = ParseGuid(proto.UpdatedBy, "Comment.UpdatedBy"),
            DeletedAt = proto.DeletedAt == 0 ? null : FromUnixTimestamp(proto.DeletedAt),
        };
    }

    /// <summary>
    /// Convert CreateCommentRequest to protobuf.
    /// </summary>
    public static Proto.CreateCommentRequest ToProto(CreateCommentRequest req)
    {
        ArgumentNullException.ThrowIfNull(req);

        return new Proto.CreateCommentRequest
        {
            WorkItemId = req.WorkItemId.ToString(),
            Content = req.Content,
        };
    }

    /// <summary>
    /// Convert UpdateCommentRequest to protobuf.
    /// </summary>
    public static Proto.UpdateCommentRequest ToProto(UpdateCommentRequest req)
    {
        ArgumentNullException.ThrowIfNull(req);

        return new Proto.UpdateCommentRequest
        {
            CommentId = req.CommentId.ToString(),
            Content = req.Content,
        };
    }

    #endregion

    #region TimeEntry Conversions

    // ==========================================================================
    // Time Entry Conversions
    // ==========================================================================

    /// <summary>
    /// Convert protobuf TimeEntry to domain TimeEntry.
    /// </summary>
    public static TimeEntry ToDomain(Proto.TimeEntry proto)
    {
        ArgumentNullException.ThrowIfNull(proto);

        return new TimeEntry
        {
            Id = ParseGuid(proto.Id, "TimeEntry.Id"),
            WorkItemId = ParseGuid(proto.WorkItemId, "TimeEntry.WorkItemId"),
            UserId = ParseGuid(proto.UserId, "TimeEntry.UserId"),
            StartedAt = FromUnixTimestamp(proto.StartedAt),
            EndedAt = proto.HasEndedAt
                ? FromUnixTimestamp(proto.EndedAt)
                : null,
            DurationSeconds = proto.HasDurationSeconds ? proto.DurationSeconds : null,
            Description = proto.HasDescription ? proto.Description : null,
            CreatedAt = FromUnixTimestamp(proto.CreatedAt),
            UpdatedAt = FromUnixTimestamp(proto.UpdatedAt),
            DeletedAt = proto.HasDeletedAt
                ? FromUnixTimestamp(proto.DeletedAt)
                : null,
        };
    }

    /// <summary>
    /// Convert domain TimeEntry to protobuf TimeEntry.
    /// </summary>
    public static Proto.TimeEntry ToProto(TimeEntry entry)
    {
        ArgumentNullException.ThrowIfNull(entry);

        var proto = new Proto.TimeEntry
        {
            Id = entry.Id.ToString(),
            WorkItemId = entry.WorkItemId.ToString(),
            UserId = entry.UserId.ToString(),
            StartedAt = ToUnixTimestamp(entry.StartedAt),
            CreatedAt = ToUnixTimestamp(entry.CreatedAt),
            UpdatedAt = ToUnixTimestamp(entry.UpdatedAt),
        };

        if (entry.EndedAt.HasValue)
        {
            proto.EndedAt = ToUnixTimestamp(entry.EndedAt.Value);
        }

        if (entry.DurationSeconds.HasValue)
        {
            proto.DurationSeconds = entry.DurationSeconds.Value;
        }

        if (entry.Description != null)
        {
            proto.Description = entry.Description;
        }

        if (entry.DeletedAt.HasValue)
        {
            proto.DeletedAt = ToUnixTimestamp(entry.DeletedAt.Value);
        }

        return proto;
    }

    #endregion

    #region Dependency Conversions

    // ==========================================================================
    // Dependency Conversions
    // ==========================================================================

    /// <summary>
    /// Convert protobuf Dependency to domain Dependency.
    /// </summary>
    public static Dependency ToDomain(Proto.Dependency proto)
    {
        ArgumentNullException.ThrowIfNull(proto);

        return new Dependency
        {
            Id = ParseGuid(proto.Id, "Dependency.Id"),
            BlockingItemId = ParseGuid(proto.BlockingItemId, "Dependency.BlockingItemId"),
            BlockedItemId = ParseGuid(proto.BlockedItemId, "Dependency.BlockedItemId"),
            Type = ToDomain(proto.DependencyType),
            CreatedAt = FromUnixTimestamp(proto.CreatedAt),
            CreatedBy = ParseGuid(proto.CreatedBy, "Dependency.CreatedBy"),
            DeletedAt = proto.HasDeletedAt
                ? FromUnixTimestamp(proto.DeletedAt)
                : null,
        };
    }

    /// <summary>
    /// Convert domain Dependency to protobuf Dependency.
    /// </summary>
    public static Proto.Dependency ToProto(Dependency dep)
    {
        ArgumentNullException.ThrowIfNull(dep);

        var proto = new Proto.Dependency
        {
            Id = dep.Id.ToString(),
            BlockingItemId = dep.BlockingItemId.ToString(),
            BlockedItemId = dep.BlockedItemId.ToString(),
            DependencyType = ToProto(dep.Type),
            CreatedAt = ToUnixTimestamp(dep.CreatedAt),
            CreatedBy = dep.CreatedBy.ToString(),
        };

        if (dep.DeletedAt.HasValue)
        {
            proto.DeletedAt = ToUnixTimestamp(dep.DeletedAt.Value);
        }

        return proto;
    }

    /// <summary>
    /// Convert protobuf DependencyType to domain DependencyType.
    /// </summary>
    public static DependencyType ToDomain(Proto.DependencyType proto)
    {
        return proto switch
        {
            Proto.DependencyType.Blocks => DependencyType.Blocks,
            Proto.DependencyType.RelatesTo => DependencyType.RelatesTo,
            _ => throw new ArgumentOutOfRangeException(nameof(proto), proto, "Unknown dependency type")
        };
    }

    /// <summary>
    /// Convert domain DependencyType to protobuf DependencyType.
    /// </summary>
    public static Proto.DependencyType ToProto(DependencyType type)
    {
        return type switch
        {
            DependencyType.Blocks => Proto.DependencyType.Blocks,
            DependencyType.RelatesTo => Proto.DependencyType.RelatesTo,
            _ => throw new ArgumentOutOfRangeException(nameof(type), type, "Unknown dependency type")
        };
    }

    #endregion

    #region Activity Log Conversions

    /// <summary>
    /// Convert protobuf ActivityLogEntry to domain ActivityLog.
    /// </summary>
    public static ActivityLog ToDomain(Proto.ActivityLogEntry proto)
    {
        ArgumentNullException.ThrowIfNull(proto);

        return new ActivityLog
        {
            Id = ParseGuid(proto.Id, "ActivityLogEntry.Id"),
            EntityType = proto.EntityType ?? string.Empty,
            EntityId = ParseGuid(proto.EntityId, "ActivityLogEntry.EntityId"),
            Action = proto.Action ?? string.Empty,
            FieldName = proto.HasFieldName ? proto.FieldName : null,
            OldValue = proto.HasOldValue ? proto.OldValue : null,
            NewValue = proto.HasNewValue ? proto.NewValue : null,
            UserId = ParseGuid(proto.UserId, "ActivityLogEntry.UserId"),
            Timestamp = FromUnixTimestamp(proto.Timestamp),
            Comment = proto.HasComment ? proto.Comment : null
        };
    }

    /// <summary>
    /// Convert protobuf ActivityLogList to domain ActivityLogPage.
    /// </summary>
    public static ActivityLogPage ToDomain(Proto.ActivityLogList proto)
    {
        ArgumentNullException.ThrowIfNull(proto);

        return new ActivityLogPage
        {
            Entries = proto.Entries.Select(ToDomain).ToList(),
            TotalCount = proto.TotalCount,
            HasMore = proto.HasMore
        };
    }

    #endregion
}