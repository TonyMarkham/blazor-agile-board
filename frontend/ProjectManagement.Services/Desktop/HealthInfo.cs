namespace ProjectManagement.Services.Desktop;

/// <summary>
/// Server health information.
/// </summary>
public sealed record HealthInfo
{
    public required string Status { get; init; }
    public long? LatencyMs { get; init; }
    public string? Version { get; init; }
}