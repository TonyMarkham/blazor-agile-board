using System.IO;
using System.Net.WebSockets;
using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Options;
using ProjectManagement.Core.Exceptions;

namespace ProjectManagement.Services.Resilience;

/// <summary>
/// Circuit breaker to prevent cascading failures.
/// Matches backend pm-ws circuit breaker behavior.
/// </summary>
public sealed class CircuitBreaker
{
    private readonly CircuitBreakerOptions _options;
    private readonly ILogger<CircuitBreaker> _logger;
    private readonly object _lock = new();

    private CircuitState _state = CircuitState.Closed;
    private int _failureCount;
    private int _successCount;
    private DateTime _lastFailureTime = DateTime.MinValue;
    private DateTime _openedAt = DateTime.MinValue;

    public CircuitState State
    {
        get
        {
            lock (_lock) return _state;
        }
    }

    public TimeSpan? RetryAfter
    {
        get
        {
            lock (_lock)
            {
                if (_state != CircuitState.Open)
                    return null;

                var elapsed = DateTime.UtcNow - _openedAt;
                var remaining = _options.OpenDuration - elapsed;
                return remaining > TimeSpan.Zero ? remaining : TimeSpan.Zero;
            }
        }
    }

    public CircuitBreaker(
        IOptions<CircuitBreakerOptions> options,
        ILogger<CircuitBreaker> logger)
    {
        _options = options.Value;
        _logger = logger;
    }

    /// <summary>
    /// Check if a request should be allowed through.
    /// </summary>
    public bool AllowRequest()
    {
        lock (_lock)
        {
            switch (_state)
            {
                case CircuitState.Closed:
                    return true;

                case CircuitState.Open:
                    // Check if we should transition to half-open
                    if (DateTime.UtcNow - _openedAt >= _options.OpenDuration)
                    {
                        _state = CircuitState.HalfOpen;
                        _successCount = 0;
                        _logger.LogInformation("Circuit breaker transitioning to HalfOpen");
                        return true;
                    }

                    return false;

                case CircuitState.HalfOpen:
                    return true;

                default:
                    return false;
            }
        }
    }

    /// <summary>
    /// Record a successful operation.
    /// </summary>
    public void RecordSuccess()
    {
        lock (_lock)
        {
            switch (_state)
            {
                case CircuitState.Closed:
                    _failureCount = 0;
                    break;

                case CircuitState.HalfOpen:
                    _successCount++;
                    if (_successCount >= _options.HalfOpenSuccessThreshold)
                    {
                        _state = CircuitState.Closed;
                        _failureCount = 0;
                        _logger.LogInformation(
                            "Circuit breaker closed after {Successes} successes",
                            _successCount);
                    }

                    break;
            }
        }
    }

    /// <summary>
    /// Record a failed operation.
    /// </summary>
    public void RecordFailure()
    {
        lock (_lock)
        {
            var now = DateTime.UtcNow;

            // Reset count if outside failure window
            if (now - _lastFailureTime > _options.FailureWindow)
            {
                _failureCount = 0;
            }

            _lastFailureTime = now;
            _failureCount++;

            switch (_state)
            {
                case CircuitState.Closed:
                    if (_failureCount >= _options.FailureThreshold)
                    {
                        _state = CircuitState.Open;
                        _openedAt = now;
                        _logger.LogWarning(
                            "Circuit breaker OPEN after {Failures} failures",
                            _failureCount);
                    }

                    break;

                case CircuitState.HalfOpen:
                    _state = CircuitState.Open;
                    _openedAt = now;
                    _logger.LogWarning(
                        "Circuit breaker reopened due to failure in HalfOpen state");
                    break;
            }
        }
    }

    /// <summary>
    /// Execute an operation with circuit breaker protection.
    /// </summary>
    public async Task<T> ExecuteAsync<T>(
        Func<CancellationToken, Task<T>> operation,
        CancellationToken ct = default)
    {
        if (!AllowRequest())
        {
            throw new CircuitOpenException(RetryAfter ?? _options.OpenDuration);
        }

        try
        {
            var result = await operation(ct);
            RecordSuccess();
            return result;
        }
        catch (Exception ex)
        {
            // Record failure for transient errors, but always re-throw
            if (ShouldRecordAsFailure(ex))
            {
                RecordFailure();
            }

            throw;
        }
    }

    private static bool ShouldRecordAsFailure(Exception ex)
    {
        // Don't count validation errors or cancellation as circuit failures
        return ex is not ValidationException
               && ex is not OperationCanceledException
               && ex is not VersionConflictException;
    }
}