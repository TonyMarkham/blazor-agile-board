namespace ProjectManagement.Services.Notifications;

public interface IToastScheduler
{
    Task DelayAsync(int delayMs, CancellationToken ct = default);
}