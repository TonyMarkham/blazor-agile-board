namespace ProjectManagement.Core.Interfaces;

/// <summary>
/// Entity that belongs to a specific work item.
/// Enables work-item-level filtering and cascade operations.
/// </summary>
public interface IWorkItemScoped
{
    /// <summary>The work item this entity belongs to.</summary>
    Guid WorkItemId { get; }
}