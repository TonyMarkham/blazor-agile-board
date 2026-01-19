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