namespace ProjectManagement.Core.Interfaces;

/// <summary>
///     Entity with full audit trail (who created/modified and when).
/// </summary>
public interface IAuditable : IEntity, ISoftDeletable
{
    /// <summary>When this entity was last modified (UTC).</summary>
    DateTime UpdatedAt { get; }

    /// <summary>User who created this entity.</summary>
    Guid CreatedBy { get; }

    /// <summary>User who last modified this entity.</summary>
    Guid UpdatedBy { get; }
}