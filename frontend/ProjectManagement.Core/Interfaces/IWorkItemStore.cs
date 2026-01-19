using ProjectManagement.Core.Models;

namespace ProjectManagement.Core.Interfaces;

/// <summary>
///     Store for work items with specialized queries.
/// </summary>
public interface IWorkItemStore : IDisposable
{
    event Action? OnChanged;

    IReadOnlyList<WorkItem> GetByProject(Guid projectId);
    WorkItem? GetById(Guid id);
    IReadOnlyList<WorkItem> GetBySprint(Guid sprintId);
    IReadOnlyList<WorkItem> GetChildren(Guid parentId);

    Task<WorkItem> CreateAsync(CreateWorkItemRequest request, CancellationToken ct = default);
    Task<WorkItem> UpdateAsync(UpdateWorkItemRequest request, CancellationToken ct = default);
    Task DeleteAsync(Guid id, CancellationToken ct = default);
    Task RefreshAsync(Guid projectId, CancellationToken ct = default);
}