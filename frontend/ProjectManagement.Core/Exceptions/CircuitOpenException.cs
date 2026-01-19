namespace ProjectManagement.Core.Exceptions;

/// <summary>
/// Circuit breaker is open - too many failures, service temporarily unavailable.
/// </summary>
public sealed class CircuitOpenException : ProjectManagementException
{
    public override string ErrorCode => "CIRCUIT_OPEN";
    public override string UserMessage => "Service temporarily unavailable. Please wait a moment.";

    public TimeSpan RetryAfter { get; init; }

    public CircuitOpenException(TimeSpan retryAfter)
        : base($"Circuit breaker open. Retry after {retryAfter.TotalSeconds}s")
    {
        RetryAfter = retryAfter;
    }
}