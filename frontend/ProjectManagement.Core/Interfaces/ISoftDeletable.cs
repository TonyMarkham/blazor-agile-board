namespace ProjectManagement.Core.Interfaces;

/// <summary>
///     Entity that supports soft deletion for audit trail preservation.
/// </summary>
public interface ISoftDeletable
{
    /// <summary>When this entity was deleted (null if not deleted).</summary>
    DateTime? DeletedAt { get; }

    /// <summary>Whether this entity has been soft-deleted.</summary>
    bool IsDeleted => DeletedAt.HasValue;
}