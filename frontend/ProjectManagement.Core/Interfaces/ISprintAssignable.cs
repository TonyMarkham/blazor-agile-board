namespace ProjectManagement.Core.Interfaces;

/// <summary>
/// Entity that can be assigned to a sprint.
/// </summary>
public interface ISprintAssignable
{
    /// <summary>The sprint this entity is assigned to (null if in backlog).</summary>
    Guid? SprintId { get; }
}