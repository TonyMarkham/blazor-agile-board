using ProjectManagement.Core.Models;

namespace ProjectManagement.Core.Exceptions;

/// <summary>
///     Connection-related failures (WebSocket connection issues).
/// </summary>
public sealed class ConnectionException : ProjectManagementException
{
    public ConnectionException(string message) : base(message)
    {
    }

    public ConnectionException(string message, Exception inner) : base(message, inner)
    {
    }

    public override string ErrorCode => "CONNECTION_FAILED";
    public override string UserMessage => "Unable to connect to server. Please check your connection.";

    public ConnectionState LastKnownState { get; init; }
    public TimeSpan? RetryAfter { get; init; }
}