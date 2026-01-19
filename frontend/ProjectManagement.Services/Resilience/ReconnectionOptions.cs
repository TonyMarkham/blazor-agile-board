namespace ProjectManagement.Services.Resilience;

/// <summary>
///     Configuration for automatic reconnection behavior.
///     Note: Reconnection is a client-only concern (not present in pm-config).
///     These values are tuned for desktop/WebSocket UX, not server-side retry policy.
/// </summary>
public sealed class ReconnectionOptions
{
    public const int DefaultMaxAttempts = 10;
    public const int DefaultInitialDelaySeconds = 1;
    public const int DefaultMaxDelaySeconds = 30;

    public int MaxAttempts { get; set; } = DefaultMaxAttempts;
    public TimeSpan InitialDelay { get; set; } = TimeSpan.FromSeconds(DefaultInitialDelaySeconds);
    public TimeSpan MaxDelay { get; set; } = TimeSpan.FromSeconds(DefaultMaxDelaySeconds);
}