namespace ProjectManagement.Core.Interfaces;

/// <summary>
/// Store for project-scoped entities.
/// </summary>
public interface IProjectScopedStore<T> : IEntityStore<T>
    where T : IEntity, ISoftDeletable, IProjectScoped
{
    /// <summary>Get all entities for a specific project.</summary>
    IReadOnlyList<T> GetByProject(Guid projectId);

    /// <summary>Refresh data for a specific project from server.</summary>
    Task RefreshAsync(Guid projectId, CancellationToken ct = default);
}