using ProjectManagement.Core.Exceptions;
using ProjectManagement.Core.Models;

namespace ProjectManagement.Core.Validation;

public sealed class CreateWorkItemRequestValidator : IValidator<CreateWorkItemRequest>
{
    private const int MaxTitleLength = 200;
    private const int MaxDescriptionLength = 10000;

    public ValidationResult Validate(CreateWorkItemRequest request)
    {
        var errors = new List<ValidationError>();

        if (string.IsNullOrWhiteSpace(request.Title))
            errors.Add(new ValidationError("title", "Title is required"));
        else if (request.Title.Length > MaxTitleLength)
            errors.Add(new ValidationError("title", $"Title must be {MaxTitleLength} characters or less"));

        if (request.ProjectId == Guid.Empty)
            errors.Add(new ValidationError("projectId", "Project ID is required"));

        if (request.Description?.Length > MaxDescriptionLength)
            errors.Add(new ValidationError("description",
                $"Description must be {MaxDescriptionLength} characters or less"));

        return errors.Count == 0 ? ValidationResult.Success() : ValidationResult.Failure(errors);
    }
}