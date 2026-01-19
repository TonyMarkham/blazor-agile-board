namespace ProjectManagement.Services.Resilience;

/// <summary>
/// States for the circuit breaker pattern.
/// </summary>
public enum CircuitState
{
    /// <summary>Normal operation - requests allowed.</summary>
    Closed,

    /// <summary>Circuit tripped - requests blocked.</summary>
    Open,

    /// <summary>Testing recovery - limited requests allowed.</summary>
    HalfOpen
}