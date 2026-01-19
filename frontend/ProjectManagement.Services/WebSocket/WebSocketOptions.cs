namespace ProjectManagement.Services.WebSocket;

public sealed class WebSocketOptions
{
    // Aligned with backend pm-config defaults
    private const string DEFAULT_SERVER_URL = "ws://localhost:8080/ws";
    private const int DEFAULT_HEARTBEAT_INTERVAL_SECS = 30;
    private const int DEFAULT_HEARTBEAT_TIMEOUT_SECS = 60;
    private const int DEFAULT_REQUEST_TIMEOUT_SECS = 30;
    private const int DEFAULT_SEND_BUFFER_SIZE = 100;
    private const int DEFAULT_RECEIVE_BUFFER_SIZE = 64 * 1024;

    /// <summary>WebSocket server URL (ws:// or wss://).</summary>
    public string ServerUrl { get; set; } = DEFAULT_SERVER_URL;

    /// <summary>JWT token for authentication (null for desktop mode).</summary>
    public string? JwtToken { get; set; }

    /// <summary>Interval between ping messages.</summary>
    public TimeSpan HeartbeatInterval { get; set; } = TimeSpan.FromSeconds(DEFAULT_HEARTBEAT_INTERVAL_SECS);

    /// <summary>Timeout waiting for pong response.</summary>
    public TimeSpan HeartbeatTimeout { get; set; } = TimeSpan.FromSeconds(DEFAULT_HEARTBEAT_TIMEOUT_SECS);

    /// <summary>Timeout for request/response operations.</summary>
    public TimeSpan RequestTimeout { get; set; } = TimeSpan.FromSeconds(DEFAULT_REQUEST_TIMEOUT_SECS);

    /// <summary>Size of send buffer (messages).</summary>
    public int SendBufferSize { get; set; } = DEFAULT_SEND_BUFFER_SIZE;

    /// <summary>Size of receive buffer (bytes).</summary>
    public int ReceiveBufferSize { get; set; } = DEFAULT_RECEIVE_BUFFER_SIZE;
}