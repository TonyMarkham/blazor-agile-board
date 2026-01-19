namespace ProjectManagement.Services.WebSocket;

using System.Net.WebSockets;
using Microsoft.Extensions.Logging;

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