  using Microsoft.Extensions.Logging;

  namespace ProjectManagement.Services.Desktop;

  public sealed class DesktopConfigService : IDesktopConfigService, IAsyncDisposable
  {
      private readonly TauriService _tauriService;
      private readonly ILogger<DesktopConfigService> _logger;
      private string? _serverStateSubscriptionId;
      private TaskCompletionSource<bool>? _serverReadyTcs;

      // Server state constants
      private const string ServerStateRunning = "running";
      private const string ServerStateFailed = "failed";

      public DesktopConfigService(
          TauriService tauriService,
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
              // Check current status first
              var status = await _tauriService.GetServerStatusAsync(linkedCts.Token);
              if (status.State == ServerStateRunning && status.IsHealthy)
              {
                  _logger.LogInformation("Server already running on port {Port}", status.Port);
                  return status.WebsocketUrl ?? throw new InvalidOperationException("Server running but WebSocket URL is null");
              }

              // Subscribe to state changes
              _serverStateSubscriptionId = await _tauriService.SubscribeToServerStateAsync(
                  OnServerStateChangedAsync,
                  linkedCts.Token);

              // Wait for server ready
              using (linkedCts.Token.Register(() =>
              {
                  if (timeoutCts.IsCancellationRequested)
                      _serverReadyTcs.TrySetException(new TimeoutException("Server startup timed out"));
                  else
                      _serverReadyTcs.TrySetCanceled(ct);
              }))
              {
                  await _serverReadyTcs.Task;

                  // Get the final URL after server is ready
                  var finalStatus = await _tauriService.GetServerStatusAsync(linkedCts.Token);
                  return finalStatus.WebsocketUrl ?? throw new InvalidOperationException("Server ready but WebSocket URL is null");
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