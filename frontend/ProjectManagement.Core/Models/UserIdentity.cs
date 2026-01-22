  using System.ComponentModel.DataAnnotations;
  using System.Text.Json.Serialization;
  using System.Text.RegularExpressions;

  namespace ProjectManagement.Core.Models;

  /// <summary>
  /// Persistent user identity stored locally on the desktop.
  /// Used to maintain consistent user ID across app restarts.
  /// </summary>
  public sealed record UserIdentity
  {
      /// <summary>Current schema version for migration support.</summary>
      public const int CurrentSchemaVersion = 1;

      [JsonPropertyName("id")]
      public required Guid Id { get; init; }

      [JsonPropertyName("name")]
      [StringLength(100, ErrorMessage = "Name cannot exceed 100 characters")]
      public string? Name { get; init; }

      [JsonPropertyName("email")]
      [EmailAddress(ErrorMessage = "Invalid email format")]
      public string? Email { get; init; }

      [JsonPropertyName("created_at")]
      public required DateTime CreatedAt { get; init; }

      [JsonPropertyName("schema_version")]
      public int SchemaVersion { get; init; } = CurrentSchemaVersion;

      /// <summary>
      /// Creates a new user identity with a fresh UUID.
      /// </summary>
      public static UserIdentity Create(string? name = null, string? email = null)
      {
          // Validate email format if provided
          if (!string.IsNullOrWhiteSpace(email) && !IsValidEmail(email))
          {
              throw new ArgumentException("Invalid email format", nameof(email));
          }

          return new UserIdentity
          {
              Id = Guid.NewGuid(),
              Name = name?.Trim(),
              Email = email?.Trim().ToLowerInvariant(),
              CreatedAt = DateTime.UtcNow,
              SchemaVersion = CurrentSchemaVersion
          };
      }

      /// <summary>
      /// Validates email format using regex.
      /// </summary>
      public static bool IsValidEmail(string? email)
      {
          if (string.IsNullOrWhiteSpace(email))
              return true; // Empty is valid (optional field)

          // RFC 5322 simplified pattern
          const string pattern = @"^[^@\s]+@[^@\s]+\.[^@\s]+$";
          return Regex.IsMatch(email, pattern, RegexOptions.IgnoreCase);
      }

      /// <summary>
      /// Migrates identity from older schema version if needed.
      /// </summary>
      public static UserIdentity Migrate(UserIdentity old)
      {
          if (old.SchemaVersion >= CurrentSchemaVersion)
              return old;

          // Future migrations go here
          // if (old.SchemaVersion < 2) { ... migrate to v2 ... }

          return old with { SchemaVersion = CurrentSchemaVersion };
      }
  }