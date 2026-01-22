namespace ProjectManagement.Services.Desktop;

/// <summary>
/// Server state event from Tauri backend.
/// </summary>
public sealed record ServerStateEvent
{
    public required string State { get; init; }
    public int? Port { get; init; }
    public string? Error { get; init; }
    public DateTime Timestamp { get; init; } = DateTime.UtcNow;
}