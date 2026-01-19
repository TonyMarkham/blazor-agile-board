# Session 20.3: Resilience Patterns

**Parent Plan**: [session-20-plan.md](session-20-plan.md)
**Target**: ~30k tokens
**Prerequisites**: Session 20.2 complete (WebSocket client foundation)

---

## Scope

**Goal**: Circuit breaker, retry policy, reconnection service

**Estimated Tokens**: ~30k

## Context: Types Available from Previous Sessions

**From Session 20.1 (ProjectManagement.Core):**
- `ProjectManagement.Core.Models.*` - WorkItem, Sprint, CreateWorkItemRequest, UpdateWorkItemRequest, FieldChange, ConnectionState
- `ProjectManagement.Core.Interfaces.*` - IWebSocketClient, IConnectionHealth, IWorkItemStore, ISprintStore
- `ProjectManagement.Core.Exceptions.*` - ConnectionException, RequestTimeoutException, ServerRejectedException, ValidationException, VersionConflictException, CircuitOpenException

**From Session 20.2 (ProjectManagement.Services):**
- `ProjectManagement.Services.WebSocket.WebSocketOptions`
- `ProjectManagement.Services.WebSocket.WebSocketClient`
- `ProjectManagement.Services.WebSocket.ConnectionHealthTracker`

**Protobuf namespace convention:**
- `using Pm = ProjectManagement.Core.Proto;`

---

### Phase 3.1: Circuit Breaker

```csharp
// CircuitBreakerOptions.cs
namespace ProjectManagement.Services.Resilience;

/// <summary>
/// Configuration for circuit breaker behavior.
/// </summary>
public sealed class CircuitBreakerOptions
{
    /// <summary>Number of failures before opening circuit.</summary>
    public int FailureThreshold { get; set; } = 5;

    /// <summary>Duration to keep circuit open before testing.</summary>
    public TimeSpan OpenDuration { get; set; } = TimeSpan.FromSeconds(30);

    /// <summary>Successes needed in half-open to close circuit.</summary>
    public int HalfOpenSuccessThreshold { get; set; } = 3;

    /// <summary>Window for counting failures.</summary>
    public TimeSpan FailureWindow { get; set; } = TimeSpan.FromSeconds(60);
}
```

```csharp
// CircuitState.cs
namespace ProjectManagement.Services.Resilience;

/// <summary>
/// States for the circuit breaker pattern.
/// </summary>
public enum CircuitState
{
    /// <summary>Normal operation - requests allowed.</summary>
    Closed,

    /// <summary>Circuit tripped - requests blocked.</summary>
    Open,

    /// <summary>Testing recovery - limited requests allowed.</summary>
    HalfOpen
}
```

```csharp
// CircuitBreaker.cs
using System.IO;
using System.Net.WebSockets;
using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Options;
using ProjectManagement.Core.Exceptions;

namespace ProjectManagement.Services.Resilience;

/// <summary>
/// Circuit breaker to prevent cascading failures.
/// Matches backend pm-ws circuit breaker behavior.
/// </summary>
public sealed class CircuitBreaker
{
    private readonly CircuitBreakerOptions _options;
    private readonly ILogger<CircuitBreaker> _logger;
    private readonly object _lock = new();

    private CircuitState _state = CircuitState.Closed;
    private int _failureCount;
    private int _successCount;
    private DateTime _lastFailureTime = DateTime.MinValue;
    private DateTime _openedAt = DateTime.MinValue;

    public CircuitState State
    {
        get { lock (_lock) return _state; }
    }

    public TimeSpan? RetryAfter
    {
        get
        {
            lock (_lock)
            {
                if (_state != CircuitState.Open)
                    return null;

                var elapsed = DateTime.UtcNow - _openedAt;
                var remaining = _options.OpenDuration - elapsed;
                return remaining > TimeSpan.Zero ? remaining : TimeSpan.Zero;
            }
        }
    }

    public CircuitBreaker(
        IOptions<CircuitBreakerOptions> options,
        ILogger<CircuitBreaker> logger)
    {
        _options = options.Value;
        _logger = logger;
    }

    /// <summary>
    /// Check if a request should be allowed through.
    /// </summary>
    public bool AllowRequest()
    {
        lock (_lock)
        {
            switch (_state)
            {
                case CircuitState.Closed:
                    return true;

                case CircuitState.Open:
                    // Check if we should transition to half-open
                    if (DateTime.UtcNow - _openedAt >= _options.OpenDuration)
                    {
                        _state = CircuitState.HalfOpen;
                        _successCount = 0;
                        _logger.LogInformation("Circuit breaker transitioning to HalfOpen");
                        return true;
                    }
                    return false;

                case CircuitState.HalfOpen:
                    return true;

                default:
                    return false;
            }
        }
    }

    /// <summary>
    /// Record a successful operation.
    /// </summary>
    public void RecordSuccess()
    {
        lock (_lock)
        {
            switch (_state)
            {
                case CircuitState.Closed:
                    _failureCount = 0;
                    break;

                case CircuitState.HalfOpen:
                    _successCount++;
                    if (_successCount >= _options.HalfOpenSuccessThreshold)
                    {
                        _state = CircuitState.Closed;
                        _failureCount = 0;
                        _logger.LogInformation(
                            "Circuit breaker closed after {Successes} successes",
                            _successCount);
                    }
                    break;
            }
        }
    }

    /// <summary>
    /// Record a failed operation.
    /// </summary>
    public void RecordFailure()
    {
        lock (_lock)
        {
            var now = DateTime.UtcNow;

            // Reset count if outside failure window
            if (now - _lastFailureTime > _options.FailureWindow)
            {
                _failureCount = 0;
            }

            _lastFailureTime = now;
            _failureCount++;

            switch (_state)
            {
                case CircuitState.Closed:
                    if (_failureCount >= _options.FailureThreshold)
                    {
                        _state = CircuitState.Open;
                        _openedAt = now;
                        _logger.LogWarning(
                            "Circuit breaker OPEN after {Failures} failures",
                            _failureCount);
                    }
                    break;

                case CircuitState.HalfOpen:
                    _state = CircuitState.Open;
                    _openedAt = now;
                    _logger.LogWarning(
                        "Circuit breaker reopened due to failure in HalfOpen state");
                    break;
            }
        }
    }

    /// <summary>
    /// Execute an operation with circuit breaker protection.
    /// </summary>
    public async Task<T> ExecuteAsync<T>(
        Func<CancellationToken, Task<T>> operation,
        CancellationToken ct = default)
    {
        if (!AllowRequest())
        {
            throw new CircuitOpenException(RetryAfter ?? _options.OpenDuration);
        }

        try
        {
            var result = await operation(ct);
            RecordSuccess();
            return result;
        }
        catch (Exception ex)
        {
            // Record failure for transient errors, but always re-throw
            if (ShouldRecordAsFailure(ex))
            {
                RecordFailure();
            }
            throw;
        }
    }

    private static bool ShouldRecordAsFailure(Exception ex)
    {
        // Don't count validation errors or cancellation as circuit failures
        return ex is not ValidationException
            && ex is not OperationCanceledException
            && ex is not VersionConflictException;
    }
}
```

### Phase 3.2: Retry Policy

```csharp
// RetryPolicyOptions.cs
namespace ProjectManagement.Services.Resilience;

/// <summary>
/// Configuration for retry policy behavior.
/// </summary>
public sealed class RetryPolicyOptions
{
    public int MaxAttempts { get; set; } = 3;
    public TimeSpan InitialDelay { get; set; } = TimeSpan.FromMilliseconds(100);
    // Aligned with pm-config DEFAULT_MAX_DELAY_SECS = 5
    public TimeSpan MaxDelay { get; set; } = TimeSpan.FromSeconds(5);
    public double BackoffMultiplier { get; set; } = 2.0;
}
```

```csharp
// RetryPolicy.cs
using System.IO;
using System.Net.WebSockets;
using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Options;
using ProjectManagement.Core.Exceptions;

namespace ProjectManagement.Services.Resilience;

/// <summary>
/// Retry policy with exponential backoff and jitter.
/// </summary>
public sealed class RetryPolicy
{
    private readonly RetryPolicyOptions _options;
    private readonly ILogger<RetryPolicy> _logger;
    // Note: Random.Shared is thread-safe on .NET 6+ (our target)

    public RetryPolicy(
        IOptions<RetryPolicyOptions> options,
        ILogger<RetryPolicy> logger)
    {
        _options = options.Value;
        _logger = logger;
    }

    /// <summary>
    /// Execute an operation with retry logic.
    /// </summary>
    public async Task<T> ExecuteAsync<T>(
        Func<CancellationToken, Task<T>> operation,
        CancellationToken ct = default)
    {
        var attempt = 0;
        var delay = _options.InitialDelay;

        while (true)
        {
            attempt++;

            try
            {
                return await operation(ct);
            }
            catch (Exception ex) when (ShouldRetry(ex, attempt))
            {
                var jitteredDelay = AddJitter(delay);

                _logger.LogWarning(
                    ex,
                    "Attempt {Attempt}/{MaxAttempts} failed. Retrying in {Delay}ms",
                    attempt,
                    _options.MaxAttempts,
                    jitteredDelay.TotalMilliseconds);

                await Task.Delay(jitteredDelay, ct);

                delay = TimeSpan.FromMilliseconds(
                    Math.Min(
                        delay.TotalMilliseconds * _options.BackoffMultiplier,
                        _options.MaxDelay.TotalMilliseconds));
            }
        }
    }

    private bool ShouldRetry(Exception ex, int attempt)
    {
        if (attempt >= _options.MaxAttempts)
            return false;

        // Don't retry non-transient errors
        return ex is ConnectionException
            or RequestTimeoutException
            or IOException
            or WebSocketException;
    }

    private static TimeSpan AddJitter(TimeSpan delay)
    {
        // Add up to 25% jitter to prevent thundering herd
        var jitterFactor = 1.0 + (Random.Shared.NextDouble() * 0.25);
        return TimeSpan.FromMilliseconds(delay.TotalMilliseconds * jitterFactor);
    }
}
```

### Phase 3.3: Reconnection Service

```csharp
// ReconnectionOptions.cs
namespace ProjectManagement.Services.Resilience;

/// <summary>
/// Configuration for automatic reconnection behavior.
/// Note: Reconnection is a client-only concern (not present in pm-config).
/// These values are tuned for desktop/WebSocket UX, not server-side retry policy.
/// </summary>
public sealed class ReconnectionOptions
{
    public int MaxAttempts { get; set; } = 10;
    public TimeSpan InitialDelay { get; set; } = TimeSpan.FromSeconds(1);
    public TimeSpan MaxDelay { get; set; } = TimeSpan.FromSeconds(30);
}
```

```csharp
// ReconnectionService.cs
using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Options;
using ProjectManagement.Core.Exceptions;
using ProjectManagement.Core.Interfaces;
using ProjectManagement.Core.Models;

namespace ProjectManagement.Services.Resilience;

/// <summary>
/// Handles automatic reconnection with exponential backoff.
/// Rehydrates subscriptions after successful reconnect.
/// </summary>
public sealed class ReconnectionService : IDisposable
{
    private readonly IWebSocketClient _client;
    private readonly ILogger<ReconnectionService> _logger;
    private readonly ReconnectionOptions _options;

    // Track subscribed project IDs for rehydration after reconnect
    private readonly HashSet<Guid> _subscribedProjects = new();
    private readonly object _subscriptionLock = new();

    private CancellationTokenSource? _reconnectCts;
    private Task? _reconnectTask;
    private readonly SemaphoreSlim _lock = new(1, 1);
    private bool _disposed;

    public event Action<int>? OnReconnecting;
    public event Action? OnReconnected;
    public event Action<Exception>? OnReconnectFailed;

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
    /// Register a project subscription for rehydration after reconnect.
    /// Call this whenever the client subscribes to a project.
    /// </summary>
    public void TrackSubscription(Guid projectId)
    {
        lock (_subscriptionLock)
        {
            _subscribedProjects.Add(projectId);
        }
    }

    /// <summary>
    /// Remove a project subscription from rehydration tracking.
    /// Call this when the client unsubscribes from a project.
    /// </summary>
    public void UntrackSubscription(Guid projectId)
    {
        lock (_subscriptionLock)
        {
            _subscribedProjects.Remove(projectId);
        }
    }

    /// <summary>
    /// Get currently tracked subscriptions.
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

    private void HandleStateChanged(ConnectionState state)
    {
        if (state == ConnectionState.Reconnecting && _reconnectTask == null)
        {
            _ = StartReconnectionAsync();
        }
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

    public void Dispose()
    {
        if (_disposed) return;
        _disposed = true;

        _client.OnStateChanged -= HandleStateChanged;
        _reconnectCts?.Cancel();
        _reconnectCts?.Dispose();
        _lock.Dispose();
    }
}
```

### Phase 3.4: Resilient WebSocket Client Wrapper

```csharp
// ResilientWebSocketClient.cs
using Microsoft.Extensions.Logging;
using ProjectManagement.Core.Interfaces;
using ProjectManagement.Core.Models;
using ProjectManagement.Services.WebSocket;

namespace ProjectManagement.Services.Resilience;

/// <summary>
/// WebSocket client wrapper with circuit breaker and retry protection.
/// </summary>
public sealed class ResilientWebSocketClient : IWebSocketClient
{
    private readonly WebSocketClient _inner;
    private readonly CircuitBreaker _circuitBreaker;
    private readonly RetryPolicy _retryPolicy;
    private readonly ILogger<ResilientWebSocketClient> _logger;

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

    public Task ConnectAsync(CancellationToken ct = default)
        => _inner.ConnectAsync(ct);

    public Task DisconnectAsync(CancellationToken ct = default)
        => _inner.DisconnectAsync(ct);

    public Task SubscribeAsync(IEnumerable<Guid> projectIds, CancellationToken ct = default)
        => ExecuteWithResilienceAsync(
            token => _inner.SubscribeAsync(projectIds, token),
            ct);

    public Task UnsubscribeAsync(IEnumerable<Guid> projectIds, CancellationToken ct = default)
        => ExecuteWithResilienceAsync(
            token => _inner.UnsubscribeAsync(projectIds, token),
            ct);

    public Task<WorkItem> CreateWorkItemAsync(
        CreateWorkItemRequest request,
        CancellationToken ct = default)
        => ExecuteWithResilienceAsync(
            token => _inner.CreateWorkItemAsync(request, token),
            ct);

    public Task<WorkItem> UpdateWorkItemAsync(
        UpdateWorkItemRequest request,
        CancellationToken ct = default)
        => ExecuteWithResilienceAsync(
            token => _inner.UpdateWorkItemAsync(request, token),
            ct);

    public Task DeleteWorkItemAsync(Guid workItemId, CancellationToken ct = default)
        => ExecuteWithResilienceAsync(
            async token =>
            {
                await _inner.DeleteWorkItemAsync(workItemId, token);
                return true;
            },
            ct);

    public Task<IReadOnlyList<WorkItem>> GetWorkItemsAsync(
        Guid projectId,
        DateTime? since = null,
        CancellationToken ct = default)
        => ExecuteWithResilienceAsync(
            token => _inner.GetWorkItemsAsync(projectId, since, token),
            ct);

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

    public ValueTask DisposeAsync() => _inner.DisposeAsync();
}
```

### Files Summary for Sub-Session 20.3

| File | Purpose |
|------|---------|
| `CircuitBreakerOptions.cs` | Circuit breaker configuration |
| `CircuitState.cs` | Circuit breaker state enum |
| `CircuitBreaker.cs` | Circuit breaker pattern |
| `RetryPolicyOptions.cs` | Retry configuration |
| `RetryPolicy.cs` | Retry with exponential backoff |
| `ReconnectionOptions.cs` | Reconnection configuration |
| `ReconnectionService.cs` | Automatic reconnection |
| `ResilientWebSocketClient.cs` | Wrapper combining resilience patterns |
| **Total** | **8 files** |

### Success Criteria for 20.3

- [ ] Circuit breaker opens after configured failures
- [ ] Circuit breaker transitions through Closed -> Open -> HalfOpen -> Closed
- [ ] Retry policy uses exponential backoff with jitter
- [ ] Reconnection service handles disconnects
- [ ] All resilience patterns are thread-safe

---
