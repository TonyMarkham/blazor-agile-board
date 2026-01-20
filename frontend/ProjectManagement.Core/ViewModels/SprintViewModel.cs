using ProjectManagement.Core.Models;

namespace ProjectManagement.Core.ViewModels;

/// <summary>
/// View model for Sprint. Combines immutable domain data with UI state.
/// Note: Sprint.StartDate and Sprint.EndDate are non-nullable (per Session 20).
/// </summary>
public sealed class SprintViewModel : IViewModel<Sprint>, IEquatable<SprintViewModel>
{
    public SprintViewModel(Sprint model, bool isPendingSync = false)
    {
        ArgumentNullException.ThrowIfNull(model);
        Model = model;
        IsPendingSync = isPendingSync;
    }

    public Sprint Model { get; }
    public bool IsPendingSync { get; }

    // === Identity ===
    public Guid Id => Model.Id;
    public Guid ProjectId => Model.ProjectId;

    // === Core Properties ===
    public string Name => Model.Name;
    public string? Goal => Model.Goal;
    public DateTime StartDate => Model.StartDate;
    public DateTime EndDate => Model.EndDate;
    public SprintStatus Status => Model.Status;

    // === Audit ===
    public DateTime? DeletedAt => Model.DeletedAt;

    // === Computed Properties ===
    public bool IsDeleted => Model.DeletedAt.HasValue;
    public bool IsPlanned => Model.Status == SprintStatus.Planned;
    public bool IsActive => Model.Status == SprintStatus.Active;
    public bool IsCompleted => Model.Status == SprintStatus.Completed;

    public string StatusDisplayName => Status switch
    {
        SprintStatus.Planned => "Planned",
        SprintStatus.Active => "Active",
        SprintStatus.Completed => "Completed",
        _ => Status.ToString()
    };

    /// <summary>
    /// Formatted date range for display (e.g., "Jan 15 - Jan 29").
    /// </summary>
    public string DateRangeDisplay => $"{StartDate:MMM d} - {EndDate:MMM d}";

    /// <summary>
    /// Full date range with year for clarity.
    /// </summary>
    public string DateRangeDisplayFull => $"{StartDate:MMM d, yyyy} - {EndDate:MMM d, yyyy}";

    /// <summary>
    /// Days remaining in the sprint (only meaningful when Active).
    /// Returns null if sprint is not active.
    /// </summary>
    public int? DaysRemaining
    {
        get
        {
            if (Status != SprintStatus.Active) return null;
            var remaining = (EndDate.Date - DateTime.UtcNow.Date).TotalDays;
            return Math.Max(0, (int)remaining);
        }
    }

    /// <summary>
    /// Total duration of the sprint in days.
    /// </summary>
    public int DurationDays => Math.Max(1, (int)(EndDate.Date - StartDate.Date).TotalDays);

    /// <summary>
    /// Progress percentage through the sprint (0-100).
    /// Based on elapsed time, not completed work.
    /// </summary>
    public double ProgressPercent
    {
        get
        {
            if (Status == SprintStatus.Completed) return 100;
            if (Status == SprintStatus.Planned) return 0;

            var total = (EndDate - StartDate).TotalDays;
            if (total <= 0) return 100;

            var elapsed = (DateTime.UtcNow - StartDate).TotalDays;
            return Math.Clamp(elapsed / total * 100, 0, 100);
        }
    }

    /// <summary>
    /// True if the sprint has passed its end date but is not yet completed.
    /// </summary>
    public bool IsOverdue => Status == SprintStatus.Active && DateTime.UtcNow.Date > EndDate.Date;

    // === Equality ===
    public bool Equals(SprintViewModel? other)
    {
        if (other is null) return false;
        if (ReferenceEquals(this, other)) return true;
        return Id == other.Id && Status == other.Status && IsPendingSync == other.IsPendingSync;
    }

    public override bool Equals(object? obj) => Equals(obj as SprintViewModel);

    public override int GetHashCode() => HashCode.Combine(Id, Status, IsPendingSync);

    public static bool operator ==(SprintViewModel? left, SprintViewModel? right) =>
        left?.Equals(right) ?? right is null;

    public static bool operator !=(SprintViewModel? left, SprintViewModel? right) =>
        !(left == right);
}
