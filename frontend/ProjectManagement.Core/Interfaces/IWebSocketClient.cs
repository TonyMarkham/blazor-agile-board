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

    // ==========================================================================
    // Time Entry Events
    // ==========================================================================

    /// <summary>
    /// Fired when a timer is started.
    /// Parameters: (started entry, optionally the entry that was auto-stopped)
    /// </summary>
    event Action<TimeEntry, TimeEntry?>? OnTimerStarted;

    /// <summary>Fired when a timer is stopped.</summary>
    event Action<TimeEntry>? OnTimerStopped;

    /// <summary>Fired when a manual time entry is created.</summary>
    event Action<TimeEntry>? OnTimeEntryCreated;

    /// <summary>Fired when a time entry is updated.</summary>
    event Action<TimeEntry>? OnTimeEntryUpdated;

    /// <summary>
    /// Fired when a time entry is deleted.
    /// Parameters: (timeEntryId, workItemId)
    /// </summary>
    event Action<Guid, Guid>? OnTimeEntryDeleted;

    // ==========================================================================
    // Time Entry Operations
    // ==========================================================================

    /// <summary>
    /// Start a timer on a work item.
    /// If the user already has a running timer, it will be automatically stopped.
    /// </summary>
    /// <returns>
    /// Tuple of (started entry, stopped entry if any was auto-stopped)
    /// </returns>
    Task<(TimeEntry Started, TimeEntry? Stopped)> StartTimerAsync(
        StartTimerRequest request,
        CancellationToken ct = default);

    /// <summary>
    /// Stop a running timer.
    /// Only the owner can stop their timer.
    /// </summary>
    /// <returns>The stopped entry with duration calculated.</returns>
    Task<TimeEntry> StopTimerAsync(
        Guid timeEntryId,
        CancellationToken ct = default);

    /// <summary>
    /// Create a manual (already completed) time entry.
    /// Use this for logging time after the fact.
    /// </summary>
    Task<TimeEntry> CreateTimeEntryAsync(
        CreateTimeEntryRequest request,
        CancellationToken ct = default);

    /// <summary>
    /// Update an existing time entry.
    /// Only the owner can update their entries.
    /// </summary>
    Task<TimeEntry> UpdateTimeEntryAsync(
        UpdateTimeEntryRequest request,
        CancellationToken ct = default);

    /// <summary>
    /// Delete a time entry (soft delete).
    /// Only the owner can delete their entries.
    /// </summary>
    Task DeleteTimeEntryAsync(
        Guid timeEntryId,
        CancellationToken ct = default);

    /// <summary>
    /// Get time entries for a work item with pagination.
    /// </summary>
    /// <param name="workItemId">The work item to get entries for.</param>
    /// <param name="limit">Max entries to return (default 100, max 500).</param>
    /// <param name="offset">Number of entries to skip for pagination.</param>
    /// <returns>Tuple of (entries, total count for pagination).</returns>
    Task<(IReadOnlyList<TimeEntry> Entries, int TotalCount)> GetTimeEntriesAsync(
        Guid workItemId,
        int? limit = null,
        int? offset = null,
        CancellationToken ct = default);

    /// <summary>
    /// Get the current user's running timer, if any.
    /// </summary>
    /// <returns>The running timer, or null if none.</returns>
    Task<TimeEntry?> GetRunningTimerAsync(CancellationToken ct = default);

    // ==========================================================================
    // Dependency Events
    // ==========================================================================

    /// <summary>Fired when a dependency is created.</summary>
    event Action<Dependency>? OnDependencyCreated;

    /// <summary>
    /// Fired when a dependency is deleted.
    /// Parameters: (dependencyId, blockingItemId, blockedItemId)
    /// </summary>
    event Action<Guid, Guid, Guid>? OnDependencyDeleted;

    // ==========================================================================
    // Dependency Operations
    // ==========================================================================

    /// <summary>
    /// Create a dependency between two work items.
    /// Both items must be in the same project.
    /// Circular dependencies are rejected for Blocks type.
    /// </summary>
    Task<Dependency> CreateDependencyAsync(
        CreateDependencyRequest request,
        CancellationToken ct = default);

    /// <summary>
    /// Delete a dependency.
    /// Requires Edit permission on the project.
    /// </summary>
    Task DeleteDependencyAsync(
        Guid dependencyId,
        CancellationToken ct = default);

    /// <summary>
    /// Get dependencies for a work item.
    /// Returns both items blocking this one and items blocked by this one.
    /// </summary>
    /// <returns>
    /// Tuple of (items blocking this work item, items blocked by this work item)
    /// </returns>
    Task<(IReadOnlyList<Dependency> Blocking, IReadOnlyList<Dependency> Blocked)> GetDependenciesAsync(
        Guid workItemId,
        CancellationToken ct = default);

    // ============================================================
    // Activity Log Events
    // ============================================================

    /// <summary>Fired when an activity log entry is created.</summary>
    event Action<ActivityLog>? OnActivityLogCreated;

    // ============================================================
    // Activity Log Operations
    // ============================================================

    /// <summary>
    /// Get activity log entries for an entity.
    /// </summary>
    Task<ActivityLogPage> GetActivityLogAsync(
        GetActivityLogRequest request,
        CancellationToken ct = default);
}