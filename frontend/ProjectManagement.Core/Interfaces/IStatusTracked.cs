namespace ProjectManagement.Core.Interfaces;

/// <summary>
/// Entity that has a workflow status.
/// Status values map to SwimLanes for Kanban board display.
/// </summary>
public interface IStatusTracked
{
    /// <summary>Current workflow status (e.g., "backlog", "in_progress", "done").</summary>
    string Status { get; }
}