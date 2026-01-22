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
            Version = proto.Version,
            CreatedAt = FromUnixTimestamp(proto.CreatedAt),
            UpdatedAt = FromUnixTimestamp(proto.UpdatedAt),
            CreatedBy = ParseGuid(proto.CreatedBy, "WorkItem.CreatedBy"),
            UpdatedBy = ParseGuid(proto.UpdatedBy, "WorkItem.UpdatedBy"),
            DeletedAt = proto.DeletedAt == 0 ? null : FromUnixTimestamp(proto.DeletedAt)
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
}