  using ProjectManagement.Core.Models;
  using ProjectManagement.Core.ViewModels;

  namespace ProjectManagement.Core.Interfaces;

  /// <summary>
  /// Store for managing project state with optimistic updates.
  /// </summary>
  public interface IProjectStore
  {
      /// <summary>
      /// All non-deleted projects, ordered by title.
      /// </summary>
      IReadOnlyList<ProjectViewModel> Projects { get; }

      /// <summary>
      /// The currently selected project (for context display).
      /// </summary>
      ProjectViewModel? CurrentProject { get; }

      /// <summary>
      /// Whether the store has been loaded from the server.
      /// </summary>
      bool IsLoaded { get; }

      /// <summary>
      /// Fired when the current project changes.
      /// </summary>
      event Action? OnCurrentProjectChanged;

      /// <summary>
      /// Fired when the projects list changes.
      /// </summary>
      event Action? OnProjectsChanged;

      /// <summary>
      /// Set the current project context.
      /// </summary>
      void SetCurrentProject(Guid projectId);

      /// <summary>
      /// Clear the current project context.
      /// </summary>
      void ClearCurrentProject();

      /// <summary>
      /// Get a project by ID.
      /// </summary>
      ProjectViewModel? GetById(Guid id);

      /// <summary>
      /// Create a new project.
      /// </summary>
      Task<Project> CreateAsync(CreateProjectRequest request, CancellationToken ct = default);

      /// <summary>
      /// Update an existing project.
      /// </summary>
      Task<Project> UpdateAsync(UpdateProjectRequest request, CancellationToken ct = default);

      /// <summary>
      /// Delete a project.
      /// </summary>
      Task DeleteAsync(Guid id, int expectedVersion, CancellationToken ct = default);

      /// <summary>
      /// Refresh projects from the server.
      /// </summary>
      Task RefreshAsync(CancellationToken ct = default);

      /// <summary>
      /// Check if a project has pending changes.
      /// </summary>
      bool IsPending(Guid id);
  }