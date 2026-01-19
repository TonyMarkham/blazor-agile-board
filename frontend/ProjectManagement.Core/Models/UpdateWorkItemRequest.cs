namespace ProjectManagement.Core.Models;

public sealed record UpdateWorkItemRequest
{
    public required Guid WorkItemId { get; init; }
    public int? ExpectedVersion { get; init; }
    public string? Title { get; init; }
    public string? Description { get; init; }
    public string? Status { get; init; }
    public string? Priority { get; init; }
    public Guid? AssigneeId { get; init; }
    public int? StoryPoints { get; init; }
    public Guid? SprintId { get; init; }
    public int? Position { get; init; }
}