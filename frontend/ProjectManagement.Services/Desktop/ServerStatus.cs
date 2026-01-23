using System.Text.Json.Serialization;

namespace ProjectManagement.Services.Desktop;

/// <summary>
/// Server status returned from Tauri IPC (get_server_status command).
/// </summary>
public sealed record ServerStatus
{
    /// <summary>
    /// Server state: Starting, Running, Restarting, Failed, Stopped.
    /// </summary>
    [JsonPropertyName("state")]
    public string State { get; init; } = "Unknown";

    /// <summary>
    /// WebSocket URL when server is running.
    /// </summary>
    [JsonPropertyName("websocket_url")]
    public string? WebsocketUrl { get; init; }

    /// <summary>
    /// Health check result.
    /// </summary>
    [JsonPropertyName("health")]
    public HealthStatus? Health { get; init; }

    /// <summary>
    /// Error message if state is Failed.
    /// </summary>
    [JsonPropertyName("error")]
    public string? Error { get; init; }
    
    /// <summary>
    /// Whether the server is healthy and ready for connections.
    /// </summary>
    [JsonPropertyName("is_healthy")]
    public bool IsHealthy { get; init; }

    /// <summary>
    /// Port number when server is running.
    /// </summary>
    [JsonPropertyName("port")]
    public int? Port { get; init; }
    
    /// <summary>
    /// Server process ID (for debugging orphan processes).
    /// </summary>
    [JsonPropertyName("pid")]
    public uint? Pid { get; init; }
}