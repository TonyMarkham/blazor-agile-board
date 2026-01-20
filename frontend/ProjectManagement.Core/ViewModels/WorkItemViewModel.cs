using ProjectManagement.Core.Models;

namespace ProjectManagement.Core.ViewModels;

/// <summary>
/// View model for WorkItem. Combines immutable domain data with UI state.
/// Exposes commonly-accessed properties for convenient Razor binding.
/// </summary>
public sealed class WorkItemViewModel : IViewModel<WorkItem>, IEquatable<WorkItemViewModel>
{
    public WorkItemViewModel(WorkItem model, bool isPendingSync = false)
    {
        ArgumentNullException.ThrowIfNull(model);
        Model = model;
        IsPendingSync = isPendingSync;
    }

    public WorkItem Model { get; }
    public bool IsPendingSync { get; }

    // === Identity ===
    public Guid Id => Model.Id;
    public int Version => Model.Version;

    // === Core Properties ===
    public WorkItemType ItemType => Model.ItemType;
    public string Title => Model.Title;
    public string? Description => Model.Description;
    public string Status => Model.Status;
    public string Priority => Model.Priority;
    public int? StoryPoints => Model.StoryPoints;
    public int Position => Model.Position;

    // === Relationships ===
    public Guid ProjectId => Model.ProjectId;
    public Guid? ParentId => Model.ParentId;
    public Guid? SprintId => Model.SprintId;
    public Guid? AssigneeId => Model.AssigneeId;

    // === Audit ===
    public Guid CreatedBy => Model.CreatedBy;
    public Guid UpdatedBy => Model.UpdatedBy;
    public DateTime CreatedAt => Model.CreatedAt;
    public DateTime UpdatedAt => Model.UpdatedAt;
    public DateTime? DeletedAt => Model.DeletedAt;

    // === Computed Properties ===
    public bool IsDeleted => Model.DeletedAt.HasValue;
    public bool IsCompleted => Model.Status == "done";

    public string StatusDisplayName => Status switch
    {
        "backlog" => "Backlog",
        "todo" => "To Do",
        "in_progress" => "In Progress",
        "review" => "Review",
        "done" => "Done",
        _ => Status
    };

    public string PriorityDisplayName => Priority switch
    {
        "critical" => "Critical",
        "high" => "High",
        "medium" => "Medium",
        "low" => "Low",
        _ => Priority
    };

    public string ItemTypeDisplayName => ItemType switch
    {
        WorkItemType.Project => "Project",
        WorkItemType.Epic => "Epic",
        WorkItemType.Story => "Story",
        WorkItemType.Task => "Task",
        _ => ItemType.ToString()
    };

    /// <summary>
    /// Priority sort order (lower = more urgent).
    /// </summary>
    public int PrioritySortOrder => Priority switch
    {
        "critical" => 0,
        "high" => 1,
        "medium" => 2,
        "low" => 3,
        _ => 4
    };

    // === Equality ===
    public bool Equals(WorkItemViewModel? other)
    {
        if (other is null) return false;
        if (ReferenceEquals(this, other)) return true;
        return Id == other.Id && Version == other.Version && IsPendingSync == other.IsPendingSync;
    }

    public override bool Equals(object? obj) => Equals(obj as WorkItemViewModel);

    public override int GetHashCode() => HashCode.Combine(Id, Version, IsPendingSync);

    public static bool operator ==(WorkItemViewModel? left, WorkItemViewModel? right) =>
        left?.Equals(right) ?? right is null;

    public static bool operator !=(WorkItemViewModel? left, WorkItemViewModel? right) =>
        !(left == right);
}
