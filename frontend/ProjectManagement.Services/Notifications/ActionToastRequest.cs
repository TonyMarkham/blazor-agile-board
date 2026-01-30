namespace ProjectManagement.Services.Notifications;

/// <summary>
/// Represents a toast with an action button (e.g., "Undo")
/// </summary>
public sealed record ActionToastRequest
{
    public Guid Id { get; init; } = Guid.NewGuid();
    public string Message { get; init; } = string.Empty;
    public string ActionText { get; init; } = string.Empty;
    public Func<Task> OnAction { get; init; } = () => Task.CompletedTask;
    public int DurationMs { get; init; } = 5000;
}