namespace ProjectManagement.Core.Models;

public sealed record CreateWorkItemRequest
{
    public required Guid ProjectId { get; init; }
    public required WorkItemType ItemType { get; init; }
    public required string Title { get; init; }
    public string? Description { get; init; }
    public Guid? ParentId { get; init; }
    public string Status { get; init; } = "backlog";
    public string Priority { get; init; } = "medium";
    public Guid? AssigneeId { get; init; }
    public int? StoryPoints { get; init; }
    public Guid? SprintId { get; init; }
}