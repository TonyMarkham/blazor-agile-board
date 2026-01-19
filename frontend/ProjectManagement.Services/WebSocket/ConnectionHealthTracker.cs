  using ProjectManagement.Core.Models;

  namespace ProjectManagement.Services.WebSocket;

  using System.Collections.Concurrent;
  using ProjectManagement.Core.Interfaces;

  /// <summary>
  /// Tracks connection health metrics including latency from ping/pong.
  /// Uses per-message correlation for accurate latency measurement.
  /// </summary>
  internal sealed class ConnectionHealthTracker : IConnectionHealth
  {
      // Track outstanding pings by messageId for accurate latency correlation
      private readonly ConcurrentDictionary<string, long> _pendingPings = new();

      private long _lastPongReceivedTicks;
      private long _lastMessageReceivedTicks;
      private long _lastMessageSentTicks;
      private int _pendingRequestCount;
      private int _reconnectAttempts;
      private long _latencyMs;

      public ConnectionQuality Quality
      {
          get
          {
              if (LastMessageReceived == null)
                  return ConnectionQuality.Unknown;

              var timeSinceMessage = DateTime.UtcNow - LastMessageReceived.Value;
              if (timeSinceMessage > TimeSpan.FromMinutes(2))
                  return ConnectionQuality.Disconnected;

              if (!Latency.HasValue)
                  return ConnectionQuality.Unknown;

              return Latency.Value.TotalMilliseconds switch
              {
                  < 100 => ConnectionQuality.Excellent,
                  < 300 => ConnectionQuality.Good,
                  < 1000 => ConnectionQuality.Fair,
                  _ => ConnectionQuality.Poor
              };
          }
      }

      public TimeSpan? Latency =>
          _latencyMs > 0 ? TimeSpan.FromMilliseconds(_latencyMs) : null;

      public DateTime? LastMessageReceived =>
          _lastMessageReceivedTicks > 0
              ? new DateTime(_lastMessageReceivedTicks, DateTimeKind.Utc)
              : null;

      public DateTime? LastMessageSent =>
          _lastMessageSentTicks > 0
              ? new DateTime(_lastMessageSentTicks, DateTimeKind.Utc)
              : null;

      public int PendingRequestCount => _pendingRequestCount;
      public int ReconnectAttempts => _reconnectAttempts;

      public void RecordConnected()
      {
          Interlocked.Exchange(ref _reconnectAttempts, 0);
          _pendingPings.Clear();
      }

      public void RecordDisconnected()
      {
          Interlocked.Increment(ref _reconnectAttempts);
          _pendingPings.Clear();
      }

      /// <summary>
      /// Record that a ping was sent with a specific message ID.
      /// </summary>
      public void RecordPingSent(string messageId)
      {
          _pendingPings[messageId] = DateTime.UtcNow.Ticks;

          // Clean up old pings (> 2 minutes) to prevent memory leak
          var cutoff = DateTime.UtcNow.AddMinutes(-2).Ticks;
          foreach (var kvp in _pendingPings)
          {
              if (kvp.Value < cutoff)
                  _pendingPings.TryRemove(kvp.Key, out _);
          }
      }

      /// <summary>
      /// Record pong received, correlating with the original ping by messageId.
      /// </summary>
      public void RecordPong(string messageId, long serverTimestamp)
      {
          var now = DateTime.UtcNow.Ticks;
          Interlocked.Exchange(ref _lastPongReceivedTicks, now);

          // Correlate with the specific ping that generated this pong
          if (_pendingPings.TryRemove(messageId, out var pingSentTicks))
          {
              var latency = (now - pingSentTicks) / TimeSpan.TicksPerMillisecond;
              Interlocked.Exchange(ref _latencyMs, latency);
          }
      }

      public void RecordMessageReceived()
      {
          Interlocked.Exchange(ref _lastMessageReceivedTicks, DateTime.UtcNow.Ticks);
      }

      public void RecordMessageSent()
      {
          Interlocked.Exchange(ref _lastMessageSentTicks, DateTime.UtcNow.Ticks);
      }

      public void RecordRequestSent()
      {
          Interlocked.Increment(ref _pendingRequestCount);
      }

      public void RecordResponseReceived()
      {
          Interlocked.Decrement(ref _pendingRequestCount);
      }
  }