using Bunit;
using FluentAssertions;
using Microsoft.Extensions.DependencyInjection;
using Moq;
using ProjectManagement.Components.Activity;
using ProjectManagement.Core.Interfaces;
using ProjectManagement.Core.Models;
using Radzen;
using Xunit;

namespace ProjectManagement.Components.Tests.Activity;

public class ActivityFeedIntegrationTests : BunitContext
{
    public ActivityFeedIntegrationTests()
    {
        Services.AddScoped<DialogService>();
        Services.AddScoped<NotificationService>();
        Services.AddScoped<TooltipService>();
        Services.AddScoped<ContextMenuService>();

        Services.AddLogging();
        JSInterop.Mode = JSRuntimeMode.Loose;
    }

    [Fact]
    public void ActivityFeed_PrependsEntry_OnRealtimeEvent()
    {
        var client = new Mock<IWebSocketClient>();
        var entityId = Guid.NewGuid();

        client.Setup(c => c.GetActivityLogAsync(
                It.IsAny<GetActivityLogRequest>(),
                It.IsAny<CancellationToken>()))
            .ReturnsAsync(new ActivityLogPage
            {
                Entries = new List<ActivityLog>
                {
                    new()
                    {
                        Id = Guid.NewGuid(),
                        EntityType = "work_item",
                        EntityId = entityId,
                        Action = "created",
                        Timestamp = DateTime.UtcNow
                    }
                },
                HasMore = false,
                TotalCount = 1
            });

        Services.AddSingleton(client.Object);

        var cut = Render<ActivityFeed>(p => p
            .Add(x => x.EntityType, "work_item")
            .Add(x => x.EntityId, entityId)
            .Add(x => x.PageSize, 20));

        cut.WaitForAssertion(() =>
            cut.FindAll(".activity-item").Count.Should().Be(1));

        var incoming = new ActivityLog
        {
            Id = Guid.NewGuid(),
            EntityType = "work_item",
            EntityId = entityId,
            Action = "updated",
            Timestamp = DateTime.UtcNow
        };

        client.Raise(c => c.OnActivityLogCreated += null, incoming);

        cut.WaitForAssertion(() =>
            cut.FindAll(".activity-item").Count.Should().Be(2));
    }
}
