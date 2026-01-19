namespace ProjectManagement.Core.Exceptions;

/// <summary>
///     Request timed out waiting for server response.
/// </summary>
public sealed class RequestTimeoutException : ProjectManagementException
{
    public RequestTimeoutException(string messageId, TimeSpan timeout)
        : base($"Request {messageId} timed out after {timeout.TotalSeconds}s")
    {
        MessageId = messageId;
        Timeout = timeout;
    }

    public override string ErrorCode => "REQUEST_TIMEOUT";
    public override string UserMessage => "The request timed out. Please try again.";

    public string? MessageId { get; init; }
    public TimeSpan Timeout { get; init; }
}