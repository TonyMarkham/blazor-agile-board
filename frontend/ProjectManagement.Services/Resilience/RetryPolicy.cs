using System.IO;
using System.Net.WebSockets;
using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Options;
using ProjectManagement.Core.Exceptions;

namespace ProjectManagement.Services.Resilience;

/// <summary>
/// Retry policy with exponential backoff and jitter.
/// </summary>
public sealed class RetryPolicy
{
    private readonly RetryPolicyOptions _options;
    private readonly ILogger<RetryPolicy> _logger;
    // Note: Random.Shared is thread-safe on .NET 6+ (our target)

    public RetryPolicy(
        IOptions<RetryPolicyOptions> options,
        ILogger<RetryPolicy> logger)
    {
        _options = options.Value;
        _logger = logger;
    }

    /// <summary>
    /// Execute an operation with retry logic.
    /// </summary>
    public async Task<T> ExecuteAsync<T>(
        Func<CancellationToken, Task<T>> operation,
        CancellationToken ct = default)
    {
        var attempt = 0;
        var delay = _options.InitialDelay;

        while (true)
        {
            attempt++;

            try
            {
                return await operation(ct);
            }
            catch (Exception ex) when (ShouldRetry(ex, attempt))
            {
                var jitteredDelay = AddJitter(delay);

                _logger.LogWarning(
                    ex,
                    "Attempt {Attempt}/{MaxAttempts} failed. Retrying in {Delay}ms",
                    attempt,
                    _options.MaxAttempts,
                    jitteredDelay.TotalMilliseconds);

                await Task.Delay(jitteredDelay, ct);

                delay = TimeSpan.FromMilliseconds(
                    Math.Min(
                        delay.TotalMilliseconds * _options.BackoffMultiplier,
                        _options.MaxDelay.TotalMilliseconds));
            }
        }
    }

    private bool ShouldRetry(Exception ex, int attempt)
    {
        if (attempt >= _options.MaxAttempts)
            return false;

        // Don't retry non-transient errors
        return ex is ConnectionException
            or RequestTimeoutException
            or IOException
            or WebSocketException;
    }

    private static TimeSpan AddJitter(TimeSpan delay)
    {
        // Add up to 25% jitter to prevent thundering herd
        var jitterFactor = 1.0 + (Random.Shared.NextDouble() * 0.25);
        return TimeSpan.FromMilliseconds(delay.TotalMilliseconds * jitterFactor);
    }
}