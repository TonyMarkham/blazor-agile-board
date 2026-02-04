  namespace ProjectManagement.Core.Models;

  /// <summary>
  /// A project is a top-level organizational container for work items.
  /// Unlike work items, projects have a unique key and can be archived.
  /// </summary>
  public sealed record Project
  {
      /// <summary>
      /// Unique identifier.
      /// </summary>
      public required Guid Id { get; init; }

      /// <summary>
      /// Project title/name.
      /// </summary>
      public required string Title { get; init; }

      /// <summary>
      /// Optional description.
      /// </summary>
      public string? Description { get; init; }

      /// <summary>
      /// Unique short identifier (e.g., "PROJ", "WEBAPP").
      /// Immutable after creation.
      /// </summary>
      public required string Key { get; init; }

      /// <summary>
      /// Current lifecycle status.
      /// </summary>
      public required ProjectStatus Status { get; init; }

      /// <summary>
      /// Version for optimistic concurrency control.
      /// </summary>
      public required int Version { get; init; }

      /// <summary>
      /// When the project was created.
      /// </summary>
      public required DateTime CreatedAt { get; init; }

      /// <summary>
      /// When the project was last updated.
      /// </summary>
      public required DateTime UpdatedAt { get; init; }

      /// <summary>
      /// User who created the project.
      /// </summary>
      public required Guid CreatedBy { get; init; }

      /// <summary>
      /// User who last updated the project.
      /// </summary>
      public required Guid UpdatedBy { get; init; }

      /// <summary>
      /// When the project was soft-deleted (null if not deleted).
      /// </summary>
      public DateTime? DeletedAt { get; init; }
      /// <summary>
      /// Next sequential number to assign to work items.
      /// Atomically incremented when creating work items.
      /// </summary>
      public int NextWorkItemNumber { get; init; } = 1;

      /// <summary>
      /// Check if the project is deleted.
      /// </summary>
      public bool IsDeleted => DeletedAt.HasValue;

      /// <summary>
      /// Check if the project is archived.
      /// </summary>
      public bool IsArchived => Status == ProjectStatus.Archived;
  }