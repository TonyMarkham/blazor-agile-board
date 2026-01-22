using ProjectManagement.Core.Models;

namespace ProjectManagement.Core.Interfaces;

public interface IWebSocketClient : IAsyncDisposable
{
    /// <summary>Current connection state.</summary>
    ConnectionState State { get; }

    /// <summary>Connection health metrics.</summary>
    IConnectionHealth Health { get; }

    /// <summary>Fired when connection state changes.</summary>
    event Action<ConnectionState>? OnStateChanged;

    /// <summary>Fired when a work item is created by another user.</summary>
    event Action<WorkItem>? OnWorkItemCreated;

    /// <summary>Fired when a work item is updated by another user.</summary>
    event Action<WorkItem, IReadOnlyList<FieldChange>>? OnWorkItemUpdated;

    /// <summary>Fired when a work item is deleted by another user.</summary>
    event Action<Guid>? OnWorkItemDeleted;

    // Connection
    Task ConnectAsync(CancellationToken ct = default);
    Task DisconnectAsync(CancellationToken ct = default);

    // Subscriptions
    Task SubscribeAsync(IEnumerable<Guid> projectIds, CancellationToken ct = default);
    Task UnsubscribeAsync(IEnumerable<Guid> projectIds, CancellationToken ct = default);

    // Work Item Operations
    Task<WorkItem> CreateWorkItemAsync(CreateWorkItemRequest request, CancellationToken ct = default);
    Task<WorkItem> UpdateWorkItemAsync(UpdateWorkItemRequest request, CancellationToken ct = default);
    Task DeleteWorkItemAsync(Guid workItemId, CancellationToken ct = default);

    Task<IReadOnlyList<WorkItem>> GetWorkItemsAsync(Guid projectId, DateTime? since = null,
        CancellationToken ct = default);
    
    // ============================================================
    // Project Events
    // ============================================================

    /// <summary>
    /// Fired when a project is created (by us or another user).
    /// </summary>
    event Action<Project>? OnProjectCreated;

    /// <summary>
    /// Fired when a project is updated (by us or another user).
    /// </summary>
    event Action<Project, IReadOnlyList<FieldChange>>? OnProjectUpdated;

    /// <summary>
    /// Fired when a project is deleted (by us or another user).
    /// </summary>
    event Action<Guid>? OnProjectDeleted;

    // ============================================================
    // Project Operations
    // ============================================================

    /// <summary>
    /// Create a new project.
    /// </summary>
    Task<Project> CreateProjectAsync(CreateProjectRequest request, CancellationToken ct = default);

    /// <summary>
    /// Update an existing project.
    /// </summary>
    Task<Project> UpdateProjectAsync(UpdateProjectRequest request, CancellationToken ct = default);

    /// <summary>
    /// Delete a project (soft delete).
    /// </summary>
    Task DeleteProjectAsync(Guid projectId, int expectedVersion, CancellationToken ct = default);

    /// <summary>
    /// Get all projects.
    /// </summary>
    Task<IReadOnlyList<Project>> GetProjectsAsync(CancellationToken ct = default);
}