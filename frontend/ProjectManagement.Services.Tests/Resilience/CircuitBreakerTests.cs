  using System.IO;
  using FluentAssertions;
  using Microsoft.Extensions.Logging;
  using Microsoft.Extensions.Options;
  using Moq;
  using ProjectManagement.Core.Exceptions;
  using ProjectManagement.Services.Resilience;
  using Xunit;

  namespace ProjectManagement.Services.Tests.Resilience;

  public class CircuitBreakerTests
  {
      private readonly CircuitBreaker _sut;
      private readonly Mock<ILogger<CircuitBreaker>> _logger;

      public CircuitBreakerTests()
      {
          _logger = new Mock<ILogger<CircuitBreaker>>();
          var options = Options.Create(new CircuitBreakerOptions
          {
              FailureThreshold = 3,
              OpenDuration = TimeSpan.FromMilliseconds(100),
              HalfOpenSuccessThreshold = 2,
              FailureWindow = TimeSpan.FromSeconds(60)
          });
          _sut = new CircuitBreaker(options, _logger.Object);
      }

      [Fact]
      public void InitialState_IsClosed()
      {
          _sut.State.Should().Be(CircuitState.Closed);
      }

      [Fact]
      public void AllowRequest_WhenClosed_ReturnsTrue()
      {
          _sut.AllowRequest().Should().BeTrue();
      }

      [Fact]
      public void RecordFailure_BelowThreshold_StaysClosed()
      {
          _sut.RecordFailure();
          _sut.RecordFailure();

          _sut.State.Should().Be(CircuitState.Closed);
      }

      [Fact]
      public void RecordFailure_AtThreshold_OpensCircuit()
      {
          _sut.RecordFailure();
          _sut.RecordFailure();
          _sut.RecordFailure();

          _sut.State.Should().Be(CircuitState.Open);
      }

      [Fact]
      public void AllowRequest_WhenOpen_ReturnsFalse()
      {
          for (int i = 0; i < 3; i++)
              _sut.RecordFailure();

          _sut.AllowRequest().Should().BeFalse();
      }

      [Fact]
      public async Task AllowRequest_AfterOpenDuration_TransitionsToHalfOpen()
      {
          for (int i = 0; i < 3; i++)
              _sut.RecordFailure();

          await Task.Delay(150);

          _sut.AllowRequest().Should().BeTrue();
          _sut.State.Should().Be(CircuitState.HalfOpen);
      }

      [Fact]
      public void RecordSuccess_InClosed_ResetsFailureCount()
      {
          _sut.RecordFailure();
          _sut.RecordFailure();
          _sut.RecordSuccess();

          _sut.State.Should().Be(CircuitState.Closed);

          _sut.RecordFailure();
          _sut.RecordFailure();
          _sut.State.Should().Be(CircuitState.Closed);
      }

      [Fact]
      public async Task RecordSuccess_InHalfOpen_ClosesAfterThreshold()
      {
          for (int i = 0; i < 3; i++)
              _sut.RecordFailure();

          await Task.Delay(150);
          _sut.AllowRequest();

          _sut.RecordSuccess();
          _sut.State.Should().Be(CircuitState.HalfOpen);

          _sut.RecordSuccess();
          _sut.State.Should().Be(CircuitState.Closed);
      }

      [Fact]
      public async Task RecordFailure_InHalfOpen_ReopensCircuit()
      {
          for (int i = 0; i < 3; i++)
              _sut.RecordFailure();

          await Task.Delay(150);
          _sut.AllowRequest();

          _sut.RecordFailure();

          _sut.State.Should().Be(CircuitState.Open);
      }

      [Fact]
      public async Task ExecuteAsync_WhenCircuitOpen_ThrowsCircuitOpenException()
      {
          for (int i = 0; i < 3; i++)
              _sut.RecordFailure();

          var act = () => _sut.ExecuteAsync(ct => Task.FromResult(42));

          await act.Should().ThrowAsync<CircuitOpenException>();
      }

      [Fact]
      public async Task ExecuteAsync_RecordsSuccessOnCompletion()
      {
          var result = await _sut.ExecuteAsync(ct => Task.FromResult(42));

          result.Should().Be(42);
          _sut.State.Should().Be(CircuitState.Closed);
      }

      [Fact]
      public async Task ExecuteAsync_RecordsFailureOnException()
      {
          var act = () => _sut.ExecuteAsync<int>(
              ct => throw new IOException("Network error"));

          await act.Should().ThrowAsync<IOException>();

          _sut.RecordFailure();
          _sut.RecordFailure();
          _sut.State.Should().Be(CircuitState.Open);
      }

      [Fact]
      public async Task ExecuteAsync_DoesNotRecordValidationExceptionAsFailure()
      {
          var act = () => _sut.ExecuteAsync<int>(
              ct => throw new ValidationException("title", "Required"));

          await act.Should().ThrowAsync<ValidationException>();

          _sut.RecordFailure();
          _sut.RecordFailure();
          _sut.State.Should().Be(CircuitState.Closed);
      }

      [Fact]
      public async Task ExecuteAsync_DoesNotRecordVersionConflictAsFailure()
      {
          var act = () => _sut.ExecuteAsync<int>(
              ct => throw new VersionConflictException(Guid.NewGuid(), 1, 2));

          await act.Should().ThrowAsync<VersionConflictException>();

          _sut.RecordFailure();
          _sut.RecordFailure();
          _sut.State.Should().Be(CircuitState.Closed);
      }

      [Fact]
      public void RetryAfter_WhenClosed_ReturnsNull()
      {
          _sut.RetryAfter.Should().BeNull();
      }

      [Fact]
      public void RetryAfter_WhenOpen_ReturnsRemainingTime()
      {
          for (int i = 0; i < 3; i++)
              _sut.RecordFailure();

          var retryAfter = _sut.RetryAfter;

          retryAfter.Should().NotBeNull();
          retryAfter!.Value.Should().BePositive();
          retryAfter!.Value.Should().BeLessOrEqualTo(TimeSpan.FromMilliseconds(100));
      }
  }