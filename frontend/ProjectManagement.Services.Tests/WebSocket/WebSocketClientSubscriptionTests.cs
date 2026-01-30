using FluentAssertions;
using Microsoft.Extensions.Logging.Abstractions;
using Microsoft.Extensions.Options;
using ProjectManagement.Core.Validation;
using Pm = ProjectManagement.Core.Proto;
using ProjectManagement.Services.Tests.Mocks;
using ProjectManagement.Services.WebSocket;
using Xunit;

namespace ProjectManagement.Services.Tests.WebSocket;

public class WebSocketClientSubscriptionTests
{
    private static WebSocketClient CreateClient(MockWebSocketConnection connection)
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

    [Fact]
    public async Task SubscribeAsync_SendsSubscribeMessage()
    {
        var conn = new MockWebSocketConnection();
        var client = CreateClient(conn);
        await client.ConnectAsync();

        var id = Guid.NewGuid();
        await client.SubscribeAsync(new[] { id });

        var msg = Pm.WebSocketMessage.Parser.ParseFrom(conn.SentMessages.Last());
        msg.PayloadCase.Should().Be(Pm.WebSocketMessage.PayloadOneofCase.Subscribe);
        msg.Subscribe.ProjectIds.Should().Contain(id.ToString());
    }

    [Fact]
    public async Task UnsubscribeAsync_SendsUnsubscribeMessage()
    {
        var conn = new MockWebSocketConnection();
        var client = CreateClient(conn);
        await client.ConnectAsync();

        var id = Guid.NewGuid();
        await client.UnsubscribeAsync(new[] { id });

        var msg = Pm.WebSocketMessage.Parser.ParseFrom(conn.SentMessages.Last());
        msg.PayloadCase.Should().Be(Pm.WebSocketMessage.PayloadOneofCase.Unsubscribe);
        msg.Unsubscribe.ProjectIds.Should().Contain(id.ToString());
    }
}