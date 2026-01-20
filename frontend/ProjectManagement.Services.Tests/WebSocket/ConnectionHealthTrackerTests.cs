  using FluentAssertions;
  using ProjectManagement.Core.Models;
  using ProjectManagement.Services.WebSocket;
  using Xunit;

  namespace ProjectManagement.Services.Tests.WebSocket;

  public class ConnectionHealthTrackerTests
  {
      private readonly ConnectionHealthTracker _sut = new();

      [Fact]
      public void InitialState_HasNullLatency()
      {
          _sut.Latency.Should().BeNull();
      }

      [Fact]
      public void InitialState_QualityIsUnknown()
      {
          _sut.Quality.Should().Be(ConnectionQuality.Unknown);
      }

      [Fact]
      public void RecordPong_WithCorrelatedPing_UpdatesLatency()
      {
          var messageId = "ping-1";
          _sut.RecordPingSent(messageId);

          Thread.Sleep(10);

          _sut.RecordPong(messageId, DateTimeOffset.UtcNow.ToUnixTimeSeconds());

          _sut.Latency.Should().NotBeNull();
          _sut.Latency!.Value.TotalMilliseconds.Should().BeGreaterOrEqualTo(10);
      }

      [Fact]
      public void RecordPong_WithoutCorrelatedPing_DoesNotUpdateLatency()
      {
          _sut.RecordPong("unknown-id", DateTimeOffset.UtcNow.ToUnixTimeSeconds());

          _sut.Latency.Should().BeNull();
      }

      [Fact]
      public void RecordMessageReceived_UpdatesLastMessageReceived()
      {
          var before = DateTime.UtcNow;
          _sut.RecordMessageReceived();
          var after = DateTime.UtcNow;

          _sut.LastMessageReceived.Should().NotBeNull();
          _sut.LastMessageReceived!.Value.Should().BeOnOrAfter(before);
          _sut.LastMessageReceived!.Value.Should().BeOnOrBefore(after);
      }

      [Fact]
      public void RecordConnected_ResetsReconnectAttempts()
      {
          _sut.RecordDisconnected();
          _sut.RecordDisconnected();

          _sut.ReconnectAttempts.Should().Be(2);

          _sut.RecordConnected();

          _sut.ReconnectAttempts.Should().Be(0);
      }

      [Fact]
      public void RecordDisconnected_IncrementsReconnectAttempts()
      {
          _sut.RecordDisconnected();
          _sut.RecordDisconnected();
          _sut.RecordDisconnected();

          _sut.ReconnectAttempts.Should().Be(3);
      }

      [Fact]
      public void Quality_WithExcellentLatency_ReturnsExcellent()
      {
          var messageId = "ping-1";
          _sut.RecordPingSent(messageId);
          _sut.RecordMessageReceived();
          Thread.Sleep(50);
          _sut.RecordPong(messageId, DateTimeOffset.UtcNow.ToUnixTimeSeconds());

          _sut.Quality.Should().Be(ConnectionQuality.Excellent);
      }

      [Fact]
      public void RecordRequestSent_IncrementsPendingCount()
      {
          _sut.RecordRequestSent();
          _sut.RecordRequestSent();

          _sut.PendingRequestCount.Should().Be(2);
      }

      [Fact]
      public void RecordResponseReceived_DecrementsPendingCount()
      {
          _sut.RecordRequestSent();
          _sut.RecordRequestSent();
          _sut.RecordResponseReceived();

          _sut.PendingRequestCount.Should().Be(1);
      }
  }