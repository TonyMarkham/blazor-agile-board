using System.Net.WebSockets;
using System.Threading.Channels;
using FluentAssertions;
using Google.Protobuf;
using Microsoft.Extensions.Logging.Abstractions;
using Microsoft.Extensions.Options;
using ProjectManagement.Core.Models;
using ProjectManagement.Core.Validation;
using ProjectManagement.Services.WebSocket;
using Xunit;

namespace ProjectManagement.Services.Tests.WebSocket;

using Pm = ProjectManagement.Core.Proto;

public class WebSocketClientActivityLogTests
{
    [Fact]
    public async Task GetActivityLogAsync_SendsRequest_AndMapsResponse()
    {
        var connection = new ScriptedWebSocketConnection();
        var client = CreateClient(connection);

        connection.OnSend = request =>
        {
            request.PayloadCase.Should().Be(Pm.WebSocketMessage.PayloadOneofCase.GetActivityLogRequest);
            request.GetActivityLogRequest.EntityType.Should().Be("work_item");
            request.GetActivityLogRequest.EntityId.Should().NotBeNullOrEmpty();
            request.GetActivityLogRequest.Limit.Should().Be(25);
            request.GetActivityLogRequest.Offset.Should().Be(10);

            var response = new Pm.WebSocketMessage
            {
                MessageId = request.MessageId,
                Timestamp = request.Timestamp,
                ActivityLogList = new Pm.ActivityLogList
                {
                    TotalCount = 1,
                    HasMore = false
                }
            };

            response.ActivityLogList.Entries.Add(new Pm.ActivityLogEntry
            {
                Id = Guid.NewGuid().ToString(),
                EntityType = "work_item",
                EntityId = request.GetActivityLogRequest.EntityId,
                Action = "updated",
                UserId = Guid.NewGuid().ToString(),
                Timestamp = DateTimeOffset.UtcNow.ToUnixTimeSeconds()
            });

            return response;
        };

        await client.ConnectAsync();

        var result = await client.GetActivityLogAsync(new GetActivityLogRequest
        {
            EntityType = "work_item",
            EntityId = Guid.NewGuid(),
            Limit = 25,
            Offset = 10
        });

        result.TotalCount.Should().Be(1);
        result.HasMore.Should().BeFalse();
        result.Entries.Should().HaveCount(1);
    }

    [Fact]
    public async Task ActivityLogCreated_Broadcast_RaisesEvent()
    {
        var connection = new ScriptedWebSocketConnection();
        var client = CreateClient(connection);

        var tcs = new TaskCompletionSource<ActivityLog>(
            TaskCreationOptions.RunContinuationsAsynchronously);

        client.OnActivityLogCreated += entry => tcs.TrySetResult(entry);

        await client.ConnectAsync();

        var msg = new Pm.WebSocketMessage
        {
            MessageId = Guid.NewGuid().ToString(),
            Timestamp = DateTimeOffset.UtcNow.ToUnixTimeSeconds(),
            ActivityLogCreated = new Pm.ActivityLogCreated
            {
                Entry = new Pm.ActivityLogEntry
                {
                    Id = Guid.NewGuid().ToString(),
                    EntityType = "project",
                    EntityId = Guid.NewGuid().ToString(),
                    Action = "created",
                    UserId = Guid.NewGuid().ToString(),
                    Timestamp = DateTimeOffset.UtcNow.ToUnixTimeSeconds()
                }
            }
        };

        connection.EnqueueIncoming(msg);

        var received = await tcs.Task.WaitAsync(TimeSpan.FromSeconds(1));
        received.Action.Should().Be("created");
        received.EntityType.Should().Be("project");
    }

    private static WebSocketClient CreateClient(ScriptedWebSocketConnection connection)
    {
        var options = Options.Create(new WebSocketOptions { ServerUrl = "ws://localhost:8000/ws" });

        return new WebSocketClient(
            options,
            NullLogger<WebSocketClient>.Instance,
            NullLoggerFactory.Instance,
            new CreateWorkItemRequestValidator(),
            new UpdateWorkItemRequestValidator(),
            () => connection);
    }

    private sealed class ScriptedWebSocketConnection : IWebSocketConnection
    {
        private readonly Channel<byte[]> _incoming = Channel.CreateUnbounded<byte[]>();
        private WebSocketState _state = WebSocketState.None;

        public List<byte[]> SentMessages { get; } = new();
        public Func<Pm.WebSocketMessage, Pm.WebSocketMessage?>? OnSend { get; set; }

        public WebSocketState State => _state;

        public Task ConnectAsync(Uri uri, CancellationToken ct)
        {
            _state = WebSocketState.Open;
            return Task.CompletedTask;
        }

        public ValueTask SendAsync(ReadOnlyMemory<byte> buffer, CancellationToken ct)
        {
            var bytes = buffer.ToArray();
            SentMessages.Add(bytes);

            if (OnSend is not null)
            {
                var request = Pm.WebSocketMessage.Parser.ParseFrom(bytes);
                var response = OnSend(request);

                if (response is not null)
                    _incoming.Writer.TryWrite(response.ToByteArray());
            }

            return ValueTask.CompletedTask;
        }

        public async ValueTask<ValueWebSocketReceiveResult> ReceiveAsync(
            Memory<byte> buffer,
            CancellationToken ct)
        {
            var data = await _incoming.Reader.ReadAsync(ct);
            data.CopyTo(buffer);
            return new ValueWebSocketReceiveResult(data.Length, WebSocketMessageType.Binary, true);
        }

        public Task CloseAsync(WebSocketCloseStatus status, string? description, CancellationToken ct)
        {
            _state = WebSocketState.Closed;
            return Task.CompletedTask;
        }

        public ValueTask DisposeAsync()
        {
            _state = WebSocketState.Closed;
            _incoming.Writer.TryComplete();
            return ValueTask.CompletedTask;
        }

        public void EnqueueIncoming(Pm.WebSocketMessage message)
        {
            _incoming.Writer.TryWrite(message.ToByteArray());
        }
    }
}
