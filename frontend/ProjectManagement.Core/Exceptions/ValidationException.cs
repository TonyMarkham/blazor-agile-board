namespace ProjectManagement.Core.Exceptions;

/// <summary>
/// Client-side validation failure.
/// </summary>
public sealed class ValidationException : ProjectManagementException
{
    public override string ErrorCode => "VALIDATION_ERROR";
    public override string UserMessage => Errors.FirstOrDefault()?.Message ?? "Invalid input.";

    public IReadOnlyList<ValidationError> Errors { get; }

    public ValidationException(IEnumerable<ValidationError> errors)
        : base($"Validation failed: {string.Join(", ", errors.Select(e => e.Message))}")
    {
        Errors = errors.ToList().AsReadOnly();
    }

    public ValidationException(string field, string message)
        : this(new[] { new ValidationError(field, message) })
    {
    }
}

/// <summary>
/// Single validation error (field + message).
/// </summary>
public sealed record ValidationError(string Field, string Message);