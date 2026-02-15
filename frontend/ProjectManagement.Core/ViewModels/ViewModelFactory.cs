using Microsoft.Extensions.Logging;
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
    private readonly ILogger<ViewModelFactory> _logger;

    public ViewModelFactory(
        IWorkItemStore workItemStore,
        ISprintStore sprintStore,
        IProjectStore projectStore,
        ILogger<ViewModelFactory> logger)
    {
        ArgumentNullException.ThrowIfNull(workItemStore);
        ArgumentNullException.ThrowIfNull(sprintStore);
        ArgumentNullException.ThrowIfNull(projectStore);
        ArgumentNullException.ThrowIfNull(logger);
        _workItemStore = workItemStore;
        _sprintStore = sprintStore;
        _projectStore = projectStore;
        _logger = logger;
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
    /// Create a WorkItemViewModel with progress computed via O(1) dictionary lookups.
    /// Uses server-provided DescendantIds instead of scanning all items.
    /// </summary>
    /// <remarks>
    /// <paramref name="itemLookup"/> should contain the full unfiltered item set
    /// for accurate progress counts. If items are filtered before building the
    /// lookup, descendants computed by the backend may be missing, and progress
    /// bars will only reflect items present in the lookup.
    /// </remarks>
    public WorkItemViewModel Create(WorkItem workItem, IReadOnlyDictionary<Guid, WorkItem> itemLookup)
    {
        ArgumentNullException.ThrowIfNull(workItem);
        ArgumentNullException.ThrowIfNull(itemLookup);

        var isPending = _workItemStore.IsPending(workItem.Id);
        ChildProgress? taskProgress = null;
        ChildProgress? storyProgress = null;

        if (workItem.ItemType == WorkItemType.Epic)
        {
            // Collect stories and tasks from pre-computed descendants
            var stories = new List<WorkItem>();
            var allTasks = new List<WorkItem>();
            var missingCount = 0;

            foreach (var descId in workItem.DescendantIds)
            {
                if (!itemLookup.TryGetValue(descId, out var desc))
                {
                    missingCount++;
                    continue;
                }

                if (desc.DeletedAt is not null) continue;

                if (desc.ItemType == WorkItemType.Story) stories.Add(desc);
                else if (desc.ItemType == WorkItemType.Task) allTasks.Add(desc);
            }

            if (missingCount > 0)
            {
                _logger.LogDebug(
                    "Epic {EpicId}: {MissingCount} descendants not in lookup (likely filtered)",
                    workItem.Id, missingCount);
            }

            storyProgress = ComputeProgress(stories);
            taskProgress = ComputeProgress(allTasks);
        }
        else if (workItem.ItemType == WorkItemType.Story)
        {
            // Collect child tasks from pre-computed descendants
            var tasks = new List<WorkItem>();
            var missingCount = 0;

            foreach (var descId in workItem.DescendantIds)
            {
                if (!itemLookup.TryGetValue(descId, out var desc))
                {
                    missingCount++;
                    continue;
                }

                if (desc.ItemType != WorkItemType.Task || desc.DeletedAt is not null)
                    continue;

                tasks.Add(desc);
            }

            if (missingCount > 0)
            {
                _logger.LogDebug(
                    "Story {StoryId}: {MissingCount} descendants not in lookup (likely filtered)",
                    workItem.Id, missingCount);
            }

            taskProgress = ComputeProgress(tasks);
        }

        return new WorkItemViewModel(workItem, isPending)
        {
            TaskProgress = taskProgress,
            StoryProgress = storyProgress,
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
    /// Create ViewModels for multiple work items.
    /// Uses O(N) dictionary lookup when DescendantIds are populated,
    /// falls back to legacy O(N squared) scan for backward compatibility.
    /// </summary>
    /// <remarks>
    /// For accurate progress counts with the optimized path, pass the full
    /// unfiltered item set. Filtering before this call means descendants
    /// computed by the backend may be missing from the dictionary.
    /// </remarks>
    public IReadOnlyList<WorkItemViewModel> CreateMany(IEnumerable<WorkItem> items)
    {
        ArgumentNullException.ThrowIfNull(items);
        var itemList = items.ToList();

        // Check if hierarchy data is available from the server
        var hasHierarchy = itemList.Any(i => i.DescendantIds.Count > 0);

        if (hasHierarchy)
        {
            // O(N) path: build lookup table once, use DescendantIds for progress
            var lookup = itemList.ToDictionary(i => i.Id);
            return itemList.Select(item => Create(item, lookup)).ToList();
        }

        // Legacy O(N squared) fallback for older servers
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
