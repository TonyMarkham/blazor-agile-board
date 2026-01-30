using Microsoft.Extensions.Logging;
using Radzen;

namespace ProjectManagement.Services.Notifications;

/// <summary>
/// Production-grade toast service with queue management
/// </summary>
public sealed class ToastService : IToastService, IDisposable
{
    private readonly INotificationService _radzen;
    private readonly ILogger<ToastService> _logger;
    private readonly IToastScheduler _scheduler;
    private readonly object _lock = new();

    public static class Defaults
    {
        public const int SuccessDurationMs = 3000;
        public const int ErrorDurationMs = 5000;
        public const int WarningDurationMs = 4000;
        public const int InfoDurationMs = 3000;
        public const int ActionDurationMs = 5000;
        public const int MaxConcurrentToasts = 3;
        public const int MaxActionToasts = 2;
    }

    private int _activeCount;
    public int ActiveCount => _activeCount;
    internal Task? LastDecrementTask { get; private set; }

    private readonly List<ActionToastRequest> _actionToasts = new();
    public IReadOnlyList<ActionToastRequest> ActionToasts => _actionToasts.AsReadOnly();
    public event Action? OnActionToastsChanged;

    public ToastService(INotificationService radzen, ILogger<ToastService> logger, IToastScheduler scheduler)
    {
        _radzen = radzen ?? throw new ArgumentNullException(nameof(radzen));
        _logger = logger ?? throw new ArgumentNullException(nameof(logger));
        _scheduler = scheduler ?? throw new ArgumentNullException(nameof(scheduler));
    }

    public void ShowSuccess(string message, string? title = null, int? durationMs = null)
    {
        ArgumentNullException.ThrowIfNull(message);
        Show(NotificationSeverity.Success, message, title ?? "Success", durationMs ?? Defaults.SuccessDurationMs);
    }

    public void ShowError(string message, string? title = null, int? durationMs = null)
    {
        ArgumentNullException.ThrowIfNull(message);
        _logger.LogWarning("User error displayed: {Message}", message);

        _radzen.Notify(new NotificationMessage
        {
            Severity = NotificationSeverity.Error,
            Summary = title ?? "Error",
            Detail = message,
            Duration = durationMs ?? Defaults.ErrorDurationMs,
            CloseOnClick = true,
        });
    }

    public void ShowWarning(string message, string? title = null, int? durationMs = null)
    {
        ArgumentNullException.ThrowIfNull(message);
        Show(NotificationSeverity.Warning, message, title ?? "Warning", durationMs ?? Defaults.WarningDurationMs);
    }

    public void ShowInfo(string message, string? title = null, int? durationMs = null)
    {
        ArgumentNullException.ThrowIfNull(message);
        Show(NotificationSeverity.Info, message, title ?? "Info", durationMs ?? Defaults.InfoDurationMs);
    }

    public void ShowWithAction(string message, string actionText, Func<Task> onAction, int durationMs = 5000)
    {
        ArgumentNullException.ThrowIfNull(message);
        ArgumentNullException.ThrowIfNull(actionText);
        ArgumentNullException.ThrowIfNull(onAction);

        ActionToastRequest request;
        lock (_lock)
        {
            if (_actionToasts.Count >= Defaults.MaxActionToasts)
            {
                _logger.LogDebug("Action toast suppressed (queue full): {Message}", message);
                return;
            }

            request = new ActionToastRequest
            {
                Message = message,
                ActionText = actionText,
                OnAction = onAction,
                DurationMs = durationMs
            };
            _actionToasts.Add(request);
        }

        OnActionToastsChanged?.Invoke();
    }

    public void DismissActionToast(Guid id)
    {
        lock (_lock)
        {
            var index = _actionToasts.FindIndex(t => t.Id == id);
            if (index >= 0)
                _actionToasts.RemoveAt(index);
        }

        OnActionToastsChanged?.Invoke();
    }

    public void Clear()
    {
        lock (_lock)
        {
            _activeCount = 0;
            _actionToasts.Clear();
        }

        OnActionToastsChanged?.Invoke();
    }

    private void Show(NotificationSeverity severity, string message, string title, int durationMs)
    {
        lock (_lock)
        {
            if (_activeCount >= Defaults.MaxConcurrentToasts)
            {
                _logger.LogDebug("Toast suppressed (queue full): {Message}", message);
                return;
            }
            _activeCount++;
        }

        _radzen.Notify(new NotificationMessage
        {
            Severity = severity,
            Summary = title,
            Detail = message,
            Duration = durationMs,
            CloseOnClick = true,
        });

        LastDecrementTask = DecrementAfterDelay(durationMs);
    }

    private async Task DecrementAfterDelay(int delayMs)
    {
        await _scheduler.DelayAsync(delayMs).ConfigureAwait(false);
        lock (_lock)
        {
            _activeCount = Math.Max(0, _activeCount - 1);
        }
    }

    public void Dispose()
    {
        Clear();
    }
}
