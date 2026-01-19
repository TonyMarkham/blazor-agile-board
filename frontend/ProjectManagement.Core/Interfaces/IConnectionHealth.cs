using ProjectManagement.Core.Models;

namespace ProjectManagement.Core.Interfaces;

public interface IConnectionHealth
{
  ConnectionQuality Quality { get; }
  TimeSpan? Latency { get; }
  DateTime? LastMessageReceived { get; }
  DateTime? LastMessageSent { get; }
  int PendingRequestCount { get; }
  int ReconnectAttempts { get; }
}