using ProjectManagement.Core.Models;

namespace ProjectManagement.Core.Interfaces;

/// <summary>
///     Store for sprints with specialized operations.
/// </summary>
public interface ISprintStore : IDisposable
{
    event Action? OnChanged;

    IReadOnlyList<Sprint> GetByProject(Guid projectId);
    Sprint? GetById(Guid id);
    Sprint? GetActiveSprint(Guid projectId);

    Task<Sprint> CreateAsync(CreateSprintRequest request, CancellationToken ct = default);
    Task<Sprint> UpdateAsync(UpdateSprintRequest request, CancellationToken ct = default);
    Task<Sprint> StartSprintAsync(Guid sprintId, CancellationToken ct = default);
    Task<Sprint> CompleteSprintAsync(Guid sprintId, CancellationToken ct = default);
    Task DeleteAsync(Guid id, CancellationToken ct = default);
    Task RefreshAsync(Guid projectId, CancellationToken ct = default);
}