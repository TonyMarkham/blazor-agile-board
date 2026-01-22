namespace ProjectManagement.Core.Models;

/// <summary>
/// Request to create a new project.
/// </summary>
public sealed record CreateProjectRequest
{
    /// <summary>
    /// Project title (1-200 characters).
    /// </summary>
    public required string Title { get; init; }

    /// <summary>
    /// Optional description (max 10000 characters).
    /// </summary>
    public string? Description { get; init; }

    /// <summary>
    /// Unique project key (2-20 alphanumeric characters).
    /// Will be stored uppercase.
    /// </summary>
    public required string Key { get; init; }
}