namespace ProjectManagement.Services.Resilience;

/// <summary>
///     Configuration for retry policy behavior.
/// </summary>
public sealed class RetryPolicyOptions
{
    public const int DefaultMaxAttempts = 3;
    public const int DefaultInitialDelayMilliseconds = 100;
    public const int DefaultMaxDelaySeconds = 5; // Aligned with pm-config
    public const double DefaultBackoffMultiplier = 2.0;

    public int MaxAttempts { get; set; } = DefaultMaxAttempts;
    public TimeSpan InitialDelay { get; set; } = TimeSpan.FromMilliseconds(DefaultInitialDelayMilliseconds);
    public TimeSpan MaxDelay { get; set; } = TimeSpan.FromSeconds(DefaultMaxDelaySeconds);
    public double BackoffMultiplier { get; set; } = DefaultBackoffMultiplier;
}