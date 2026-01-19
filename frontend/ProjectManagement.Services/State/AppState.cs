using Microsoft.Extensions.Logging;
using ProjectManagement.Core.Interfaces;

namespace ProjectManagement.Services.State;

/// <summary>
///     Root state container for the application.
///     Provides centralized access to all stores.
/// </summary>
public sealed class AppState : IDisposable
{
    private readonly IWebSocketClient _client;
    private readonly ILogger<AppState> _logger;

    private bool _disposed;

    public AppState(
        IWebSocketClient client,
        IWorkItemStore workItems,
        ISprintStore sprints,
        ILogger<AppState> logger)
    {
        _client = client;
        _logger = logger;

        WorkItems = workItems;
        Sprints = sprints;

        // Forward events                                                                                             
        _client.OnStateChanged += state =>
        {
            OnConnectionStateChanged?.Invoke(state);
            OnStateChanged?.Invoke();
        };

        workItems.OnChanged += () => OnStateChanged?.Invoke();
        sprints.OnChanged += () => OnStateChanged?.Invoke();
    }

    public IWorkItemStore WorkItems { get; }
    public ISprintStore Sprints { get; }
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
    }

    public event Action? OnStateChanged;
    public event Action<ConnectionState>? OnConnectionStateChanged;

    /// <summary>
    ///     Initialize state by connecting and loading initial data.
    /// </summary>
    public async Task InitializeAsync(CancellationToken ct = default)
    {
        ThrowIfDisposed();

        _logger.LogInformation("Initializing application state");

        await _client.ConnectAsync(ct);

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

    private void ThrowIfDisposed()
    {
        ObjectDisposedException.ThrowIf(_disposed, this);
    }
}