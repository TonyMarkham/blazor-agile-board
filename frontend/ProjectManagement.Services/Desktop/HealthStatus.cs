using System.Text.Json.Serialization;

namespace ProjectManagement.Services.Desktop;

/// <summary>
/// Health status from server.
/// </summary>
public sealed record HealthStatus
{
    /// <summary>
    /// Health status: healthy, degraded, unhealthy.
    /// </summary>
    [JsonPropertyName("status")]
    public string Status { get; init; } = "unknown";
}