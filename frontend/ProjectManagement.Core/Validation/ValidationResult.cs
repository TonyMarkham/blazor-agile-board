using ProjectManagement.Core.Exceptions;

namespace ProjectManagement.Core.Validation;

public sealed class ValidationResult
{
    private ValidationResult(IReadOnlyList<ValidationError> errors)
    {
        Errors = errors;
    }

    public bool IsValid => Errors.Count == 0;
    public IReadOnlyList<ValidationError> Errors { get; }

    public static ValidationResult Success()
    {
        return new ValidationResult([]);
    }

    public static ValidationResult Failure(params ValidationError[] errors)
    {
        return new ValidationResult(errors);
    }

    public static ValidationResult Failure(IEnumerable<ValidationError> errors)
    {
        return new ValidationResult(errors.ToList());
    }

    public void ThrowIfInvalid()
    {
        if (!IsValid)
            throw new ValidationException(Errors);
    }
}