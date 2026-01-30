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

public class ActivityFeedTests : BunitContext
{
    public ActivityFeedTests()
    {
        // Radzen services used by child components
        Services.AddScoped<DialogService>();
        Services.AddScoped<NotificationService>();
        Services.AddScoped<TooltipService>();
        Services.AddScoped<ContextMenuService>();

        Services.AddLogging();
        JSInterop.Mode = JSRuntimeMode.Loose;
    }

    [Fact]
    public void ActivityFeed_ShowsSkeleton_WhileLoading()
    {
        var client = new Mock<IWebSocketClient>();
        var tcs = new TaskCompletionSource<ActivityLogPage>();

        client.Setup(c => c.GetActivityLogAsync(
                It.IsAny<GetActivityLogRequest>(),
                It.IsAny<CancellationToken>()))
            .Returns(tcs.Task);

        Services.AddSingleton(client.Object);

        var cut = Render<ActivityFeed>(p => p
            .Add(x => x.EntityType, "work_item")
            .Add(x => x.EntityId, Guid.NewGuid())
            .Add(x => x.PageSize, 20));

        cut.WaitForAssertion(() =>
            cut.FindAll(".activity-skeleton").Count.Should().Be(1));
    }

    [Fact]
    public void ActivityFeed_ShowsError_WhenLoadFails()
    {
        var client = new Mock<IWebSocketClient>();
        client.Setup(c => c.GetActivityLogAsync(
                It.IsAny<GetActivityLogRequest>(),
                It.IsAny<CancellationToken>()))
            .ThrowsAsync(new InvalidOperationException("boom"));

        Services.AddSingleton(client.Object);

        var cut = Render<ActivityFeed>(p => p
            .Add(x => x.EntityType, "work_item")
            .Add(x => x.EntityId, Guid.NewGuid())
            .Add(x => x.PageSize, 20));

        cut.WaitForAssertion(() =>
        {
            cut.FindAll(".activity-error").Count.Should().Be(1);
            cut.Markup.Should().Contain("Failed to load activity");
        });
    }

    [Fact]
    public void ActivityFeed_RendersEntries_WhenDataLoaded()
    {
        var client = new Mock<IWebSocketClient>();
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
                        EntityId = Guid.NewGuid(),
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
            .Add(x => x.EntityId, Guid.NewGuid())
            .Add(x => x.PageSize, 20));

        cut.WaitForAssertion(() =>
            cut.FindAll(".activity-item").Count.Should().Be(1));

        cut.Markup.Should().Contain("Created");
    }
}
