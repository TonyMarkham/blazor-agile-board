namespace ProjectManagement.Services.State;

/// <summary>
///     Tracks a pending optimistic update for rollback capability.
/// </summary>
internal sealed record OptimisticUpdate<T>(
    Guid EntityId,
    T? OriginalValue,
    T OptimisticValue)
{
    public DateTime CreatedAt { get; } = DateTime.UtcNow;

    public bool IsCreate => OriginalValue is null;
}