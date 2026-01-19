  namespace ProjectManagement.Services.Resilience;

  /// <summary>
  /// Configuration for circuit breaker behavior.
  /// </summary>
  public sealed class CircuitBreakerOptions
  {
      public const int DefaultFailureThreshold = 5;
      public const int DefaultOpenDurationSeconds = 30;
      public const int DefaultHalfOpenSuccessThreshold = 3;
      public const int DefaultFailureWindowSeconds = 60;

      /// <summary>Number of failures before opening circuit.</summary>
      public int FailureThreshold { get; set; } = DefaultFailureThreshold;

      /// <summary>Duration to keep circuit open before testing.</summary>
      public TimeSpan OpenDuration { get; set; } = TimeSpan.FromSeconds(DefaultOpenDurationSeconds);

      /// <summary>Successes needed in half-open to close circuit.</summary>
      public int HalfOpenSuccessThreshold { get; set; } = DefaultHalfOpenSuccessThreshold;

      /// <summary>Window for counting failures.</summary>
      public TimeSpan FailureWindow { get; set; } = TimeSpan.FromSeconds(DefaultFailureWindowSeconds);
  }