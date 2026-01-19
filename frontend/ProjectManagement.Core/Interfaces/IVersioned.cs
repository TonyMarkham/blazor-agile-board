namespace ProjectManagement.Core.Interfaces;

/// <summary>
/// Entity that supports optimistic concurrency control.
/// </summary>
public interface IVersioned
{
    /// <summary>Version number for optimistic locking.</summary>
    int Version { get; }
}