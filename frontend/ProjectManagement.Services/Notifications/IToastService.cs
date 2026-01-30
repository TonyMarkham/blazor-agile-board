namespace ProjectManagement.Services.Notifications;

/// <summary>
/// Toast notification service for user feedback
/// </summary>
public interface IToastService
{
    void ShowSuccess(string message, string? title = null, int? durationMs = null);
    void ShowError(string message, string? title = null, int? durationMs = null);
    void ShowWarning(string message, string? title = null, int? durationMs = null);
    void ShowInfo(string message, string? title = null, int? durationMs = null);

    void ShowWithAction(string message, string actionText, Func<Task> onAction, int durationMs = 5000);

    void Clear();

    int ActiveCount { get; }

    IReadOnlyList<ActionToastRequest> ActionToasts { get; }
    event Action? OnActionToastsChanged;
    void DismissActionToast(Guid id);
}