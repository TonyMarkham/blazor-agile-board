using ProjectManagement.Core.Interfaces;
using ProjectManagement.Core.Models;

namespace ProjectManagement.Core.ViewModels;

/// <summary>
/// Factory for creating ViewModels with proper pending state.
/// Registered as Scoped service to access stores.
/// </summary>
public sealed class ViewModelFactory
{
    private readonly IWorkItemStore _workItemStore;
    private readonly ISprintStore _sprintStore;

    public ViewModelFactory(IWorkItemStore workItemStore, ISprintStore sprintStore)
    {
        ArgumentNullException.ThrowIfNull(workItemStore);
        ArgumentNullException.ThrowIfNull(sprintStore);
        _workItemStore = workItemStore;
        _sprintStore = sprintStore;
    }

    /// <summary>
    /// Create a WorkItemViewModel from a WorkItem, checking pending state.
    /// </summary>
    public WorkItemViewModel Create(WorkItem item)
    {
        ArgumentNullException.ThrowIfNull(item);
        var isPending = _workItemStore.IsPending(item.Id);
        return new WorkItemViewModel(item, isPending);
    }

    /// <summary>
    /// Create a SprintViewModel from a Sprint, checking pending state.
    /// </summary>
    public SprintViewModel Create(Sprint sprint)
    {
        ArgumentNullException.ThrowIfNull(sprint);
        var isPending = _sprintStore.IsPending(sprint.Id);
        return new SprintViewModel(sprint, isPending);
    }

    /// <summary>
    /// Create ViewModels for multiple work items.
    /// </summary>
    public IReadOnlyList<WorkItemViewModel> CreateMany(IEnumerable<WorkItem> items)
    {
        ArgumentNullException.ThrowIfNull(items);
        return items.Select(Create).ToList();
    }

    /// <summary>
    /// Create ViewModels for multiple sprints.
    /// </summary>
    public IReadOnlyList<SprintViewModel> CreateMany(IEnumerable<Sprint> sprints)
    {
        ArgumentNullException.ThrowIfNull(sprints);
        return sprints.Select(Create).ToList();
    }

    /// <summary>
    /// Create a WorkItemViewModel with explicit pending state (for optimistic creates).
    /// </summary>
    public WorkItemViewModel CreateWithPendingState(WorkItem item, bool isPending)
    {
        ArgumentNullException.ThrowIfNull(item);
        return new WorkItemViewModel(item, isPending);
    }

    /// <summary>
    /// Create a SprintViewModel with explicit pending state (for optimistic creates).
    /// </summary>
    public SprintViewModel CreateWithPendingState(Sprint sprint, bool isPending)
    {
        ArgumentNullException.ThrowIfNull(sprint);
        return new SprintViewModel(sprint, isPending);
    }
}
