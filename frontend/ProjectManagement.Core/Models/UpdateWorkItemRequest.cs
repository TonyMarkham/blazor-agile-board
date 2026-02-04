namespace ProjectManagement.Core.Models;

public sealed record UpdateWorkItemRequest
{
    public required Guid WorkItemId { get; init; }
    public required int ExpectedVersion { get; init; }
    public string? Title { get; init; }
    public string? Description { get; init; }
    public string? Status { get; init; }
    public string? Priority { get; init; }
    public Guid? AssigneeId { get; init; }
    public int? StoryPoints { get; init; }
    public Guid? SprintId { get; init; }
    public int? Position { get; init; }
    
    /// <summary>
    /// Parent work item ID. Set to Guid.Empty to clear the parent.
    /// Leave null to keep the current parent unchanged.
    /// </summary>
    public Guid? ParentId { get; init; }

    /// <summary>
    /// Indicates whether ParentId should be updated (including clearing).
    /// Required because null ParentId could mean "no change" or "clear parent".
    /// </summary>
    public bool UpdateParent { get; init; }
}