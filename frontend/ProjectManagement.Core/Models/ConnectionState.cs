namespace ProjectManagement.Core.Models;

/// <summary>
///     WebSocket connection lifecycle state.
/// </summary>
public enum ConnectionState
{
    /// <summary>Not connected to server.</summary>
    Disconnected,

    /// <summary>Attempting to establish connection.</summary>
    Connecting,

    /// <summary>Connected and ready for operations.</summary>
    Connected,

    /// <summary>Connection lost, attempting to reconnect.</summary>
    Reconnecting,

    /// <summary>Permanently closed (disposed).</summary>
    Closed
}