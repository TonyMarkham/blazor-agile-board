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
    Task ConnectAsync(Guid userId, CancellationToken ct = default);
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

    // ============================================================
    // Sprint Events (received from server)
    // ============================================================

    /// <summary>
    /// Fired when a sprint is created (by this or another client).
    /// </summary>
    event Action<Sprint>? OnSprintCreated;

    /// <summary>
    /// Fired when a sprint is updated (by this or another client).
    /// </summary>
    event Action<Sprint, IReadOnlyList<FieldChange>>? OnSprintUpdated;

    /// <summary>
    /// Fired when a sprint is deleted (by this or another client).
    /// </summary>
    event Action<Guid>? OnSprintDeleted;

    // ============================================================
    // Sprint Operations (send to server)
    // ============================================================

    /// <summary>
    /// Create a new sprint in a project.
    /// </summary>
    Task<Sprint> CreateSprintAsync(CreateSprintRequest request, CancellationToken ct = default);

    /// <summary>
    /// Update an existing sprint.
    /// Uses optimistic locking - ExpectedVersion must match server's version.
    /// </summary>
    Task<Sprint> UpdateSprintAsync(UpdateSprintRequest request, CancellationToken ct = default);

    /// <summary>
    /// Delete a sprint (soft delete).
    /// Requires admin permission. Cannot delete completed sprints.
    /// </summary>
    Task DeleteSprintAsync(Guid sprintId, CancellationToken ct = default);

    /// <summary>
    /// Get all sprints for a project.
    /// </summary>
    Task<IReadOnlyList<Sprint>> GetSprintsAsync(Guid projectId, CancellationToken ct = default);

    // ============================================================
    // Comment Events (received from server)
    // ============================================================

    /// <summary>
    /// Fired when a comment is created (by this or another client).
    /// </summary>
    event Action<Comment>? OnCommentCreated;

    /// <summary>
    /// Fired when a comment is updated (by this or another client).
    /// </summary>
    event Action<Comment>? OnCommentUpdated;

    /// <summary>
    /// Fired when a comment is deleted (by this or another client).
    /// </summary>
    event Action<Guid>? OnCommentDeleted;

    // ============================================================
    // Comment Operations (send to server)
    // ============================================================

    /// <summary>
    /// Create a comment on a work item.
    /// </summary>
    Task<Comment> CreateCommentAsync(CreateCommentRequest request, CancellationToken ct = default);

    /// <summary>
    /// Update a comment.
    /// Only the comment author can update their own comment.
    /// </summary>
    Task<Comment> UpdateCommentAsync(UpdateCommentRequest request, CancellationToken ct = default);

    /// <summary>
    /// Delete a comment.
    /// Only the comment author can delete their own comment.
    /// </summary>
    Task DeleteCommentAsync(Guid commentId, CancellationToken ct = default);

    /// <summary>
    /// Get all comments for a work item.
    /// </summary>
    Task<IReadOnlyList<Comment>> GetCommentsAsync(Guid workItemId, CancellationToken ct = default);
}