namespace ProjectManagement.Services.Desktop;

public interface IDesktopConfigService
{
    Task<bool> IsDesktopModeAsync();
    Task<string> GetWebSocketUrlAsync(CancellationToken ct = default);
    Task<string> WaitForServerAsync(TimeSpan timeout, CancellationToken ct = default);
}