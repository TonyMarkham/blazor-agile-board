namespace ProjectManagement.Core.Interfaces;

/// <summary>
///     Base interface for all domain entities.
///     Every entity has a unique identifier and creation timestamp.
/// </summary>
public interface IEntity
{
    /// <summary>Unique identifier (UUID).</summary>
    Guid Id { get; }

    /// <summary>When this entity was created (UTC).</summary>
    DateTime CreatedAt { get; }
}