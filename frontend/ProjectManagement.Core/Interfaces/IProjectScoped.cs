namespace ProjectManagement.Core.Interfaces;

/// <summary>
/// Entity that belongs to a specific project.
/// Enables project-level filtering and authorization.
/// </summary>
public interface IProjectScoped
{
    /// <summary>The project this entity belongs to.</summary>
    Guid ProjectId { get; }
}