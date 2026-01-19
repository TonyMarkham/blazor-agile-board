namespace ProjectManagement.Core.Exceptions;

/// <summary>
/// Server rejected the request (validation failure, authorization, etc.).
/// </summary>
public sealed class ServerRejectedException : ProjectManagementException
{
    public override string ErrorCode { get; }
    public override string UserMessage { get; }

    public string? Field { get; init; }

    public ServerRejectedException(string errorCode, string message, string? field = null)
        : base(message)
    {
        ErrorCode = errorCode;
        UserMessage = SanitizeMessage(message);
        Field = field;
    }

    private static string SanitizeMessage(string message)
    {
        // Never expose internal details
        if (message.Contains("SQLITE", StringComparison.OrdinalIgnoreCase) ||
            message.Contains("sqlx", StringComparison.OrdinalIgnoreCase) ||
            message.Contains("stack trace", StringComparison.OrdinalIgnoreCase))
        {
            return "An internal error occurred. Please try again later.";
        }

        return message.Length > 200 ? message[..200] : message;
    }
}