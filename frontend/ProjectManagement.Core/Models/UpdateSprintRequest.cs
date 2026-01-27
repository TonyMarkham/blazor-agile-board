namespace ProjectManagement.Core.Models;

/// <summary>
/// Request to update an existing sprint.
/// Includes ExpectedVersion for optimistic locking.
/// </summary>
public sealed record UpdateSprintRequest
{
    /// <summary>
    /// ID of the sprint to update.
    /// </summary>
    public required Guid SprintId { get; init; }

    /// <summary>
    /// Expected version for optimistic locking.
    /// If the server's version doesn't match, the update is rejected.
    /// </summary>
    public required int ExpectedVersion { get; init; }

    /// <summary>
    /// New name for the sprint (optional).
    /// </summary>
    public string? Name { get; init; }

    /// <summary>
    /// New goal for the sprint (optional).
    /// </summary>
    public string? Goal { get; init; }

    /// <summary>
    /// New start date (optional).
    /// </summary>
    public DateTime? StartDate { get; init; }

    /// <summary>
    /// New end date (optional).
    /// </summary>
    public DateTime? EndDate { get; init; }

    /// <summary>
    /// New status (optional).
    /// Status transitions are validated by the server.
    /// </summary>
    public SprintStatus? Status { get; init; }
}