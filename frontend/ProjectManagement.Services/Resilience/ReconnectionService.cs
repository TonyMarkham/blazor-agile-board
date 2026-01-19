using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Options;
using ProjectManagement.Core.Exceptions;
using ProjectManagement.Core.Interfaces;

namespace ProjectManagement.Services.Resilience;

/// <summary>
///     Handles automatic reconnection with exponential backoff.
///     Rehydrates subscriptions after successful reconnect.
/// </summary>
public sealed class ReconnectionService : IDisposable
{
    private readonly IWebSocketClient _client;
    private readonly SemaphoreSlim _lock = new(1, 1);
    private readonly ILogger<ReconnectionService> _logger;
    private readonly ReconnectionOptions _options;

    // Track subscribed project IDs for rehydration after reconnect
    private readonly HashSet<Guid> _subscribedProjects = new();
    private readonly object _subscriptionLock = new();
    private bool _disposed;

    private CancellationTokenSource? _reconnectCts;
    private Task? _reconnectTask;

    public ReconnectionService(
        IWebSocketClient client,
        IOptions<ReconnectionOptions> options,
        ILogger<ReconnectionService> logger)
    {
        _client = client;
        _options = options.Value;
        _logger = logger;

        _client.OnStateChanged += HandleStateChanged;
    }

    /// <summary>
    ///     Get currently tracked subscriptions.
    /// </summary>
    public IReadOnlyList<Guid> TrackedSubscriptions
    {
        get
        {
            lock (_subscriptionLock)
            {
                return _subscribedProjects.ToList();
            }
        }
    }

    public void Dispose()
    {
        if (_disposed) return;
        _disposed = true;

        _client.OnStateChanged -= HandleStateChanged;
        _reconnectCts?.Cancel();
        _reconnectCts?.Dispose();
        _lock.Dispose();
    }

    public event Action<int>? OnReconnecting;
    public event Action? OnReconnected;
    public event Action<Exception>? OnReconnectFailed;

    /// <summary>
    ///     Register a project subscription for rehydration after reconnect.
    ///     Call this whenever the client subscribes to a project.
    /// </summary>
    public void TrackSubscription(Guid projectId)
    {
        lock (_subscriptionLock)
        {
            _subscribedProjects.Add(projectId);
        }
    }

    /// <summary>
    ///     Remove a project subscription from rehydration tracking.
    ///     Call this when the client unsubscribes from a project.
    /// </summary>
    public void UntrackSubscription(Guid projectId)
    {
        lock (_subscriptionLock)
        {
            _subscribedProjects.Remove(projectId);
        }
    }

    private void HandleStateChanged(ConnectionState state)
    {
        if (state == ConnectionState.Reconnecting && _reconnectTask == null) _ = StartReconnectionAsync();
    }

    private async Task StartReconnectionAsync()
    {
        await _lock.WaitAsync();
        try
        {
            if (_reconnectTask != null || _disposed)
                return;

            _reconnectCts = new CancellationTokenSource();
            _reconnectTask = ReconnectLoopAsync(_reconnectCts.Token);
        }
        finally
        {
            _lock.Release();
        }
    }

    private async Task ReconnectLoopAsync(CancellationToken ct)
    {
        var attempt = 0;
        var delay = _options.InitialDelay;

        while (!ct.IsCancellationRequested && attempt < _options.MaxAttempts)
        {
            attempt++;
            OnReconnecting?.Invoke(attempt);

            _logger.LogInformation(
                "Reconnection attempt {Attempt}/{MaxAttempts}",
                attempt,
                _options.MaxAttempts);

            try
            {
                await _client.ConnectAsync(ct);

                // Rehydrate subscriptions after successful reconnect
                var subscriptions = TrackedSubscriptions;
                if (subscriptions.Count > 0)
                {
                    _logger.LogInformation(
                        "Rehydrating {Count} project subscriptions",
                        subscriptions.Count);
                    await _client.SubscribeAsync(subscriptions, ct);
                }

                _logger.LogInformation("Reconnected successfully");
                OnReconnected?.Invoke();

                await _lock.WaitAsync(ct);
                try
                {
                    _reconnectTask = null;
                }
                finally
                {
                    _lock.Release();
                }

                return;
            }
            catch (Exception ex) when (ex is not OperationCanceledException)
            {
                _logger.LogWarning(
                    ex,
                    "Reconnection attempt {Attempt} failed",
                    attempt);

                if (attempt < _options.MaxAttempts)
                {
                    var jitteredDelay = AddJitter(delay);
                    await Task.Delay(jitteredDelay, ct);
                    delay = TimeSpan.FromMilliseconds(
                        Math.Min(
                            delay.TotalMilliseconds * 2,
                            _options.MaxDelay.TotalMilliseconds));
                }
            }
        }

        _logger.LogError(
            "Failed to reconnect after {MaxAttempts} attempts",
            _options.MaxAttempts);

        OnReconnectFailed?.Invoke(
            new ConnectionException($"Failed to reconnect after {attempt} attempts"));

        await _lock.WaitAsync(ct);
        try
        {
            _reconnectTask = null;
        }
        finally
        {
            _lock.Release();
        }
    }

    private static TimeSpan AddJitter(TimeSpan delay)
    {
        var jitter = Random.Shared.NextDouble() * 0.25;
        return TimeSpan.FromMilliseconds(delay.TotalMilliseconds * (1 + jitter));
    }
}