using Microsoft.Extensions.Logging;

namespace ProjectManagement.Services.Desktop;

public sealed class DesktopConfigService : IDesktopConfigService, IAsyncDisposable
{
    private readonly ITauriService _tauriService;
    private readonly ILogger<DesktopConfigService> _logger;
    private string? _serverStateSubscriptionId;
    private TaskCompletionSource<bool>? _serverReadyTcs;

    // Server state constants
    private const string ServerStateRunning = "running";
    private const string ServerStateFailed = "failed";

    public DesktopConfigService(
        ITauriService tauriService,
        ILogger<DesktopConfigService> logger)
    {
        _tauriService = tauriService;
        _logger = logger;
    }

    public async Task<bool> IsDesktopModeAsync()
    {
        return await _tauriService.IsDesktopAsync();
    }

    public async Task<string> GetWebSocketUrlAsync(CancellationToken ct = default)
    {
        return await _tauriService.GetWebSocketUrlAsync(ct);
    }

    public async Task<string> WaitForServerAsync(TimeSpan timeout, CancellationToken ct = default)
    {
        using var timeoutCts = new CancellationTokenSource(timeout);
        using var linkedCts = CancellationTokenSource.CreateLinkedTokenSource(ct, timeoutCts.Token);

        _serverReadyTcs = new TaskCompletionSource<bool>(
            TaskCreationOptions.RunContinuationsAsynchronously);

        try
        {
            // STEP 1: Subscribe to events FIRST (before any status check)
            // This ensures we don't miss the server-ready event
            _serverStateSubscriptionId = await _tauriService.SubscribeToServerStateAsync(
                OnServerStateChangedAsync,
                linkedCts.Token);

            _logger.LogDebug("Subscribed to server state events");

            // STEP 2: Send "I'm ready" ping - get current status
            // This eliminates the race condition: we're already subscribed,
            // so if server starts between now and the response, we'll get the event
            var status = await _tauriService.NotifyReadyAsync(linkedCts.Token);
            _logger.LogDebug(
                "Received initial status: State={State}, Port={Port}, Pid={Pid}",
                status.State, status.Port, status.Pid);

            // STEP 3: Check if server is already running
            if (status.State == ServerStateRunning && status.IsHealthy && status.WebsocketUrl != null)
            {
                _logger.LogInformation(
                    "Server already running on port {Port} (pid={Pid})",
                    status.Port, status.Pid);
                return status.WebsocketUrl;
            }

            // STEP 4: Check for error state
            if (status.State == ServerStateFailed)
            {
                throw new InvalidOperationException(
                    status.Error ?? "Server failed to start");
            }

            // STEP 5: Wait for server-ready event
            _logger.LogDebug("Server not ready yet (state={State}), waiting for event...", status.State);

            using (linkedCts.Token.Register(() =>
                   {
                       if (timeoutCts.IsCancellationRequested)
                           _serverReadyTcs.TrySetException(new TimeoutException("Server startup timed out"));
                       else
                           _serverReadyTcs.TrySetCanceled(ct);
                   }))
            {
                await _serverReadyTcs.Task;

                // Get the WebSocket URL from final status
                var finalStatus = await _tauriService.GetServerStatusAsync(linkedCts.Token);

                if (finalStatus.WebsocketUrl == null)
                {
                    throw new InvalidOperationException("Server ready but WebSocket URL is null");
                }

                _logger.LogInformation(
                    "Server ready on port {Port} (pid={Pid})",
                    finalStatus.Port, finalStatus.Pid);
                return finalStatus.WebsocketUrl;
            }
        }
        finally
        {
            // Cleanup subscription
            if (_serverStateSubscriptionId != null)
            {
                await _tauriService.UnsubscribeAsync(_serverStateSubscriptionId);
                _serverStateSubscriptionId = null;
            }
        }
    }

    private Task OnServerStateChangedAsync(ServerStateEvent evt)
    {
        _logger.LogDebug("Server state changed: {State}", evt.State);

        switch (evt.State)
        {
            case ServerStateRunning:
                _serverReadyTcs?.TrySetResult(true);
                break;

            case ServerStateFailed:
                var error = new Exception(evt.Error ?? "Server failed to start");
                _serverReadyTcs?.TrySetException(error);
                break;
        }

        return Task.CompletedTask;
    }

    public async ValueTask DisposeAsync()
    {
        if (_serverStateSubscriptionId != null)
        {
            await _tauriService.UnsubscribeAsync(_serverStateSubscriptionId);
        }

        await _tauriService.DisposeAsync();
    }
}