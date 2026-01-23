namespace ProjectManagement.Services.Desktop;

/// <summary>
/// Interface for Tauri IPC operations.
/// Extracted for testability - allows mocking without IJSRuntime.
/// </summary>
public interface ITauriService : IAsyncDisposable
{
    /// <summary>
    /// Checks if running in Tauri desktop environment.
    /// </summary>
    Task<bool> IsDesktopAsync();

    /// <summary>
    /// Gets current server status from Tauri backend.
    /// </summary>
    Task<ServerStatus> GetServerStatusAsync(CancellationToken ct = default);

    /// <summary>
    /// Gets WebSocket URL for connecting to server.
    /// </summary>
    Task<string> GetWebSocketUrlAsync(CancellationToken ct = default);

    /// <summary>
    /// Notifies Tauri that WASM is ready and requests current server status.
    /// Called AFTER subscribing to events to eliminate race condition.
    /// </summary>
    Task<ServerStatus> NotifyReadyAsync(CancellationToken ct = default);

    /// <summary>
    /// Subscribes to server state change events.
    /// Returns subscription ID for unsubscribing.
    /// </summary>
    Task<string> SubscribeToServerStateAsync(
        Func<ServerStateEvent, Task> callback,
        CancellationToken ct = default);

    /// <summary>
    /// Unsubscribes from server state events.
    /// </summary>
    Task UnsubscribeAsync(string subscriptionId);

    /// <summary>
    /// Requests server restart.
    /// </summary>
    Task RestartServerAsync(CancellationToken ct = default);

    /// <summary>
    /// Exports diagnostics bundle and returns file path.
    /// </summary>
    Task<string> ExportDiagnosticsAsync(CancellationToken ct = default);
}