using Microsoft.Extensions.Logging;
using ProjectManagement.Core.Interfaces;
using ProjectManagement.Core.Models;
using ProjectManagement.Services.WebSocket;

namespace ProjectManagement.Services.Resilience;

/// <summary>
///     WebSocket client wrapper with circuit breaker and retry protection.
/// </summary>
public sealed class ResilientWebSocketClient : IWebSocketClient
{
    private readonly CircuitBreaker _circuitBreaker;
    private readonly WebSocketClient _inner;
    private readonly ILogger<ResilientWebSocketClient> _logger;
    private readonly RetryPolicy _retryPolicy;

    public ResilientWebSocketClient(
        WebSocketClient inner,
        CircuitBreaker circuitBreaker,
        RetryPolicy retryPolicy,
        ILogger<ResilientWebSocketClient> logger)
    {
        _inner = inner;
        _circuitBreaker = circuitBreaker;
        _retryPolicy = retryPolicy;
        _logger = logger;
    }

    public ConnectionState State => _inner.State;
    public IConnectionHealth Health => _inner.Health;

    public event Action<ConnectionState>? OnStateChanged
    {
        add => _inner.OnStateChanged += value;
        remove => _inner.OnStateChanged -= value;
    }

    public event Action<WorkItem>? OnWorkItemCreated
    {
        add => _inner.OnWorkItemCreated += value;
        remove => _inner.OnWorkItemCreated -= value;
    }

    public event Action<WorkItem, IReadOnlyList<FieldChange>>? OnWorkItemUpdated
    {
        add => _inner.OnWorkItemUpdated += value;
        remove => _inner.OnWorkItemUpdated -= value;
    }

    public event Action<Guid>? OnWorkItemDeleted
    {
        add => _inner.OnWorkItemDeleted += value;
        remove => _inner.OnWorkItemDeleted -= value;
    }

    public event Action<Project>? OnProjectCreated
    {
        add => _inner.OnProjectCreated += value;
        remove => _inner.OnProjectCreated -= value;
    }

    public event Action<Project, IReadOnlyList<FieldChange>>? OnProjectUpdated
    {
        add => _inner.OnProjectUpdated += value;
        remove => _inner.OnProjectUpdated -= value;
    }

    public event Action<Guid>? OnProjectDeleted
    {
        add => _inner.OnProjectDeleted += value;
        remove => _inner.OnProjectDeleted -= value;
    }

    public Task ConnectAsync(CancellationToken ct = default)
    {
        return _inner.ConnectAsync(ct);
    }

    public Task DisconnectAsync(CancellationToken ct = default)
    {
        return _inner.DisconnectAsync(ct);
    }

    public Task SubscribeAsync(IEnumerable<Guid> projectIds, CancellationToken ct = default)
    {
        return ExecuteWithResilienceAsync(
            token => _inner.SubscribeAsync(projectIds, token),
            ct);
    }

    public Task UnsubscribeAsync(IEnumerable<Guid> projectIds, CancellationToken ct = default)
    {
        return ExecuteWithResilienceAsync(
            token => _inner.UnsubscribeAsync(projectIds, token),
            ct);
    }

    public Task<WorkItem> CreateWorkItemAsync(
        CreateWorkItemRequest request,
        CancellationToken ct = default)
    {
        return ExecuteWithResilienceAsync(
            token => _inner.CreateWorkItemAsync(request, token),
            ct);
    }

    public Task<WorkItem> UpdateWorkItemAsync(
        UpdateWorkItemRequest request,
        CancellationToken ct = default)
    {
        return ExecuteWithResilienceAsync(
            token => _inner.UpdateWorkItemAsync(request, token),
            ct);
    }

    public Task DeleteWorkItemAsync(Guid workItemId, CancellationToken ct = default)
    {
        return ExecuteWithResilienceAsync(
            async token =>
            {
                await _inner.DeleteWorkItemAsync(workItemId, token);
                return true;
            },
            ct);
    }

    public Task<IReadOnlyList<WorkItem>> GetWorkItemsAsync(
        Guid projectId,
        DateTime? since = null,
        CancellationToken ct = default)
    {
        return ExecuteWithResilienceAsync(
            token => _inner.GetWorkItemsAsync(projectId, since, token),
            ct);
    }

    public Task<Project> CreateProjectAsync(
        CreateProjectRequest request,
        CancellationToken ct = default)
    {
        return ExecuteWithResilienceAsync(
            token => _inner.CreateProjectAsync(request, token),
            ct);
    }

    public Task<Project> UpdateProjectAsync(
        UpdateProjectRequest request,
        CancellationToken ct = default)
    {
        return ExecuteWithResilienceAsync(
            token => _inner.UpdateProjectAsync(request, token),
            ct);
    }

    public Task DeleteProjectAsync(Guid projectId, int expectedVersion, CancellationToken ct = default)
    {
        return ExecuteWithResilienceAsync(
            async token =>
            {
                await _inner.DeleteProjectAsync(projectId, expectedVersion, token);
                return true;
            },
            ct);
    }

    public Task<IReadOnlyList<Project>> GetProjectsAsync(CancellationToken ct = default)
    {
        return ExecuteWithResilienceAsync(
            token => _inner.GetProjectsAsync(token),
            ct);
    }

    public ValueTask DisposeAsync()
    {
        return _inner.DisposeAsync();
    }

    private async Task<T> ExecuteWithResilienceAsync<T>(
        Func<CancellationToken, Task<T>> operation,
        CancellationToken ct)
    {
        return await _circuitBreaker.ExecuteAsync(
            token => _retryPolicy.ExecuteAsync(operation, token),
            ct);
    }

    private async Task ExecuteWithResilienceAsync(
        Func<CancellationToken, Task> operation,
        CancellationToken ct)
    {
        await _circuitBreaker.ExecuteAsync(
            async token =>
            {
                await _retryPolicy.ExecuteAsync(
                    async t =>
                    {
                        await operation(t);
                        return true;
                    },
                    token);
                return true;
            },
            ct);
    }
}