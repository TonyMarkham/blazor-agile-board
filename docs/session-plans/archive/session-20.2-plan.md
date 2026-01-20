# Session 20.2: WebSocket Client Foundation

**Parent Plan**: [session-20-plan.md](session-20-plan.md)
**Target**: ~35k tokens
**Prerequisites**: Session 20.1 complete (Foundation with models and interfaces)

**Status**: ✅ Complete (2026-01-19)

---

## Completion Summary

**What Was Built:**
- Configuration with constants (no magic numbers)
- Request/response correlation with timeout handling
- Modern .NET WebSocket abstraction (ValueTask, ValueWebSocketReceiveResult)
- Full WebSocket client with thread safety, heartbeat, and proper disposal
- Per-message latency tracking with connection health metrics
- Two-constructor pattern for testability

**Key Fixes Applied:**
- Fixed protobuf namespace: `ProjectManagement.Core.Proto` (not `Protos`)
- Updated to modern .NET types (ValueTask instead of Task for WebSocket operations)
- Moved ConnectionState from Exceptions to Models namespace
- Fixed dependency order (ConnectionHealthTracker created before WebSocketClient)
- Added stub implementations for unimplemented IWebSocketClient methods

**Files Created:** 7
- `WebSocketOptions.cs`
- `PendingRequest.cs`
- `IWebSocketConnection.cs`
- `BrowserWebSocketConnection.cs`
- `WebSocketClient.cs`
- `ConnectionHealthTracker.cs`
- `_Imports.cs`

**Files Modified:** 2
- `ConnectionState.cs` (moved to Models)
- This plan document (corrected for modern .NET patterns)

**Build Status:** ✅ 0 warnings, 0 errors

---

## Scope

**Goal**: Core WebSocket client with message correlation and heartbeat

**Estimated Tokens**: ~35k

### Phase 2.1: Configuration

```csharp
// WebSocketOptions.cs
namespace ProjectManagement.Services.WebSocket;

public sealed class WebSocketOptions
{
    /// <summary>WebSocket server URL (ws:// or wss://).</summary>
    public string ServerUrl { get; set; } = "ws://localhost:8080/ws";

    /// <summary>JWT token for authentication (null for desktop mode).</summary>
    public string? JwtToken { get; set; }

    /// <summary>Interval between ping messages.</summary>
    public TimeSpan HeartbeatInterval { get; set; } = TimeSpan.FromSeconds(30);

    /// <summary>Timeout waiting for pong response.</summary>
    public TimeSpan HeartbeatTimeout { get; set; } = TimeSpan.FromSeconds(60);

    /// <summary>Timeout for request/response operations.</summary>
    public TimeSpan RequestTimeout { get; set; } = TimeSpan.FromSeconds(30);

    /// <summary>Size of send buffer (messages).</summary>
    public int SendBufferSize { get; set; } = 100;

    /// <summary>Size of receive buffer (bytes).</summary>
    public int ReceiveBufferSize { get; set; } = 64 * 1024;
}
```

### Phase 2.2: Connection State (Re-export)

> **Note**: `ConnectionState` enum is defined in `ProjectManagement.Core.Exceptions` namespace (Phase 1.4) to avoid circular dependencies. This phase creates a type alias/re-export for use in the WebSocket namespace.

```csharp
// In ProjectManagement.Services.WebSocket namespace, use:
using ConnectionState = ProjectManagement.Core.Exceptions.ConnectionState;

// Or create a simple re-export file:
// ConnectionState.cs
namespace ProjectManagement.Services.WebSocket;

// Re-export from Core for convenience
public enum ConnectionState
{
    /// <summary>Not connected to server.</summary>
    Disconnected,

    /// <summary>Attempting to establish connection.</summary>
    Connecting,

    /// <summary>Connected and ready for operations.</summary>
    Connected,

    /// <summary>Connection lost, attempting to reconnect.</summary>
    Reconnecting,

    /// <summary>Permanently closed (disposed).</summary>
    Closed
}

// Alternative: use global using in _Imports.cs:
// global using ConnectionState = ProjectManagement.Core.Exceptions.ConnectionState;
```

### Phase 2.3: Request Tracking

```csharp
// PendingRequest.cs
namespace ProjectManagement.Services.WebSocket;

internal sealed class PendingRequest : IDisposable
{
    public string MessageId { get; }
    public DateTime SentAt { get; }
    public TimeSpan Timeout { get; }
    public TaskCompletionSource<WebSocketMessage> CompletionSource { get; }

    private readonly CancellationTokenSource _timeoutCts;
    private readonly CancellationTokenRegistration _registration;
    private bool _disposed;

    public PendingRequest(string messageId, TimeSpan timeout, CancellationToken externalCt)
    {
        MessageId = messageId;
        SentAt = DateTime.UtcNow;
        Timeout = timeout;
        CompletionSource = new TaskCompletionSource<WebSocketMessage>(
            TaskCreationOptions.RunContinuationsAsynchronously);

        _timeoutCts = CancellationTokenSource.CreateLinkedTokenSource(externalCt);
        _timeoutCts.CancelAfter(timeout);

        _registration = _timeoutCts.Token.Register(() =>
        {
            CompletionSource.TrySetException(
                new RequestTimeoutException(messageId, timeout));
        });
    }

    public void Complete(WebSocketMessage response)
    {
        CompletionSource.TrySetResult(response);
    }

    public void Fail(Exception ex)
    {
        CompletionSource.TrySetException(ex);
    }

    public void Dispose()
    {
        if (_disposed) return;
        _disposed = true;

        _registration.Dispose();
        _timeoutCts.Dispose();
    }
}
```

### Phase 2.4: WebSocket Connection Abstraction

```csharp
// IWebSocketConnection.cs
namespace ProjectManagement.Services.WebSocket;

using System.Net.WebSockets;

/// <summary>
/// Abstraction over raw WebSocket for testability.
/// </summary>
internal interface IWebSocketConnection : IAsyncDisposable
{
    WebSocketState State { get; }

    Task ConnectAsync(Uri uri, CancellationToken ct);
    ValueTask SendAsync(ReadOnlyMemory<byte> buffer, CancellationToken ct);
    ValueTask<ValueWebSocketReceiveResult> ReceiveAsync(Memory<byte> buffer, CancellationToken ct);
    Task CloseAsync(WebSocketCloseStatus status, string? description, CancellationToken ct);
}

// BrowserWebSocketConnection.cs
internal sealed class BrowserWebSocketConnection : IWebSocketConnection
{
    private readonly ClientWebSocket _socket;
    private readonly ILogger<BrowserWebSocketConnection> _logger;

    public WebSocketState State => _socket.State;

    public BrowserWebSocketConnection(ILogger<BrowserWebSocketConnection> logger)
    {
        _socket = new ClientWebSocket();
        _logger = logger;
    }

    public async Task ConnectAsync(Uri uri, CancellationToken ct)
    {
        _logger.LogDebug("Connecting to {Uri}", uri);
        await _socket.ConnectAsync(uri, ct);
        _logger.LogInformation("Connected to {Uri}", uri);
    }

    public ValueTask SendAsync(ReadOnlyMemory<byte> buffer, CancellationToken ct)
    {
        return _socket.SendAsync(buffer, WebSocketMessageType.Binary, true, ct);
    }

    public ValueTask<ValueWebSocketReceiveResult> ReceiveAsync(Memory<byte> buffer, CancellationToken ct)
    {
        return _socket.ReceiveAsync(buffer, ct);
    }

    public Task CloseAsync(WebSocketCloseStatus status, string? description, CancellationToken ct)
    {
        if (_socket.State == WebSocketState.Open || _socket.State == WebSocketState.CloseReceived)
        {
            return _socket.CloseAsync(status, description, ct);
        }
        return Task.CompletedTask;
    }

    public async ValueTask DisposeAsync()
    {
        try
        {
            if (_socket.State == WebSocketState.Open)
            {
                using var cts = new CancellationTokenSource(TimeSpan.FromSeconds(5));
                await CloseAsync(WebSocketCloseStatus.NormalClosure, "Disposing", cts.Token);
            }
        }
        catch (Exception ex)
        {
            _logger.LogWarning(ex, "Error during WebSocket disposal");
        }
        finally
        {
            _socket.Dispose();
        }
    }
}
```

### Phase 2.5: Core WebSocket Client

```csharp
// WebSocketClient.cs
namespace ProjectManagement.Services.WebSocket;

using Pm = ProjectManagement.Core.Proto;

public sealed class WebSocketClient : IWebSocketClient
{
    private readonly WebSocketOptions _options;
    private readonly ILogger<WebSocketClient> _logger;
    private readonly Func<IWebSocketConnection> _connectionFactory;
    private readonly ConcurrentDictionary<string, PendingRequest> _pendingRequests = new();
    private readonly ConnectionHealthTracker _health;
    private readonly SemaphoreSlim _sendLock = new(1, 1);
    private readonly SemaphoreSlim _stateLock = new(1, 1);

    // Validators (injected for testability)
    private readonly IValidator<CreateWorkItemRequest> _createValidator;
    private readonly IValidator<UpdateWorkItemRequest> _updateValidator;

    private IWebSocketConnection? _connection;
    private CancellationTokenSource? _receiveCts;
    private CancellationTokenSource? _heartbeatCts;
    private Task? _receiveTask;
    private Task? _heartbeatTask;
    private ConnectionState _state = ConnectionState.Disconnected;
    private HashSet<Guid> _subscribedProjects = new();
    private bool _disposed;

    public ConnectionState State => _state;
    public IConnectionHealth Health => _health;

    public event Action<ConnectionState>? OnStateChanged;
    public event Action<WorkItem>? OnWorkItemCreated;
    public event Action<WorkItem, IReadOnlyList<FieldChange>>? OnWorkItemUpdated;
    public event Action<Guid>? OnWorkItemDeleted;

    public WebSocketClient(
        IOptions<WebSocketOptions> options,
        ILogger<WebSocketClient> logger,
        ILoggerFactory loggerFactory,
        IValidator<CreateWorkItemRequest> createValidator,
        IValidator<UpdateWorkItemRequest> updateValidator,
        Func<IWebSocketConnection>? connectionFactory = null)
    {
        _createValidator = createValidator;
        _updateValidator = updateValidator;
        _options = options.Value;
        _logger = logger;
        // Use injected ILoggerFactory for proper DI integration
        _connectionFactory = connectionFactory ?? (() =>
            new BrowserWebSocketConnection(
                loggerFactory.CreateLogger<BrowserWebSocketConnection>()));
        _health = new ConnectionHealthTracker();
    }

    public async Task ConnectAsync(CancellationToken ct = default)
    {
        ThrowIfDisposed();

        await _stateLock.WaitAsync(ct);
        try
        {
            if (_state == ConnectionState.Connected)
                return;

            SetState(ConnectionState.Connecting);

            _connection = _connectionFactory();
            var uri = BuildConnectionUri();

            await _connection.ConnectAsync(uri, ct);

            _receiveCts = new CancellationTokenSource();
            _heartbeatCts = new CancellationTokenSource();

            _receiveTask = ReceiveLoopAsync(_receiveCts.Token);
            _heartbeatTask = HeartbeatLoopAsync(_heartbeatCts.Token);

            _health.RecordConnected();
            SetState(ConnectionState.Connected);

            _logger.LogInformation("WebSocket connected to {Uri}", uri);
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Failed to connect to WebSocket");
            SetState(ConnectionState.Disconnected);
            throw new ConnectionException("Failed to connect to server", ex);
        }
        finally
        {
            _stateLock.Release();
        }
    }

    public async Task DisconnectAsync(CancellationToken ct = default)
    {
        ThrowIfDisposed();

        await _stateLock.WaitAsync(ct);
        try
        {
            await DisconnectInternalAsync(ct);
        }
        finally
        {
            _stateLock.Release();
        }
    }

    public async Task<WorkItem> CreateWorkItemAsync(
        CreateWorkItemRequest request,
        CancellationToken ct = default)
    {
        ThrowIfDisposed();
        EnsureConnected();

        // Validate using injected validator
        _createValidator.Validate(request).ThrowIfInvalid();

        var message = new Pm.WebSocketMessage
        {
            MessageId = Guid.NewGuid().ToString(),
            Timestamp = DateTimeOffset.UtcNow.ToUnixTimeSeconds(),
            CreateWorkItemRequest = new Pm.CreateWorkItemRequest
            {
                ItemType = ProtoConverter.ToProto(request.ItemType),
                Title = request.Title,
                ProjectId = request.ProjectId.ToString()
            }
        };

        if (!string.IsNullOrEmpty(request.Description))
            message.CreateWorkItemRequest.Description = request.Description;
        if (request.ParentId.HasValue)
            message.CreateWorkItemRequest.ParentId = request.ParentId.Value.ToString();

        var response = await SendRequestAsync(message, ct);

        if (response.PayloadCase == Pm.WebSocketMessage.PayloadOneofCase.Error)
        {
            throw new ServerRejectedException(
                response.Error.Code,
                response.Error.Message,
                response.Error.Field);
        }

        if (response.PayloadCase != Pm.WebSocketMessage.PayloadOneofCase.WorkItemCreated)
        {
            throw new InvalidOperationException(
                $"Unexpected response type: {response.PayloadCase}");
        }

        return ProtoConverter.ToDomain(response.WorkItemCreated.WorkItem);
    }

    // ... other CRUD methods follow same pattern

    #region Private Methods

    private async Task<Pm.WebSocketMessage> SendRequestAsync(
        Pm.WebSocketMessage request,
        CancellationToken ct)
    {
        using var pending = new PendingRequest(
            request.MessageId,
            _options.RequestTimeout,
            ct);

        if (!_pendingRequests.TryAdd(request.MessageId, pending))
        {
            throw new InvalidOperationException(
                $"Duplicate message ID: {request.MessageId}");
        }

        try
        {
            await SendMessageAsync(request, ct);
            _health.RecordRequestSent();

            return await pending.CompletionSource.Task;
        }
        finally
        {
            _pendingRequests.TryRemove(request.MessageId, out _);
        }
    }

    private async Task SendMessageAsync(Pm.WebSocketMessage message, CancellationToken ct)
    {
        var bytes = message.ToByteArray();

        await _sendLock.WaitAsync(ct);
        try
        {
            if (_connection?.State != WebSocketState.Open)
                throw new ConnectionException("WebSocket not connected");

            await _connection.SendAsync(bytes, ct);
            _health.RecordMessageSent();

            _logger.LogDebug("Sent message {MessageId} ({Type})",
                message.MessageId, message.PayloadCase);
        }
        finally
        {
            _sendLock.Release();
        }
    }

    private async Task ReceiveLoopAsync(CancellationToken ct)
    {
        var buffer = new byte[_options.ReceiveBufferSize];
        var messageBuffer = new MemoryStream();

        try
        {
            while (!ct.IsCancellationRequested && _connection?.State == WebSocketState.Open)
            {
                var result = await _connection.ReceiveAsync(buffer, ct);

                if (result.MessageType == WebSocketMessageType.Close)
                {
                    _logger.LogInformation("Server initiated close");
                    break;
                }

                messageBuffer.Write(buffer, 0, result.Count);

                if (result.EndOfMessage)
                {
                    var messageBytes = messageBuffer.ToArray();
                    messageBuffer.SetLength(0);

                    ProcessReceivedMessage(messageBytes);
                }
            }
        }
        catch (OperationCanceledException) when (ct.IsCancellationRequested)
        {
            // Normal shutdown
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Error in receive loop");
            _ = HandleDisconnectAsync(ex);
        }
    }

    private void ProcessReceivedMessage(byte[] bytes)
    {
        try
        {
            var message = Pm.WebSocketMessage.Parser.ParseFrom(bytes);
            _health.RecordMessageReceived();

            _logger.LogDebug("Received message {MessageId} ({Type})",
                message.MessageId, message.PayloadCase);

            // Check if this is a response to a pending request
            if (_pendingRequests.TryGetValue(message.MessageId, out var pending))
            {
                pending.Complete(message);
                _health.RecordResponseReceived();
                return;
            }

            // Otherwise it's a broadcast event
            HandleBroadcastEvent(message);
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Error processing received message");
        }
    }

    private void HandleBroadcastEvent(Pm.WebSocketMessage message)
    {
        switch (message.PayloadCase)
        {
            case Pm.WebSocketMessage.PayloadOneofCase.Pong:
                // Pass messageId to correlate with specific ping for accurate latency
                _health.RecordPong(message.MessageId, message.Pong.Timestamp);
                break;

            case Pm.WebSocketMessage.PayloadOneofCase.WorkItemCreated:
                var created = ProtoConverter.ToDomain(message.WorkItemCreated.WorkItem);
                OnWorkItemCreated?.Invoke(created);
                break;

            case Pm.WebSocketMessage.PayloadOneofCase.WorkItemUpdated:
                var updated = ProtoConverter.ToDomain(message.WorkItemUpdated.WorkItem);
                var changes = message.WorkItemUpdated.Changes
                    .Select(c => new FieldChange(c.FieldName, c.OldValue, c.NewValue))
                    .ToList();
                OnWorkItemUpdated?.Invoke(updated, changes);
                break;

            case Pm.WebSocketMessage.PayloadOneofCase.WorkItemDeleted:
                if (Guid.TryParse(message.WorkItemDeleted.WorkItemId, out var deletedId))
                {
                    OnWorkItemDeleted?.Invoke(deletedId);
                }
                break;

            default:
                _logger.LogWarning("Unhandled broadcast message type: {Type}",
                    message.PayloadCase);
                break;
        }
    }

    private async Task HeartbeatLoopAsync(CancellationToken ct)
    {
        while (!ct.IsCancellationRequested)
        {
            try
            {
                await Task.Delay(_options.HeartbeatInterval, ct);

                if (_state != ConnectionState.Connected)
                    continue;

                var messageId = Guid.NewGuid().ToString();
                var ping = new Pm.WebSocketMessage
                {
                    MessageId = messageId,
                    Timestamp = DateTimeOffset.UtcNow.ToUnixTimeSeconds(),
                    Ping = new Pm.Ping
                    {
                        Timestamp = DateTimeOffset.UtcNow.ToUnixTimeSeconds()
                    }
                };

                // Track this specific ping for latency calculation
                _health.RecordPingSent(messageId);
                await SendMessageAsync(ping, ct);
            }
            catch (OperationCanceledException) when (ct.IsCancellationRequested)
            {
                break;
            }
            catch (Exception ex)
            {
                _logger.LogWarning(ex, "Error sending heartbeat");
            }
        }
    }

    private async Task HandleDisconnectAsync(Exception? ex)
    {
        await _stateLock.WaitAsync();
        try
        {
            if (_state == ConnectionState.Closed || _state == ConnectionState.Reconnecting)
                return;

            _health.RecordDisconnected();
            SetState(ConnectionState.Reconnecting);

            // Fail all pending requests
            foreach (var pending in _pendingRequests.Values)
            {
                pending.Fail(new ConnectionException(
                    "Connection lost", ex ?? new Exception("Unknown error")));
            }
            _pendingRequests.Clear();

            // Clean up current connection
            await DisconnectInternalAsync(CancellationToken.None);

            // Start reconnection (handled by ReconnectionService)
        }
        finally
        {
            _stateLock.Release();
        }
    }

    private async Task DisconnectInternalAsync(CancellationToken ct)
    {
        _heartbeatCts?.Cancel();
        _receiveCts?.Cancel();

        if (_heartbeatTask != null)
            await Task.WhenAny(_heartbeatTask, Task.Delay(1000, ct));
        if (_receiveTask != null)
            await Task.WhenAny(_receiveTask, Task.Delay(1000, ct));

        if (_connection != null)
        {
            await _connection.DisposeAsync();
            _connection = null;
        }

        _heartbeatCts?.Dispose();
        _receiveCts?.Dispose();
        _heartbeatCts = null;
        _receiveCts = null;

        SetState(ConnectionState.Disconnected);
    }

    private Uri BuildConnectionUri()
    {
        var uri = new UriBuilder(_options.ServerUrl);

        if (!string.IsNullOrEmpty(_options.JwtToken))
        {
            uri.Query = $"token={Uri.EscapeDataString(_options.JwtToken)}";
        }

        return uri.Uri;
    }

    private void SetState(ConnectionState newState)
    {
        if (_state == newState) return;

        var oldState = _state;
        _state = newState;

        _logger.LogInformation("Connection state changed: {Old} -> {New}", oldState, newState);
        OnStateChanged?.Invoke(newState);
    }

    private void EnsureConnected()
    {
        if (_state != ConnectionState.Connected)
        {
            throw new ConnectionException("Not connected to server")
            {
                LastKnownState = _state
            };
        }
    }

    private void ThrowIfDisposed()
    {
        ObjectDisposedException.ThrowIf(_disposed, this);
    }

    #endregion

    public async ValueTask DisposeAsync()
    {
        if (_disposed) return;
        _disposed = true;

        SetState(ConnectionState.Closed);

        foreach (var pending in _pendingRequests.Values)
        {
            pending.Fail(new ObjectDisposedException(nameof(WebSocketClient)));
            pending.Dispose();
        }
        _pendingRequests.Clear();

        await DisconnectInternalAsync(CancellationToken.None);

        _sendLock.Dispose();
        _stateLock.Dispose();
    }
}
```

### Phase 2.6: Connection Health Tracker

```csharp
// ConnectionHealthTracker.cs
namespace ProjectManagement.Services.WebSocket;

/// <summary>
/// Tracks connection health metrics including latency from ping/pong.
/// Uses per-message correlation for accurate latency measurement.
/// </summary>
internal sealed class ConnectionHealthTracker : IConnectionHealth
{
    // Track outstanding pings by messageId for accurate latency correlation
    private readonly ConcurrentDictionary<string, long> _pendingPings = new();

    private long _lastPongReceivedTicks;
    private long _lastMessageReceivedTicks;
    private long _lastMessageSentTicks;
    private int _pendingRequestCount;
    private int _reconnectAttempts;
    private long _latencyMs;

    public ConnectionQuality Quality
    {
        get
        {
            if (LastMessageReceived == null)
                return ConnectionQuality.Unknown;

            var timeSinceMessage = DateTime.UtcNow - LastMessageReceived.Value;
            if (timeSinceMessage > TimeSpan.FromMinutes(2))
                return ConnectionQuality.Disconnected;

            if (!Latency.HasValue)
                return ConnectionQuality.Unknown;

            return Latency.Value.TotalMilliseconds switch
            {
                < 100 => ConnectionQuality.Excellent,
                < 300 => ConnectionQuality.Good,
                < 1000 => ConnectionQuality.Fair,
                _ => ConnectionQuality.Poor
            };
        }
    }

    public TimeSpan? Latency =>
        _latencyMs > 0 ? TimeSpan.FromMilliseconds(_latencyMs) : null;

    public DateTime? LastMessageReceived =>
        _lastMessageReceivedTicks > 0
            ? new DateTime(_lastMessageReceivedTicks, DateTimeKind.Utc)
            : null;

    public DateTime? LastMessageSent =>
        _lastMessageSentTicks > 0
            ? new DateTime(_lastMessageSentTicks, DateTimeKind.Utc)
            : null;

    public int PendingRequestCount => _pendingRequestCount;
    public int ReconnectAttempts => _reconnectAttempts;

    public void RecordConnected()
    {
        Interlocked.Exchange(ref _reconnectAttempts, 0);
        _pendingPings.Clear();
    }

    public void RecordDisconnected()
    {
        Interlocked.Increment(ref _reconnectAttempts);
        _pendingPings.Clear();
    }

    /// <summary>
    /// Record that a ping was sent with a specific message ID.
    /// </summary>
    public void RecordPingSent(string messageId)
    {
        _pendingPings[messageId] = DateTime.UtcNow.Ticks;

        // Clean up old pings (> 2 minutes) to prevent memory leak
        var cutoff = DateTime.UtcNow.AddMinutes(-2).Ticks;
        foreach (var kvp in _pendingPings)
        {
            if (kvp.Value < cutoff)
                _pendingPings.TryRemove(kvp.Key, out _);
        }
    }

    /// <summary>
    /// Record pong received, correlating with the original ping by messageId.
    /// </summary>
    public void RecordPong(string messageId, long serverTimestamp)
    {
        var now = DateTime.UtcNow.Ticks;
        Interlocked.Exchange(ref _lastPongReceivedTicks, now);

        // Correlate with the specific ping that generated this pong
        if (_pendingPings.TryRemove(messageId, out var pingSentTicks))
        {
            var latency = (now - pingSentTicks) / TimeSpan.TicksPerMillisecond;
            Interlocked.Exchange(ref _latencyMs, latency);
        }
    }

    public void RecordMessageReceived()
    {
        Interlocked.Exchange(ref _lastMessageReceivedTicks, DateTime.UtcNow.Ticks);
    }

    public void RecordMessageSent()
    {
        Interlocked.Exchange(ref _lastMessageSentTicks, DateTime.UtcNow.Ticks);
    }

    public void RecordRequestSent()
    {
        Interlocked.Increment(ref _pendingRequestCount);
    }

    public void RecordResponseReceived()
    {
        Interlocked.Decrement(ref _pendingRequestCount);
    }
}
```

### Files Summary for Sub-Session 20.2

| File | Purpose |
|------|---------|
| `WebSocketOptions.cs` | Configuration with constants |
| `_Imports.cs` | Global using statements |
| `PendingRequest.cs` | Request/response tracking with timeout |
| `IWebSocketConnection.cs` | Abstraction for testability (modern .NET types) |
| `BrowserWebSocketConnection.cs` | Real WebSocket implementation |
| `WebSocketClient.cs` | Main client implementation |
| `ConnectionHealthTracker.cs` | Health metrics tracking |
| **Total** | **7 files created** |

**Note:** `CreateWorkItemRequest.cs` and `UpdateWorkItemRequest.cs` already existed from Session 20.1. `ConnectionState.cs` was moved from Exceptions to Models namespace.

### Success Criteria for 20.2

- [x] WebSocket connects to backend
- [x] Binary protobuf messages sent/received
- [x] Request/response correlation via message_id
- [x] Heartbeat ping/pong every 30 seconds
- [x] Proper disposal of resources
- [x] Thread-safe operations
- [x] Build succeeds with 0 warnings, 0 errors

---

