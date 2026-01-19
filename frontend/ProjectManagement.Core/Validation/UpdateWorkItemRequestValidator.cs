using ProjectManagement.Core.Exceptions;
using ProjectManagement.Core.Models;

namespace ProjectManagement.Core.Validation;

public sealed class UpdateWorkItemRequestValidator : IValidator<UpdateWorkItemRequest>
{
    private const int MaxTitleLength = 200;
    private const int MaxDescriptionLength = 10000;

    public ValidationResult Validate(UpdateWorkItemRequest request)
    {
        var errors = new List<ValidationError>();

        if (request.WorkItemId == Guid.Empty)
            errors.Add(new ValidationError("workItemId", "Work Item ID is required"));

        if (request.Title != null)
        {
            if (string.IsNullOrWhiteSpace(request.Title))
                errors.Add(new ValidationError("title", "Title cannot be empty"));
            else if (request.Title.Length > MaxTitleLength)
                errors.Add(new ValidationError("title", $"Title must be {MaxTitleLength} characters or less"));
        }

        if (request.Description?.Length > MaxDescriptionLength)
            errors.Add(new ValidationError("description",
                $"Description must be {MaxDescriptionLength} characters or less"));

        return errors.Count == 0 ? ValidationResult.Success() : ValidationResult.Failure(errors);
    }
}