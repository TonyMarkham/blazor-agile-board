namespace ProjectManagement.Core.Models;

/// <summary>
/// Audit log entry for a domain entity.
/// Mirrors ActivityLogEntry in protobuf.
/// </summary>
public sealed record ActivityLog
{
    /// <summary>Unique identifier for this log entry.</summary>
    public Guid Id { get; init; }

    /// <summary>Entity type (e.g., "work_item", "sprint").</summary>
    public string EntityType { get; init; } = string.Empty;

    /// <summary>Entity identifier the action was performed on.</summary>
    public Guid EntityId { get; init; }

    /// <summary>Action performed (e.g., "created", "updated", "deleted").</summary>
    public string Action { get; init; } = string.Empty;

    /// <summary>Optional field name that changed.</summary>
    public string? FieldName { get; init; }

    /// <summary>Optional old value.</summary>
    public string? OldValue { get; init; }

    /// <summary>Optional new value.</summary>
    public string? NewValue { get; init; }

    /// <summary>User who performed the action.</summary>
    public Guid UserId { get; init; }

    /// <summary>When the action occurred (UTC).</summary>
    public DateTime Timestamp { get; init; }

    /// <summary>Optional comment associated with the action.</summary>
    public string? Comment { get; init; }
}