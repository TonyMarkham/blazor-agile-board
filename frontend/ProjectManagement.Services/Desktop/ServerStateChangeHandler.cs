using Microsoft.Extensions.Logging;
using Microsoft.JSInterop;

namespace ProjectManagement.Services.Desktop;

/// <summary>
/// Handler for server state change callbacks from JavaScript.
/// </summary>
internal sealed class ServerStateChangeHandler
{
    private readonly Func<string, Task> _callback;
    private readonly ILogger _logger;

    public ServerStateChangeHandler(
        Func<string, Task> callback,
        ILogger logger)
    {
        _callback = callback;
        _logger = logger;
    }

    [JSInvokable]
    public async Task OnStateChanged(string state)
    {
        _logger.LogInformation("Server state changed: {State}", state);
        try
        {
            await _callback(state);
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Error in server state change callback");
        }
    }
}