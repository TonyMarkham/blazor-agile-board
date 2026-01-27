using ProjectManagement.Core.Interfaces;

namespace ProjectManagement.Core.Models;

/// <summary>
///     A time-boxed iteration for completing work items.
/// </summary>
public sealed record Sprint : IAuditable, IProjectScoped
{
    public string Name { get; init; } = string.Empty;
    public string? Goal { get; init; }
    public DateTime StartDate { get; init; }
    public DateTime EndDate { get; init; }
    public SprintStatus Status { get; init; } = SprintStatus.Planned;
    public int Version { get; init; } = 1;
    public Guid Id { get; init; }
    public DateTime CreatedAt { get; init; }
    public DateTime UpdatedAt { get; init; }
    public Guid CreatedBy { get; init; }
    public Guid UpdatedBy { get; init; }
    public DateTime? DeletedAt { get; init; }
    public Guid ProjectId { get; init; }
}