namespace ProjectManagement.Core.Models;

public enum WorkItemType
{
    Project = 1,
    Epic = 2,
    Story = 3,
    Task = 4
}

public static class WorkItemTypeExtensions
{
    public static string ToDisplayString(this WorkItemType type)
    {
        return type switch
        {
            WorkItemType.Project => "Project",
            WorkItemType.Epic => "Epic",
            WorkItemType.Story => "Story",
            WorkItemType.Task => "Task",
            _ => throw new ArgumentOutOfRangeException(nameof(type))
        };
    }

    public static bool CanHaveParent(this WorkItemType type)
    {
        return type != WorkItemType.Project;
    }

    public static IReadOnlyList<WorkItemType> AllowedChildTypes(this WorkItemType type)
    {
        return type switch
        {
            WorkItemType.Project => [WorkItemType.Epic, WorkItemType.Story, WorkItemType.Task],
            WorkItemType.Epic => [WorkItemType.Story, WorkItemType.Task],
            WorkItemType.Story => [WorkItemType.Task],
            WorkItemType.Task => [],
            _ => []
        };
    }
}