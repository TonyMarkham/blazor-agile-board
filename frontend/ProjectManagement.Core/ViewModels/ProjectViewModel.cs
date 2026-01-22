  using ProjectManagement.Core.Models;

  namespace ProjectManagement.Core.ViewModels;

  /// <summary>
  /// ViewModel for Project, adding pending sync state and computed properties.
  /// </summary>
  public sealed class ProjectViewModel : IViewModel<Project>, IEquatable<ProjectViewModel>
  {
      /// <summary>
      /// Create a new ProjectViewModel.
      /// </summary>
      public ProjectViewModel(Project model, bool isPendingSync = false)
      {
          Model = model;
          IsPendingSync = isPendingSync;
      }

      /// <summary>
      /// The underlying Project model.
      /// </summary>
      public Project Model { get; }

      /// <summary>
      /// Whether this item has pending changes not yet confirmed by the server.
      /// </summary>
      public bool IsPendingSync { get; }

      // ============================================================
      // Delegated Properties
      // ============================================================

      public Guid Id => Model.Id;
      public string Title => Model.Title;
      public string? Description => Model.Description;
      public string Key => Model.Key;
      public ProjectStatus Status => Model.Status;
      public int Version => Model.Version;
      public DateTime CreatedAt => Model.CreatedAt;
      public DateTime UpdatedAt => Model.UpdatedAt;
      public Guid CreatedBy => Model.CreatedBy;
      public Guid UpdatedBy => Model.UpdatedBy;
      public DateTime? DeletedAt => Model.DeletedAt;
      public bool IsDeleted => Model.IsDeleted;

      // ============================================================
      // Computed Properties
      // ============================================================

      /// <summary>
      /// Human-readable status name.
      /// </summary>
      public string StatusDisplayName => Status switch
      {
          ProjectStatus.Active => "Active",
          ProjectStatus.Archived => "Archived",
          _ => Status.ToString()
      };

      /// <summary>
      /// Whether the project is archived.
      /// </summary>
      public bool IsArchived => Status == ProjectStatus.Archived;

      /// <summary>
      /// CSS class for status badge.
      /// </summary>
      public string StatusCssClass => Status switch
      {
          ProjectStatus.Active => "badge-success",
          ProjectStatus.Archived => "badge-secondary",
          _ => "badge-default"
      };

      // ============================================================
      // Equality
      // ============================================================

      public bool Equals(ProjectViewModel? other) =>
          other is not null
          && Id == other.Id
          && Version == other.Version
          && IsPendingSync == other.IsPendingSync;

      public override bool Equals(object? obj) => Equals(obj as ProjectViewModel);

      public override int GetHashCode() => HashCode.Combine(Id, Version, IsPendingSync);

      public static bool operator ==(ProjectViewModel? left, ProjectViewModel? right) =>
          left?.Equals(right) ?? right is null;

      public static bool operator !=(ProjectViewModel? left, ProjectViewModel? right) =>
          !(left == right);
  }