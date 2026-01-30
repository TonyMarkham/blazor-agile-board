using FluentAssertions;
using Microsoft.Extensions.Logging.Abstractions;
using ProjectManagement.Services.Notifications;
using Radzen;
using Xunit;

namespace ProjectManagement.Services.Tests.Notifications;

public class ToastServiceTests
{
    private sealed class FakeToastScheduler : IToastScheduler
    {
        private readonly Queue<TaskCompletionSource<bool>> _pending = new();
        public Task? LastDelayTask { get; private set; }

        public Task DelayAsync(int delayMs, CancellationToken ct = default)
        {
            var tcs = new TaskCompletionSource<bool>(TaskCreationOptions.RunContinuationsAsynchronously);
            if (ct.CanBeCanceled)
                ct.Register(() => tcs.TrySetCanceled(ct));
            _pending.Enqueue(tcs);
            LastDelayTask = tcs.Task;
            return tcs.Task;
        }

        public void CompleteNext()
        {
            if (_pending.Count == 0) return;
            _pending.Dequeue().TrySetResult(true);
        }
    }
    
    private sealed class FakeNotificationService : INotificationService
    {
        public int NotifyCount { get; private set; }

        public void Notify(NotificationMessage message)
        {
            NotifyCount++;
        }
    }


    private static (ToastService Sut, FakeNotificationService Radzen, FakeToastScheduler Scheduler) Create()
    {
        var radzen = new FakeNotificationService();
        var scheduler = new FakeToastScheduler();
        var sut = new ToastService(radzen, NullLogger<ToastService>.Instance, scheduler);
        return (sut, radzen, scheduler);
    }

    [Fact]
    public void ShowSuccess_RespectsQueueLimit()
    {
        var (sut, radzen, _) = Create();

        for (int i = 0; i < ToastService.Defaults.MaxConcurrentToasts + 1; i++)
            sut.ShowSuccess($"Message {i}");

        radzen.NotifyCount.Should().Be(ToastService.Defaults.MaxConcurrentToasts);
    }

    [Fact]
    public void ShowError_BypassesQueueLimit()
    {
        var (sut, radzen, _) = Create();

        for (int i = 0; i < ToastService.Defaults.MaxConcurrentToasts; i++)
            sut.ShowSuccess($"Message {i}");

        sut.ShowError("Critical");

        radzen.NotifyCount.Should().Be(ToastService.Defaults.MaxConcurrentToasts + 1);
    }

    [Fact]
    public void ActionToast_AddsAndDismisses()
    {
        var (sut, _, _) = Create();

        sut.ShowWithAction("Undo delete", "Undo", () => Task.CompletedTask);
        sut.ActionToasts.Should().HaveCount(1);

        var id = sut.ActionToasts.Single().Id;
        sut.DismissActionToast(id);
        sut.ActionToasts.Should().BeEmpty();
    }

    [Fact]
    public async Task ActiveCount_Decrements_WhenSchedulerCompletes()
    {
        var (sut, _, scheduler) = Create();

        sut.ShowSuccess("Saved");
        sut.ActiveCount.Should().Be(1);

        scheduler.CompleteNext();
        await sut.LastDecrementTask!;

        sut.ActiveCount.Should().Be(0);
    }
}
