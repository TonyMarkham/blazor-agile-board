namespace ProjectManagement.Core.Models;

/// <summary>
/// Types of dependency relationships.
/// </summary>
public enum DependencyType
{
    /// <summary>
    /// Blocking relationship. The blocking item must be completed
    /// before the blocked item can start. Cycles are not allowed.
    /// </summary>
    Blocks = 1,

    /// <summary>
    /// Informational relationship. Indicates items are related
    /// but does not impose any ordering constraints.
    /// Bidirectional relationships are allowed.
    /// </summary>
    RelatesTo = 2
}