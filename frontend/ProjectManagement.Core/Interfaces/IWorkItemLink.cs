namespace ProjectManagement.Core.Interfaces;

/// <summary>
/// Entity that links two work items together (e.g., dependencies).
/// </summary>
public interface IWorkItemLink
{
    /// <summary>The source/blocking work item.</summary>
    Guid BlockingItemId { get; }

    /// <summary>The target/blocked work item.</summary>
    Guid BlockedItemId { get; }
}