using Radzen;

namespace ProjectManagement.Services.Notifications;

public interface INotificationService
{
    void Notify(NotificationMessage message);
}