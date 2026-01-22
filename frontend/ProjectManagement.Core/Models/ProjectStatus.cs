namespace ProjectManagement.Core.Models;

/// <summary>
/// Project lifecycle status.
/// </summary>
public enum ProjectStatus
{
    /// <summary>
    /// Project is active and accepting work.
    /// </summary>
    Active = 0,

    /// <summary>
    /// Project is archived (read-only, hidden from default views).
    /// </summary>
    Archived = 1
}