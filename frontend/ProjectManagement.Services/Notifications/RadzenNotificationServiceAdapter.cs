using Radzen;

namespace ProjectManagement.Services.Notifications;

public sealed class RadzenNotificationServiceAdapter : INotificationService
{
    private readonly NotificationService _radzen;

    public RadzenNotificationServiceAdapter(NotificationService radzen)
    {
        _radzen = radzen ?? throw new ArgumentNullException(nameof(radzen));
    }

    public void Notify(NotificationMessage message)
    {
        _radzen.Notify(message);
    }
}