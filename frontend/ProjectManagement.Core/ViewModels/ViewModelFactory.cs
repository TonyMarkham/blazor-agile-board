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
    private readonly IProjectStore _projectStore;

    public ViewModelFactory(IWorkItemStore workItemStore, ISprintStore sprintStore, IProjectStore projectStore)
    {
        ArgumentNullException.ThrowIfNull(workItemStore);
        ArgumentNullException.ThrowIfNull(sprintStore);
        ArgumentNullException.ThrowIfNull(projectStore);
        _workItemStore = workItemStore;
        _sprintStore = sprintStore;
        _projectStore = projectStore;
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
    /// Create a WorkItemViewModel with progress computed from all project items.
    /// </summary>
    public WorkItemViewModel Create(WorkItem workItem, IEnumerable<WorkItem> allProjectItems)
    {
        ArgumentNullException.ThrowIfNull(workItem);
        ArgumentNullException.ThrowIfNull(allProjectItems);

        var isPending = _workItemStore.IsPending(workItem.Id);
        ChildProgress? taskProgress = null;
        ChildProgress? storyProgress = null;

        var allItems = allProjectItems.ToList();
        var children = allItems
            .Where(w => w.ParentId == workItem.Id && w.DeletedAt is null)
            .ToList();

        if (workItem.ItemType == WorkItemType.Epic)
        {
            // Epic: track both stories and tasks
            var stories = children.Where(c => c.ItemType == WorkItemType.Story).ToList();
            var directTasks = children.Where(c => c.ItemType == WorkItemType.Task).ToList();

            // Also count tasks under stories (grandchildren)
            var storyIds = stories.Select(s => s.Id).ToHashSet();
            var grandchildTasks = allItems
                .Where(w => w.ParentId.HasValue &&
                            storyIds.Contains(w.ParentId.Value) &&
                            w.ItemType == WorkItemType.Task &&
                            w.DeletedAt is null)
                .ToList();

            var allTasks = directTasks.Concat(grandchildTasks).ToList();

            storyProgress = ComputeProgress(stories);
            taskProgress = ComputeProgress(allTasks);
        }
        else if (workItem.ItemType == WorkItemType.Story)
        {
            // Story: track child tasks only
            var tasks = children.Where(c => c.ItemType == WorkItemType.Task).ToList();
            taskProgress = ComputeProgress(tasks);
        }

        return new WorkItemViewModel(workItem, isPending)
        {
            TaskProgress = taskProgress,
            StoryProgress = storyProgress
        };
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
    /// Create ViewModels for multiple work items, computing progress for each.
    /// </summary>
    public IReadOnlyList<WorkItemViewModel> CreateMany(IEnumerable<WorkItem> items)
    {
        ArgumentNullException.ThrowIfNull(items);
        var itemList = items.ToList();
        return itemList.Select(item => Create(item, itemList)).ToList();
    }
    
    /// <summary>
    /// Compute progress from a list of items.
    /// </summary>
    private static ChildProgress ComputeProgress(IReadOnlyList<WorkItem> items)
    {
        if (items.Count == 0) return ChildProgress.Empty;

        var byStatus = items
            .GroupBy(w => w.Status)
            .ToDictionary(g => g.Key, g => g.Count());

        var completed = byStatus.GetValueOrDefault("done", 0);

        return new ChildProgress
        {
            ByStatus = byStatus,
            Total = items.Count,
            Completed = completed
        };
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
    
    /// <summary>
    /// Create a ProjectViewModel from a Project, checking pending state.
    /// </summary>
    public ProjectViewModel Create(Project project)
    {
        ArgumentNullException.ThrowIfNull(project);
        var isPending = _projectStore.IsPending(project.Id);
        return new ProjectViewModel(project, isPending);
    }

    /// <summary>
    /// Create ViewModels for multiple projects.
    /// </summary>
    public IReadOnlyList<ProjectViewModel> CreateMany(IEnumerable<Project> projects)
    {
        ArgumentNullException.ThrowIfNull(projects);
        return projects.Select(Create).ToList();
    }

    /// <summary>
    /// Create a ProjectViewModel with explicit pending state (for optimistic creates).
    /// </summary>
    public ProjectViewModel CreateWithPendingState(Project project, bool isPending)
    {
        ArgumentNullException.ThrowIfNull(project);
        return new ProjectViewModel(project, isPending);
    }
}
