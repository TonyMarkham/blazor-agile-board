namespace ProjectManagement.Core.Models;

using ProjectManagement.Core.Interfaces;

/// <summary>
/// A time-boxed iteration for completing work items.
/// </summary>
public sealed record Sprint : IAuditable, IProjectScoped
{
    public Guid Id { get; init; }
    public Guid ProjectId { get; init; }
    public string Name { get; init; } = string.Empty;
    public string? Goal { get; init; }
    public DateTime StartDate { get; init; }
    public DateTime EndDate { get; init; }
    public SprintStatus Status { get; init; } = SprintStatus.Planned;
    public DateTime CreatedAt { get; init; }
    public DateTime UpdatedAt { get; init; }
    public Guid CreatedBy { get; init; }
    public Guid UpdatedBy { get; init; }
    public DateTime? DeletedAt { get; init; }
}