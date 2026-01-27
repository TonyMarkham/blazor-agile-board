using Microsoft.Extensions.Logging;
using ProjectManagement.Core.Interfaces;
using ProjectManagement.Core.Models;

namespace ProjectManagement.Services.State;

/// <summary>
///     Root state container for the application.
///     Provides centralized access to all stores.
/// </summary>
public sealed class AppState : IDisposable
{
    private readonly IWebSocketClient _client;
    private readonly ILogger<AppState> _logger;
    
    private UserIdentity? _currentUser;
    private readonly List<Action<ConnectionState>> _connectionStateCallbacks = new();

    public UserIdentity? CurrentUser => _currentUser;

    private bool _disposed;

    public AppState(
        IWebSocketClient client,
        IWorkItemStore workItems,
        ISprintStore sprints,
        IProjectStore projects,
        ICommentStore comments,
        ILogger<AppState> logger)
    {
        _client = client;
        _logger = logger;

        WorkItems = workItems;
        Sprints = sprints;
        Projects = projects;
        Comments = comments;

        // Forward events                                                                                             
        _client.OnStateChanged += state =>
        {
            OnConnectionStateChanged?.Invoke(state);
            NotifyConnectionStateChanged(state);
            OnStateChanged?.Invoke();
        };

        workItems.OnChanged += () => OnStateChanged?.Invoke();
        sprints.OnChanged += () => OnStateChanged?.Invoke();
        projects.OnCurrentProjectChanged += () => OnStateChanged?.Invoke();
        projects.OnProjectsChanged += () => OnStateChanged?.Invoke();
        comments.OnChanged += () => OnStateChanged?.Invoke();
    }

    public IWorkItemStore WorkItems { get; }
    public ISprintStore Sprints { get; }
    public IProjectStore Projects { get; }
    public ICommentStore Comments { get; }
    public IConnectionHealth ConnectionHealth => _client.Health;
    public ConnectionState ConnectionState => _client.State;

    public void Dispose()
    {
        if (_disposed) return;
        _disposed = true;

        if (WorkItems is IDisposable workItemsDisposable)
            workItemsDisposable.Dispose();
        if (Sprints is IDisposable sprintsDisposable)
            sprintsDisposable.Dispose();
        if (Projects is IDisposable projectsDisposable)
            projectsDisposable.Dispose();
        if (Comments is IDisposable commentsDisposable)
            commentsDisposable.Dispose();
    }
    
    public void SetCurrentUser(UserIdentity user)
    {
        _currentUser = user ?? throw new ArgumentNullException(nameof(user));
        _logger.LogInformation("AppState user set: {UserId}", user.Id);
    }

    public IDisposable SubscribeToConnectionStateChanged(Action<ConnectionState> callback)
    {
        _connectionStateCallbacks.Add(callback);
        return new CallbackDisposable(() => _connectionStateCallbacks.Remove(callback));
    }

    private void NotifyConnectionStateChanged(ConnectionState state)
    {
        foreach (var callback in _connectionStateCallbacks.ToList())
        {
            try
            {
                callback(state);
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Error in connection state callback");
            }
        }
    }

    private sealed class CallbackDisposable : IDisposable
    {
        private readonly Action _onDispose;
        public CallbackDisposable(Action onDispose) => _onDispose = onDispose;
        public void Dispose() => _onDispose();
    }

    public event Action? OnStateChanged;
    public event Action<ConnectionState>? OnConnectionStateChanged;

    /// <summary>
    ///     Initialize state by connecting and loading initial data.
    /// </summary>
    public async Task InitializeAsync(CancellationToken ct = default)
    {
        ThrowIfDisposed();

        if (_currentUser == null)
        {
            throw new InvalidOperationException(
                "CurrentUser must be set before InitializeAsync. Call SetCurrentUser() first.");
        }

        _logger.LogInformation("Initializing application state for user {UserId}", _currentUser.Id);

        await _client.ConnectAsync(_currentUser.Id, ct);

        _logger.LogInformation("Application state initialized");
    }

    /// <summary>
    ///     Load data for a specific project.
    /// </summary>
    public async Task LoadProjectAsync(Guid projectId, CancellationToken ct = default)
    {
        ThrowIfDisposed();

        _logger.LogInformation("Loading project {ProjectId}", projectId);

        // Subscribe to project updates
        await _client.SubscribeAsync([projectId], ct);

        // Load initial data
        await WorkItems.RefreshAsync(projectId, ct);
        await Sprints.RefreshAsync(projectId, ct);

        _logger.LogInformation("Project {ProjectId} loaded", projectId);
    }
    
    /// <summary>
    ///     Load all projects from the server.
    /// </summary>
    public async Task LoadProjectsAsync(CancellationToken ct = default)
    {
        ThrowIfDisposed();

        _logger.LogInformation("Loading projects");
        await Projects.RefreshAsync(ct);
        _logger.LogInformation("Projects loaded");
    }

    private void ThrowIfDisposed()
    {
        ObjectDisposedException.ThrowIf(_disposed, this);
    }
}