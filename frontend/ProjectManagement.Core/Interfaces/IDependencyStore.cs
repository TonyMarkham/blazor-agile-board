using ProjectManagement.Core.Models;

namespace ProjectManagement.Core.Interfaces;

/// <summary>
/// Store for managing dependency state.
/// Tracks both blocking and blocked relationships.
/// </summary>
public interface IDependencyStore : IDisposable
{
    /// <summary>Fired when any dependency state changes.</summary>
    event Action? OnChanged;

    /// <summary>
    /// Get items that are blocking this work item.
    /// These must be completed before this item can start.
    /// </summary>
    IReadOnlyList<Dependency> GetBlocking(Guid workItemId);

    /// <summary>
    /// Get items that this work item blocks.
    /// These cannot start until this item is complete.
    /// </summary>
    IReadOnlyList<Dependency> GetBlocked(Guid workItemId);

    /// <summary>
    /// Check if a work item has any blocking dependencies.
    /// Use this for showing blocked indicators in the UI.
    /// </summary>
    bool IsBlocked(Guid workItemId);

    /// <summary>
    /// Check if a dependency has a pending server operation.
    /// </summary>
    bool IsPending(Guid dependencyId);

    /// <summary>
    /// Create a dependency between two work items.
    /// Uses optimistic update pattern.
    /// </summary>
    Task<Dependency> CreateAsync(CreateDependencyRequest request, CancellationToken ct = default);

    /// <summary>
    /// Delete a dependency.
    /// Requires Edit permission on the project.
    /// </summary>
    Task DeleteAsync(Guid dependencyId, CancellationToken ct = default);

    /// <summary>
    /// Refresh dependencies for a work item from the server.
    /// Call when navigating to a work item detail view.
    /// </summary>
    Task RefreshAsync(Guid workItemId, CancellationToken ct = default);

    /// <summary>
    /// Get the count of items blocking this work item.
    /// Useful for badge displays.
    /// </summary>
    int GetBlockingCount(Guid workItemId);
}