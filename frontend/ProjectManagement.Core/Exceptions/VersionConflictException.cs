namespace ProjectManagement.Core.Exceptions;

/// <summary>
/// Optimistic concurrency conflict - entity was modified by another user.
/// </summary>
public sealed class VersionConflictException : ProjectManagementException
{
    public override string ErrorCode => "VERSION_CONFLICT";
    public override string UserMessage => "This item was modified by another user. Please refresh and try again.";

    public Guid EntityId { get; init; }
    public int ExpectedVersion { get; init; }
    public int ActualVersion { get; init; }

    public VersionConflictException(Guid entityId, int expected, int actual)
        : base($"Version conflict for {entityId}: expected {expected}, got {actual}")
    {
        EntityId = entityId;
        ExpectedVersion = expected;
        ActualVersion = actual;
    }
}