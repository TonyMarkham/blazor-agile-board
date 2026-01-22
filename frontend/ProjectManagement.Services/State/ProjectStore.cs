  using System.Collections.Concurrent;
  using Microsoft.Extensions.Logging;
  using ProjectManagement.Core.Interfaces;
  using ProjectManagement.Core.Models;
  using ProjectManagement.Core.ViewModels;

  namespace ProjectManagement.Services.State;

  /// <summary>
  /// Store for project state with optimistic updates and server sync.
  /// </summary>
  public sealed class ProjectStore : IProjectStore, IDisposable
  {
      private readonly IWebSocketClient _client;
      private readonly ILogger<ProjectStore> _logger;
      private readonly ConcurrentDictionary<Guid, Project> _projects = new();
      private readonly ConcurrentDictionary<Guid, OptimisticUpdate<Project>> _pendingUpdates = new();
      private Guid? _currentProjectId;
      private bool _disposed;

      public ProjectStore(IWebSocketClient client, ILogger<ProjectStore> logger)
      {
          _client = client;
          _logger = logger;

          // Subscribe to server events
          _client.OnProjectCreated += HandleProjectCreated;
          _client.OnProjectUpdated += HandleProjectUpdated;
          _client.OnProjectDeleted += HandleProjectDeleted;
      }

      public IReadOnlyList<ProjectViewModel> Projects => _projects.Values
          .Where(p => p.DeletedAt is null)
          .OrderBy(p => p.Title)
          .Select(p => new ProjectViewModel(p, _pendingUpdates.ContainsKey(p.Id)))
          .ToList();

      public ProjectViewModel? CurrentProject => _currentProjectId.HasValue
          && _projects.TryGetValue(_currentProjectId.Value, out var p)
          ? new ProjectViewModel(p, _pendingUpdates.ContainsKey(p.Id))
          : null;

      public bool IsLoaded { get; private set; }

      public event Action? OnCurrentProjectChanged;
      public event Action? OnProjectsChanged;

      public ProjectViewModel? GetById(Guid id) =>
          _projects.TryGetValue(id, out var p)
              ? new ProjectViewModel(p, _pendingUpdates.ContainsKey(id))
              : null;

      public void SetCurrentProject(Guid projectId)
      {
          if (_currentProjectId == projectId) return;
          _currentProjectId = projectId;
          NotifyCurrentProjectChanged();
      }

      public void ClearCurrentProject()
      {
          if (_currentProjectId is null) return;
          _currentProjectId = null;
          NotifyCurrentProjectChanged();
      }

      public async Task<Project> CreateAsync(CreateProjectRequest request, CancellationToken ct)
      {
          // Create optimistic project with temp ID
          var tempId = Guid.NewGuid();
          var optimistic = new Project
          {
              Id = tempId,
              Title = request.Title,
              Description = request.Description,
              Key = request.Key.ToUpperInvariant(),
              Status = ProjectStatus.Active,
              Version = 1,
              CreatedAt = DateTime.UtcNow,
              UpdatedAt = DateTime.UtcNow,
              CreatedBy = Guid.Empty, // Unknown until server responds
              UpdatedBy = Guid.Empty,
          };

          // Add optimistically
          _projects[tempId] = optimistic;
          _pendingUpdates[tempId] = new OptimisticUpdate<Project>(tempId, null, optimistic);
          NotifyProjectsChanged();

          try
          {
              var created = await _client.CreateProjectAsync(request, ct);

              // Replace temp with real
              _projects.TryRemove(tempId, out _);
              _pendingUpdates.TryRemove(tempId, out _);
              _projects[created.Id] = created;
              NotifyProjectsChanged();

              _logger.LogInformation("Created project {ProjectId} ({Key})", created.Id, created.Key);
              return created;
          }
          catch (Exception ex)
          {
              // Rollback
              _projects.TryRemove(tempId, out _);
              _pendingUpdates.TryRemove(tempId, out _);
              NotifyProjectsChanged();

              _logger.LogError(ex, "Failed to create project");
              throw;
          }
      }

      public async Task<Project> UpdateAsync(UpdateProjectRequest request, CancellationToken ct)
      {
          if (!_projects.TryGetValue(request.ProjectId, out var current))
              throw new InvalidOperationException($"Project {request.ProjectId} not found in store");

          // Create optimistic version
          var optimistic = current with
          {
              Title = request.Title ?? current.Title,
              Description = request.Description ?? current.Description,
              Status = request.Status ?? current.Status,
              Version = current.Version + 1,
              UpdatedAt = DateTime.UtcNow,
          };

          _projects[request.ProjectId] = optimistic;
          _pendingUpdates[request.ProjectId] = new OptimisticUpdate<Project>(request.ProjectId, current, optimistic);
          NotifyProjectsChanged();

          try
          {
              var updated = await _client.UpdateProjectAsync(request, ct);
              _projects[updated.Id] = updated;
              _pendingUpdates.TryRemove(request.ProjectId, out _);
              NotifyProjectsChanged();

              _logger.LogInformation("Updated project {ProjectId}", updated.Id);
              return updated;
          }
          catch (Exception ex)
          {
              // Rollback
              _projects[request.ProjectId] = current;
              _pendingUpdates.TryRemove(request.ProjectId, out _);
              NotifyProjectsChanged();

              _logger.LogError(ex, "Failed to update project {ProjectId}", request.ProjectId);
              throw;
          }
      }

      public async Task DeleteAsync(Guid id, int expectedVersion, CancellationToken ct)
      {
          if (!_projects.TryGetValue(id, out var current))
              throw new InvalidOperationException($"Project {id} not found in store");

          // Optimistic delete (mark as deleted)
          var optimistic = current with { DeletedAt = DateTime.UtcNow };
          _projects[id] = optimistic;
          _pendingUpdates[id] = new OptimisticUpdate<Project>(id, current, optimistic);
          NotifyProjectsChanged();

          try
          {
              await _client.DeleteProjectAsync(id, expectedVersion, ct);

              _projects.TryRemove(id, out _);
              _pendingUpdates.TryRemove(id, out _);

              // Clear current if deleted
              if (_currentProjectId == id)
              {
                  _currentProjectId = null;
                  NotifyCurrentProjectChanged();
              }

              NotifyProjectsChanged();
              _logger.LogInformation("Deleted project {ProjectId}", id);
          }
          catch (Exception ex)
          {
              // Rollback
              _projects[id] = current;
              _pendingUpdates.TryRemove(id, out _);
              NotifyProjectsChanged();

              _logger.LogError(ex, "Failed to delete project {ProjectId}", id);
              throw;
          }
      }

      public async Task RefreshAsync(CancellationToken ct)
      {
          var projects = await _client.GetProjectsAsync(ct);

          _projects.Clear();
          foreach (var p in projects)
              _projects[p.Id] = p;

          IsLoaded = true;
          NotifyProjectsChanged();

          _logger.LogDebug("Refreshed {Count} projects", projects.Count);
      }

      public bool IsPending(Guid id) => _pendingUpdates.ContainsKey(id);

      // ============================================================
      // Event Handlers
      // ============================================================

      private void HandleProjectCreated(Project p)
      {
          // Skip if we created it (already in store)
          if (_pendingUpdates.ContainsKey(p.Id)) return;

          _projects[p.Id] = p;
          NotifyProjectsChanged();
      }

      private void HandleProjectUpdated(Project p, IReadOnlyList<FieldChange> _)
      {
          // Skip if we updated it (already applied optimistically)
          if (_pendingUpdates.ContainsKey(p.Id)) return;

          _projects[p.Id] = p;
          NotifyProjectsChanged();
      }

      private void HandleProjectDeleted(Guid id)
      {
          // Skip if we deleted it (already removed optimistically)
          if (_pendingUpdates.ContainsKey(id)) return;

          _projects.TryRemove(id, out _);

          if (_currentProjectId == id)
          {
              _currentProjectId = null;
              NotifyCurrentProjectChanged();
          }

          NotifyProjectsChanged();
      }

      // ============================================================
      // Notifications
      // ============================================================

      private void NotifyProjectsChanged()
      {
          try { OnProjectsChanged?.Invoke(); }
          catch (Exception ex) { _logger.LogError(ex, "Error in OnProjectsChanged handler"); }
      }

      private void NotifyCurrentProjectChanged()
      {
          try { OnCurrentProjectChanged?.Invoke(); }
          catch (Exception ex) { _logger.LogError(ex, "Error in OnCurrentProjectChanged handler"); }
      }

      // ============================================================
      // Dispose
      // ============================================================

      public void Dispose()
      {
          if (_disposed) return;
          _disposed = true;

          _client.OnProjectCreated -= HandleProjectCreated;
          _client.OnProjectUpdated -= HandleProjectUpdated;
          _client.OnProjectDeleted -= HandleProjectDeleted;
      }
  }