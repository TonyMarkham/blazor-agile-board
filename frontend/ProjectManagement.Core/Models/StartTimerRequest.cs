namespace ProjectManagement.Core.Models;

/// <summary>
/// Request to start a timer on a work item.
/// If the user already has a running timer, it will be automatically stopped.
/// </summary>
public sealed record StartTimerRequest
{
    /// <summary>The work item to track time against.</summary>
    public Guid WorkItemId { get; init; }

    /// <summary>Optional description of what is being worked on.</summary>
    public string? Description { get; init; }
}