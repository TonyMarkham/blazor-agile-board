namespace ProjectManagement.Core.Interfaces;

/// <summary>
///     Entity that can have a parent (tree structure).
/// </summary>
/// <typeparam name="T">The entity type (self-referential).</typeparam>
public interface IHierarchical<T> where T : IEntity
{
    /// <summary>Parent entity ID (null for root entities).</summary>
    Guid? ParentId { get; }
}