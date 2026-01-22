namespace ProjectManagement.Core.Models;

/// <summary>
/// Type of work item in the hierarchy.
/// Note: Project is now a separate entity, not a WorkItemType.
/// </summary>
public enum WorkItemType
{
    // Project = 1, -- REMOVED: Projects are now separate entities

    /// <summary>
    /// Large body of work, contains Stories and Tasks.
    /// </summary>
    Epic = 2,

    /// <summary>
    /// User-facing feature or requirement.
    /// </summary>
    Story = 3,

    /// <summary>
    /// Atomic unit of work.
    /// </summary>
    Task = 4
}

/// <summary>
/// Extension methods for WorkItemType.
/// </summary>
public static class WorkItemTypeExtensions
{
    /// <summary>
    /// Get display name for the type.
    /// </summary>
    public static string ToDisplayString(this WorkItemType type)
    {
        return type switch
        {
            WorkItemType.Epic => "Epic",
            WorkItemType.Story => "Story",
            WorkItemType.Task => "Task",
            _ => throw new ArgumentOutOfRangeException(nameof(type))
        };
    }

    /// <summary>
    /// Check if this type can have a parent work item.
    /// All work item types can have parents now (Projects are separate).
    /// </summary>
    public static bool CanHaveParent(this WorkItemType type) => true;

    /// <summary>
    /// Get allowed child types for a work item.
    /// </summary>
    public static IReadOnlyList<WorkItemType> AllowedChildTypes(this WorkItemType type)
    {
        return type switch
        {
            WorkItemType.Epic => [WorkItemType.Story, WorkItemType.Task],
            WorkItemType.Story => [WorkItemType.Task],
            WorkItemType.Task => [],
            _ => []
        };
    }
}