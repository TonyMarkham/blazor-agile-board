  using System.Net.WebSockets;
  using ProjectManagement.Services.WebSocket;

  namespace ProjectManagement.Services.Tests.Mocks;

  /// <summary>
  /// Mock WebSocket connection for testing WebSocketClient.
  /// </summary>
  internal sealed class MockWebSocketConnection : IWebSocketConnection
  {
      private readonly Queue<byte[]> _messagesToReceive = new();
      private readonly List<byte[]> _sentMessages = new();

      private WebSocketState _state = WebSocketState.None;
      private bool _disposed;

      public WebSocketState State => _state;
      public IReadOnlyList<byte[]> SentMessages => _sentMessages;

      public void EnqueueMessageToReceive(byte[] message)
      {
          _messagesToReceive.Enqueue(message);
      }

      public void SimulateServerClose()
      {
          _state = WebSocketState.CloseReceived;
      }

      public Task ConnectAsync(Uri uri, CancellationToken ct = default)
      {
          if (_disposed)
              throw new ObjectDisposedException(nameof(MockWebSocketConnection));

          _state = WebSocketState.Open;
          return Task.CompletedTask;
      }

      public ValueTask SendAsync(ReadOnlyMemory<byte> buffer, CancellationToken ct = default)
      {
          if (_disposed)
              throw new ObjectDisposedException(nameof(MockWebSocketConnection));

          if (_state != WebSocketState.Open)
              throw new InvalidOperationException("WebSocket not open");

          _sentMessages.Add(buffer.ToArray());
          return ValueTask.CompletedTask;
      }

      public ValueTask<ValueWebSocketReceiveResult> ReceiveAsync(Memory<byte> buffer, CancellationToken ct =
          default)
      {
          if (_disposed)
              throw new ObjectDisposedException(nameof(MockWebSocketConnection));

          if (_state == WebSocketState.CloseReceived)
          {
              return ValueTask.FromResult(new ValueWebSocketReceiveResult(
                  0,
                  WebSocketMessageType.Close,
                  true));
          }

          if (_messagesToReceive.Count == 0)
          {
              return new ValueTask<ValueWebSocketReceiveResult>(
                  WaitForMessageAsync(buffer, ct));
          }

          var message = _messagesToReceive.Dequeue();
          message.CopyTo(buffer);

          return ValueTask.FromResult(new ValueWebSocketReceiveResult(
              message.Length,
              WebSocketMessageType.Binary,
              true));
      }

      public Task CloseAsync(WebSocketCloseStatus status, string? description, CancellationToken ct = default)
      {
          _state = WebSocketState.Closed;
          return Task.CompletedTask;
      }

      private async Task<ValueWebSocketReceiveResult> WaitForMessageAsync(Memory<byte> buffer,
          CancellationToken ct)
      {
          try
          {
              await Task.Delay(Timeout.Infinite, ct);
          }
          catch (OperationCanceledException)
          {
              throw;
          }

          return new ValueWebSocketReceiveResult(0, WebSocketMessageType.Binary, true);
      }

      public ValueTask DisposeAsync()
      {
          _disposed = true;
          _state = WebSocketState.Closed;
          return ValueTask.CompletedTask;
      }
  }