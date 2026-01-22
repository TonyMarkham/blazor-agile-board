  using ProjectManagement.Core.Exceptions;
  using ProjectManagement.Core.Models;

  namespace ProjectManagement.Core.Validation;

  /// <summary>
  /// Validates user registration input.
  /// </summary>
  public static class RegistrationValidator
  {
      public const int MaxNameLength = 100;
      public const int MaxEmailLength = 254; // RFC 5321 standard

      /// <summary>
      /// Validates name and email for user registration.
      /// </summary>
      /// <param name="name">Optional user name</param>
      /// <param name="email">Optional email address</param>
      /// <returns>ValidationResult with any errors found</returns>
      public static ValidationResult Validate(string? name, string? email)
      {
          var errors = new List<ValidationError>();

          // Validate name length
          if (!string.IsNullOrWhiteSpace(name) && name.Length > MaxNameLength)
          {
              errors.Add(new ValidationError(
                  Field: "Name",
                  Message: $"Name cannot exceed {MaxNameLength} characters"
              ));
          }

          // Validate email if provided
          if (!string.IsNullOrWhiteSpace(email))
          {
              if (email.Length > MaxEmailLength)
              {
                  errors.Add(new ValidationError(
                      Field: "Email",
                      Message: $"Email cannot exceed {MaxEmailLength} characters"
                  ));
              }
              else if (!UserIdentity.IsValidEmail(email))
              {
                  errors.Add(new ValidationError(
                      Field: "Email",
                      Message: "Please enter a valid email address"
                  ));
              }
          }

          return errors.Count == 0
              ? ValidationResult.Success()
              : ValidationResult.Failure(errors);
      }

      /// <summary>
      /// Validates and throws if invalid. Use in service layer.
      /// </summary>
      public static void ValidateAndThrow(string? name, string? email)
      {
          var result = Validate(name, email);
          result.ThrowIfInvalid();
      }
  }