namespace ProjectManagement.Core.Models;

/// <summary>
/// Request to update an existing project.
/// Only non-null fields will be updated.
/// </summary>
public sealed record UpdateProjectRequest
{
    /// <summary>
    /// ID of the project to update.
    /// </summary>
    public required Guid ProjectId { get; init; }

    /// <summary>
    /// Expected version for optimistic concurrency.
    /// </summary>
    public required int ExpectedVersion { get; init; }

    /// <summary>
    /// New title (null to keep current).
    /// </summary>
    public string? Title { get; init; }

    /// <summary>
    /// New description (null to keep current).
    /// </summary>
    public string? Description { get; init; }

    /// <summary>
    /// New status (null to keep current).
    /// </summary>
    public ProjectStatus? Status { get; init; }
}