using System.Text.Json.Serialization;

namespace ProjectManagement.Services.Desktop;

/// <summary>
/// Configuration provided by the Tauri desktop host.
/// Read from window.PM_CONFIG set by index.html.
/// </summary>
public sealed record DesktopConfig
{
    /// <summary>
    /// True if running in Tauri desktop mode.
    /// </summary>
    [JsonPropertyName("isDesktop")]
    public bool IsDesktop { get; init; }

    /// <summary>
    /// WebSocket server URL (set after server startup).
    /// </summary>
    [JsonPropertyName("serverUrl")]
    public string? ServerUrl { get; init; }
}