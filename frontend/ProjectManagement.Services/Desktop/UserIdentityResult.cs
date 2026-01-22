using ProjectManagement.Core.Models;

namespace ProjectManagement.Services.Desktop;

/// <summary>
/// Result wrapper for Tauri command response.
/// </summary>
internal sealed record UserIdentityResult
{
    public UserIdentity? User { get; init; }
    public string? Error { get; init; }
}