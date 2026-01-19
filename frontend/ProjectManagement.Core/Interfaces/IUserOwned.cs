namespace ProjectManagement.Core.Interfaces;

/// <summary>
/// Entity that is owned by a specific user (not assignable, but belongs to).
/// Different from IUserAssignable - this is ownership, not assignment.
/// </summary>
public interface IUserOwned
{
    /// <summary>The user who owns this entity.</summary>
    Guid UserId { get; }
}