namespace ProjectManagement.Services.Notifications;

public sealed class ToastScheduler : IToastScheduler
{
    public Task DelayAsync(int delayMs, CancellationToken ct = default) => Task.Delay(delayMs, ct);
}