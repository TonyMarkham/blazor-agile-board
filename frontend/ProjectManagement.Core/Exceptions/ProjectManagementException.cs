namespace ProjectManagement.Core.Exceptions;

/// <summary>
/// Base exception for all Project Management errors.
/// Contains correlation ID for distributed tracing.
/// </summary>
public abstract class ProjectManagementException : Exception
{
    /// <summary>
    /// Correlation ID for tracing this error across systems.
    /// </summary>
    public string? CorrelationId { get; init; }

    /// <summary>
    /// Error code for programmatic handling.
    /// </summary>
    public abstract string ErrorCode { get; }

    /// <summary>
    /// User-friendly message (safe to display in UI).
    /// </summary>
    public virtual string UserMessage => "An unexpected error occurred. Please try again.";

    protected ProjectManagementException(string message) : base(message)
    {
    }

    protected ProjectManagementException(string message, Exception inner) : base(message, inner)
    {
    }
}