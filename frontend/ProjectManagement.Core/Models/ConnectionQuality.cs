namespace ProjectManagement.Core.Models;

public enum ConnectionQuality
{
    Unknown,
    Excellent, // <100ms latency
    Good, // 100-300ms
    Fair, // 300-1000ms
    Poor, // >1000ms or packet loss
    Disconnected
}