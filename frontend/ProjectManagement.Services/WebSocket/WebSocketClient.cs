using System.Collections.Concurrent;
using System.Net.WebSockets;
using Google.Protobuf;
using Microsoft.Extensions.Logging;
using Microsoft.Extensions.Options;
using ProjectManagement.Core.Converters;
using ProjectManagement.Core.Exceptions;
using ProjectManagement.Core.Interfaces;
using ProjectManagement.Core.Models;
using ProjectManagement.Core.Validation;
using System.Web;

namespace ProjectManagement.Services.WebSocket;

using Pm = Core.Proto;

public sealed class WebSocketClient : IWebSocketClient
{
    private readonly Func<IWebSocketConnection> _connectionFactory;

    // Validators (injected for testability)                                                                                                                    
    private readonly IValidator<CreateWorkItemRequest> _createValidator;
    private readonly ConnectionHealthTracker _health;
    private readonly ILogger<WebSocketClient> _logger;
    private readonly WebSocketOptions _options;
    private readonly ConcurrentDictionary<string, PendingRequest> _pendingRequests = new();
    private readonly SemaphoreSlim _sendLock = new(1, 1);
    private readonly SemaphoreSlim _stateLock = new(1, 1);
    private readonly IValidator<UpdateWorkItemRequest> _updateValidator;

    private IWebSocketConnection? _connection;
    private bool _disposed;
    private CancellationTokenSource? _heartbeatCts;
    private Task? _heartbeatTask;
    private CancellationTokenSource? _receiveCts;
    private Task? _receiveTask;
    private HashSet<Guid> _subscribedProjects = new();
    
    // Query parameter names
    private const string QueryParamUserId = "user_id";
    private const string QueryParamToken = "token";

    public WebSocketClient(
        IOptions<WebSocketOptions> options,
        ILogger<WebSocketClient> logger,
        ILoggerFactory loggerFactory,
        IValidator<CreateWorkItemRequest> createValidator,
        IValidator<UpdateWorkItemRequest> updateValidator)
        : this(options, logger, loggerFactory, createValidator, updateValidator, null)
    {
    }

    internal WebSocketClient(
        IOptions<WebSocketOptions> options,
        ILogger<WebSocketClient> logger,
        ILoggerFactory loggerFactory,
        IValidator<CreateWorkItemRequest> createValidator,
        IValidator<UpdateWorkItemRequest> updateValidator,
        Func<IWebSocketConnection>? connectionFactory)
    {
        _createValidator = createValidator;
        _updateValidator = updateValidator;
        _options = options.Value;
        _logger = logger;
        _connectionFactory = connectionFactory ?? (() =>
            new BrowserWebSocketConnection(
                loggerFactory.CreateLogger<BrowserWebSocketConnection>()));
        _health = new ConnectionHealthTracker();
    }

    public ConnectionState State { get; private set; } = ConnectionState.Disconnected;

    public IConnectionHealth Health => _health;

    public event Action<ConnectionState>? OnStateChanged;
    public event Action<WorkItem>? OnWorkItemCreated;
    public event Action<WorkItem, IReadOnlyList<FieldChange>>? OnWorkItemUpdated;
    public event Action<Guid>? OnWorkItemDeleted;
    public event Action<Project>? OnProjectCreated;
    public event Action<Project, IReadOnlyList<FieldChange>>? OnProjectUpdated;
    public event Action<Guid>? OnProjectDeleted;

    public async Task ConnectAsync(CancellationToken ct = default)
    {
        ThrowIfDisposed();

        await _stateLock.WaitAsync(ct);
        try
        {
            if (State == ConnectionState.Connected)
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
    
    /// <summary>
    /// Connects to WebSocket server with explicit user identity.
    /// Used in desktop mode where JWT authentication is disabled.
    /// </summary>
    public async Task ConnectAsync(Guid userId, CancellationToken ct = default)
    {
        ThrowIfDisposed();

        if (userId == Guid.Empty)
        {
            throw new ArgumentException("User ID cannot be empty", nameof(userId));
        }

        await _stateLock.WaitAsync(ct);
        try
        {
            if (State == ConnectionState.Connected)
                return;

            SetState(ConnectionState.Connecting);

            _connection = _connectionFactory();
            var uri = BuildConnectionUri(userId);

            await _connection.ConnectAsync(uri, ct);

            _receiveCts = new CancellationTokenSource();
            _heartbeatCts = new CancellationTokenSource();

            _receiveTask = ReceiveLoopAsync(_receiveCts.Token);
            _heartbeatTask = HeartbeatLoopAsync(_heartbeatCts.Token);

            _health.RecordConnected();
            SetState(ConnectionState.Connected);

            _logger.LogInformation("WebSocket connected to {Uri} with user {UserId}", uri, userId);
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

    /// <summary>
    /// Reconnect to a new server URL (desktop mode only).
    /// Disconnects from current server, updates URL, and reconnects.
    /// </summary>
    /// <param name="newServerUrl">New WebSocket server URL (e.g., ws://127.0.0.1:54321/ws)</param>
    /// <param name="ct">Cancellation token</param>
    public async Task ReconnectAsync(string newServerUrl, CancellationToken ct = default)
    {
        ThrowIfDisposed();

        if (string.IsNullOrWhiteSpace(newServerUrl))
        {
            throw new ArgumentException("Server URL cannot be null or empty", nameof(newServerUrl));
        }

        _logger.LogInformation("Reconnecting to new server URL: {NewUrl}", newServerUrl);

        await _stateLock.WaitAsync(ct);
        try
        {
            // Disconnect from current server
            if (State != ConnectionState.Disconnected && State != ConnectionState.Closed)
            {
                _logger.LogInformation("Disconnecting from current server: {OldUrl}", _options.ServerUrl);
                await DisconnectInternalAsync(ct);
            }

            // Update server URL
            var oldUrl = _options.ServerUrl;
            _options.ServerUrl = newServerUrl;
            _logger.LogInformation("Server URL updated: {OldUrl} -> {NewUrl}", oldUrl, newServerUrl);

            // Clear any failed connection state
            _health.RecordDisconnected();
            SetState(ConnectionState.Disconnected);
        }
        finally
        {
            _stateLock.Release();
        }

        // Reconnect (releases lock, so call outside try-finally)
        _logger.LogInformation("Reconnecting to new server...");
        await ConnectAsync(ct);
    }

    public Task SubscribeAsync(IEnumerable<Guid> projectIds, CancellationToken ct = default)
    {
        // TODO: Implement subscription when backend handler is ready
        _logger.LogDebug("Subscribe called for projects: {ProjectIds}", string.Join(", ", projectIds));
        return Task.CompletedTask;
    }

    public Task UnsubscribeAsync(IEnumerable<Guid> projectIds, CancellationToken ct = default)
    {
        // TODO: Implement unsubscription when backend handler is ready
        _logger.LogDebug("Unsubscribe called for projects: {ProjectIds}", string.Join(", ", projectIds));
        return Task.CompletedTask;
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
                ProjectId = request.ProjectId.ToString(),
                Status = request.Status,
                Priority = request.Priority
            }
        };

        if (!string.IsNullOrEmpty(request.Description))
            message.CreateWorkItemRequest.Description = request.Description;
        if (request.ParentId.HasValue)
            message.CreateWorkItemRequest.ParentId = request.ParentId.Value.ToString();

        var response = await SendRequestAsync(message, ct);

        if (response.PayloadCase == Pm.WebSocketMessage.PayloadOneofCase.Error)
            throw new ServerRejectedException(
                response.Error.Code,
                response.Error.Message,
                response.Error.Field);

        if (response.PayloadCase != Pm.WebSocketMessage.PayloadOneofCase.WorkItemCreated)
            throw new InvalidOperationException(
                $"Unexpected response type: {response.PayloadCase}");

        return ProtoConverter.ToDomain(response.WorkItemCreated.WorkItem);
    }

    public async Task<WorkItem> UpdateWorkItemAsync(UpdateWorkItemRequest request, CancellationToken ct = default)
    {
        ThrowIfDisposed();
        EnsureConnected();

        // Validate using injected validator
        _updateValidator.Validate(request).ThrowIfInvalid();

        var message = new Pm.WebSocketMessage
        {
            MessageId = Guid.NewGuid().ToString(),
            Timestamp = DateTimeOffset.UtcNow.ToUnixTimeSeconds(),
            UpdateWorkItemRequest = new Pm.UpdateWorkItemRequest
            {
                WorkItemId = request.WorkItemId.ToString(),
                ExpectedVersion = request.ExpectedVersion
            }
        };

        // Only set fields that are being updated
        if (!string.IsNullOrEmpty(request.Title))
            message.UpdateWorkItemRequest.Title = request.Title;
        if (!string.IsNullOrEmpty(request.Description))
            message.UpdateWorkItemRequest.Description = request.Description;
        if (!string.IsNullOrEmpty(request.Status))
            message.UpdateWorkItemRequest.Status = request.Status;
        if (!string.IsNullOrEmpty(request.Priority))
            message.UpdateWorkItemRequest.Priority = request.Priority;
        if (request.AssigneeId != null)
            message.UpdateWorkItemRequest.AssigneeId = request.AssigneeId.ToString() ?? string.Empty;
        if (request.SprintId != null)
            message.UpdateWorkItemRequest.SprintId = request.SprintId.ToString() ?? string.Empty;
        if (request.Position.HasValue)
            message.UpdateWorkItemRequest.Position = request.Position.Value;
        if (request.StoryPoints.HasValue)
            message.UpdateWorkItemRequest.StoryPoints = request.StoryPoints.Value;

        var response = await SendRequestAsync(message, ct);

        if (response.PayloadCase == Pm.WebSocketMessage.PayloadOneofCase.Error)
            throw new ServerRejectedException(
                response.Error.Code,
                response.Error.Message,
                response.Error.Field);

        if (response.PayloadCase != Pm.WebSocketMessage.PayloadOneofCase.WorkItemUpdated)
            throw new InvalidOperationException(
                $"Unexpected response type: {response.PayloadCase}");

        return ProtoConverter.ToDomain(response.WorkItemUpdated.WorkItem);
    }

    public async Task DeleteWorkItemAsync(Guid workItemId, CancellationToken ct = default)
    {
        ThrowIfDisposed();
        EnsureConnected();

        var message = new Pm.WebSocketMessage
        {
            MessageId = Guid.NewGuid().ToString(),
            Timestamp = DateTimeOffset.UtcNow.ToUnixTimeSeconds(),
            DeleteWorkItemRequest = new Pm.DeleteWorkItemRequest
            {
                WorkItemId = workItemId.ToString()
            }
        };

        var response = await SendRequestAsync(message, ct);

        if (response.PayloadCase == Pm.WebSocketMessage.PayloadOneofCase.Error)
            throw new ServerRejectedException(
                response.Error.Code,
                response.Error.Message,
                response.Error.Field);

        if (response.PayloadCase != Pm.WebSocketMessage.PayloadOneofCase.WorkItemDeleted)
            throw new InvalidOperationException(
                $"Unexpected response type: {response.PayloadCase}");

        // Delete successful - no return value needed
    }

    public async Task<IReadOnlyList<WorkItem>> GetWorkItemsAsync(Guid projectId, DateTime? since = null,
        CancellationToken ct = default)
    {
        ThrowIfDisposed();
        EnsureConnected();

        var message = new Pm.WebSocketMessage
        {
            MessageId = Guid.NewGuid().ToString(),
            Timestamp = DateTimeOffset.UtcNow.ToUnixTimeSeconds(),
            GetWorkItemsRequest = new Pm.GetWorkItemsRequest
            {
                ProjectId = projectId.ToString()
            }
        };

        if (since.HasValue)
            message.GetWorkItemsRequest.SinceTimestamp = new DateTimeOffset(since.Value).ToUnixTimeSeconds();

        var response = await SendRequestAsync(message, ct);

        if (response.PayloadCase == Pm.WebSocketMessage.PayloadOneofCase.Error)
            throw new ServerRejectedException(
                response.Error.Code,
                response.Error.Message,
                response.Error.Field);

        if (response.PayloadCase != Pm.WebSocketMessage.PayloadOneofCase.WorkItemsList)
            throw new InvalidOperationException(
                $"Unexpected response type: {response.PayloadCase}");

        return response.WorkItemsList.WorkItems
            .Select(ProtoConverter.ToDomain)
            .ToList();
    }

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

    public async Task<Project> CreateProjectAsync(CreateProjectRequest request, CancellationToken ct = default)
    {
        ThrowIfDisposed();
        EnsureConnected();

        var message = new Pm.WebSocketMessage
        {
            MessageId = Guid.NewGuid().ToString(),
            Timestamp = DateTimeOffset.UtcNow.ToUnixTimeSeconds(),
            CreateProjectRequest = ProtoConverter.ToProto(request),
        };

        var response = await SendRequestAsync(message, ct);

        if (response.PayloadCase == Pm.WebSocketMessage.PayloadOneofCase.Error)
            throw new ServerRejectedException(response.Error.Code, response.Error.Message);

        if (response.PayloadCase != Pm.WebSocketMessage.PayloadOneofCase.ProjectCreated)
            throw new InvalidOperationException($"Unexpected response type: {response.PayloadCase}");

        return ProtoConverter.ToDomain(response.ProjectCreated.Project);
    }

    public async Task<Project> UpdateProjectAsync(UpdateProjectRequest request, CancellationToken ct = default)
    {
        ThrowIfDisposed();
        EnsureConnected();

        var message = new Pm.WebSocketMessage
        {
            MessageId = Guid.NewGuid().ToString(),
            Timestamp = DateTimeOffset.UtcNow.ToUnixTimeSeconds(),
            UpdateProjectRequest = ProtoConverter.ToProto(request),
        };

        var response = await SendRequestAsync(message, ct);

        if (response.PayloadCase == Pm.WebSocketMessage.PayloadOneofCase.Error)
            throw new ServerRejectedException(response.Error.Code, response.Error.Message);

        if (response.PayloadCase != Pm.WebSocketMessage.PayloadOneofCase.ProjectUpdated)
            throw new InvalidOperationException($"Unexpected response type: {response.PayloadCase}");

        return ProtoConverter.ToDomain(response.ProjectUpdated.Project);
    }

    public async Task DeleteProjectAsync(Guid projectId, int expectedVersion, CancellationToken ct = default)
    {
        ThrowIfDisposed();
        EnsureConnected();

        var message = new Pm.WebSocketMessage
        {
            MessageId = Guid.NewGuid().ToString(),
            Timestamp = DateTimeOffset.UtcNow.ToUnixTimeSeconds(),
            DeleteProjectRequest = ProtoConverter.ToDeleteProjectProto(projectId, expectedVersion),
        };

        var response = await SendRequestAsync(message, ct);

        if (response.PayloadCase == Pm.WebSocketMessage.PayloadOneofCase.Error)
            throw new ServerRejectedException(response.Error.Code, response.Error.Message);

        if (response.PayloadCase != Pm.WebSocketMessage.PayloadOneofCase.ProjectDeleted)
            throw new InvalidOperationException($"Unexpected response type: {response.PayloadCase}");
    }

    public async Task<IReadOnlyList<Project>> GetProjectsAsync(CancellationToken ct = default)
    {
        ThrowIfDisposed();
        EnsureConnected();

        var message = new Pm.WebSocketMessage
        {
            MessageId = Guid.NewGuid().ToString(),
            Timestamp = DateTimeOffset.UtcNow.ToUnixTimeSeconds(),
            ListProjectsRequest = new Pm.ListProjectsRequest(),
        };

        var response = await SendRequestAsync(message, ct);

        if (response.PayloadCase == Pm.WebSocketMessage.PayloadOneofCase.Error)
            throw new ServerRejectedException(response.Error.Code, response.Error.Message);

        if (response.PayloadCase != Pm.WebSocketMessage.PayloadOneofCase.ProjectList)
            throw new InvalidOperationException($"Unexpected response type: {response.PayloadCase}");

        return response.ProjectList.Projects.Select(ProtoConverter.ToDomain).ToList();
    }

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
            throw new InvalidOperationException(
                $"Duplicate message ID: {request.MessageId}");

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
                    OnWorkItemDeleted?.Invoke(deletedId);

                break;

            case Pm.WebSocketMessage.PayloadOneofCase.ProjectCreated:
                var createdProject = ProtoConverter.ToDomain(message.ProjectCreated.Project);
                OnProjectCreated?.Invoke(createdProject);
                break;

            case Pm.WebSocketMessage.PayloadOneofCase.ProjectUpdated:
                var updatedProject = ProtoConverter.ToDomain(message.ProjectUpdated.Project);
                var projectChanges = message.ProjectUpdated.Changes
                    .Select(c => new FieldChange(c.FieldName, c.OldValue, c.NewValue))
                    .ToList();
                OnProjectUpdated?.Invoke(updatedProject, projectChanges);
                break;

            case Pm.WebSocketMessage.PayloadOneofCase.ProjectDeleted:
                if (Guid.TryParse(message.ProjectDeleted.ProjectId, out var deletedProjectId))
                    OnProjectDeleted?.Invoke(deletedProjectId);
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
            try
            {
                await Task.Delay(_options.HeartbeatInterval, ct);

                if (State != ConnectionState.Connected)
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

    private async Task HandleDisconnectAsync(Exception? ex)
    {
        await _stateLock.WaitAsync();
        try
        {
            if (State == ConnectionState.Closed || State == ConnectionState.Reconnecting)
                return;

            _health.RecordDisconnected();
            SetState(ConnectionState.Reconnecting);

            // Fail all pending requests
            foreach (var pending in _pendingRequests.Values)
                pending.Fail(new ConnectionException(
                    "Connection lost", ex ?? new Exception("Unknown error")));

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

    private Uri BuildConnectionUri(Guid? userId = null)
    {
        var uriBuilder = new UriBuilder(_options.ServerUrl);
        var query = HttpUtility.ParseQueryString(uriBuilder.Query);

        // Add user_id if provided (desktop mode)
        if (userId.HasValue && userId.Value != Guid.Empty)
        {
            query[QueryParamUserId] = userId.Value.ToString();
        }

        // Add JWT token if available (web mode)
        if (!string.IsNullOrEmpty(_options.JwtToken))
        {
            query[QueryParamToken] = Uri.EscapeDataString(_options.JwtToken);
        }

        uriBuilder.Query = query.ToString();
        return uriBuilder.Uri;
    }

    private void SetState(ConnectionState newState)
    {
        if (State == newState) return;

        var oldState = State;
        State = newState;

        _logger.LogInformation("Connection state changed: {Old} -> {New}", oldState, newState);
        OnStateChanged?.Invoke(newState);
    }

    private void EnsureConnected()
    {
        if (State != ConnectionState.Connected)
            throw new ConnectionException("Not connected to server")
            {
                LastKnownState = State
            };
    }

    private void ThrowIfDisposed()
    {
        ObjectDisposedException.ThrowIf(_disposed, this);
    }

    #endregion
}