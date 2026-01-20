  using FluentAssertions;
  using Microsoft.Extensions.Logging;
  using Microsoft.Extensions.Options;
  using Moq;
  using ProjectManagement.Core.Interfaces;
  using ProjectManagement.Services.Resilience;
  using Xunit;

  namespace ProjectManagement.Services.Tests.Resilience;

  public class ReconnectionServiceTests : IDisposable
  {
      private readonly ReconnectionService _sut;
      private readonly Mock<IWebSocketClient> _client;
      private readonly Mock<ILogger<ReconnectionService>> _logger;

      public ReconnectionServiceTests()
      {
          _client = new Mock<IWebSocketClient>();
          _logger = new Mock<ILogger<ReconnectionService>>();
          var options = Options.Create(new ReconnectionOptions
          {
              MaxAttempts = 3,
              InitialDelay = TimeSpan.FromMilliseconds(10),
              MaxDelay = TimeSpan.FromMilliseconds(100)
          });
          _sut = new ReconnectionService(_client.Object, options, _logger.Object);
      }

      [Fact]
      public void TrackSubscription_AddsToTrackedList()
      {
          var projectId = Guid.NewGuid();

          _sut.TrackSubscription(projectId);

          _sut.TrackedSubscriptions.Should().Contain(projectId);
      }

      [Fact]
      public void UntrackSubscription_RemovesFromTrackedList()
      {
          var projectId = Guid.NewGuid();
          _sut.TrackSubscription(projectId);

          _sut.UntrackSubscription(projectId);

          _sut.TrackedSubscriptions.Should().NotContain(projectId);
      }

      [Fact]
      public void TrackedSubscriptions_ReturnsSnapshot()
      {
          var projectId = Guid.NewGuid();
          _sut.TrackSubscription(projectId);

          var snapshot = _sut.TrackedSubscriptions;
          _sut.UntrackSubscription(projectId);

          snapshot.Should().Contain(projectId);
          _sut.TrackedSubscriptions.Should().NotContain(projectId);
      }

      [Fact]
      public void TrackSubscription_IsDeduplicated()
      {
          var projectId = Guid.NewGuid();

          _sut.TrackSubscription(projectId);
          _sut.TrackSubscription(projectId);

          _sut.TrackedSubscriptions.Should().HaveCount(1);
      }

      public void Dispose()
      {
          _sut.Dispose();
      }
  }