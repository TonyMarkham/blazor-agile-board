  using System.IO;
  using FluentAssertions;
  using Microsoft.Extensions.Logging;
  using Microsoft.Extensions.Options;
  using Moq;
  using ProjectManagement.Core.Exceptions;
  using ProjectManagement.Services.Resilience;
  using Xunit;

  namespace ProjectManagement.Services.Tests.Resilience;

  public class RetryPolicyTests
  {
      private readonly RetryPolicy _sut;
      private readonly Mock<ILogger<RetryPolicy>> _logger;

      public RetryPolicyTests()
      {
          _logger = new Mock<ILogger<RetryPolicy>>();
          var options = Options.Create(new RetryPolicyOptions
          {
              MaxAttempts = 3,
              InitialDelay = TimeSpan.FromMilliseconds(10),
              MaxDelay = TimeSpan.FromMilliseconds(100),
              BackoffMultiplier = 2.0
          });
          _sut = new RetryPolicy(options, _logger.Object);
      }

      [Fact]
      public async Task ExecuteAsync_SuccessOnFirstAttempt_ReturnsResult()
      {
          var result = await _sut.ExecuteAsync(ct => Task.FromResult(42));

          result.Should().Be(42);
      }

      [Fact]
      public async Task ExecuteAsync_SuccessAfterRetry_ReturnsResult()
      {
          var attempts = 0;

          var result = await _sut.ExecuteAsync(ct =>
          {
              attempts++;
              if (attempts < 2)
                  throw new ConnectionException("Connection failed");
              return Task.FromResult(42);
          });

          result.Should().Be(42);
          attempts.Should().Be(2);
      }

      [Fact]
      public async Task ExecuteAsync_ExhaustsRetries_ThrowsLastException()
      {
          var attempts = 0;

          var act = () => _sut.ExecuteAsync<int>(ct =>
          {
              attempts++;
              throw new ConnectionException($"Attempt {attempts}");
          });

          await act.Should().ThrowAsync<ConnectionException>()
              .WithMessage("Attempt 3");
          attempts.Should().Be(3);
      }

      [Fact]
      public async Task ExecuteAsync_NonRetryableException_ThrowsImmediately()
      {
          var attempts = 0;

          var act = () => _sut.ExecuteAsync<int>(ct =>
          {
              attempts++;
              throw new InvalidOperationException("Not retryable");
          });

          await act.Should().ThrowAsync<InvalidOperationException>();
          attempts.Should().Be(1);
      }

      [Fact]
      public async Task ExecuteAsync_RetriesIOException()
      {
          var attempts = 0;

          await _sut.ExecuteAsync(ct =>
          {
              attempts++;
              if (attempts < 2)
                  throw new IOException("IO failed");
              return Task.FromResult(true);
          });

          attempts.Should().Be(2);
      }

      [Fact]
      public async Task ExecuteAsync_RetriesRequestTimeoutException()
      {
          var attempts = 0;

          await _sut.ExecuteAsync(ct =>
          {
              attempts++;
              if (attempts < 2)
                  throw new RequestTimeoutException("req-1", TimeSpan.FromSeconds(30));
              return Task.FromResult(true);
          });

          attempts.Should().Be(2);
      }

      [Fact]
      public async Task ExecuteAsync_CancellationRespected()
      {
          using var cts = new CancellationTokenSource();
          cts.Cancel();

          var act = () => _sut.ExecuteAsync(
              ct =>
              {
                  ct.ThrowIfCancellationRequested();
                  return Task.FromResult(42);
              },
              cts.Token);

          await act.Should().ThrowAsync<OperationCanceledException>();
      }

      [Fact]
      public async Task ExecuteAsync_DelayIncreases()
      {
          var attempts = new List<DateTime>();

          var act = () => _sut.ExecuteAsync<int>(ct =>
          {
              attempts.Add(DateTime.UtcNow);
              throw new ConnectionException("Failed");
          });

          await act.Should().ThrowAsync<ConnectionException>();

          var delay1 = (attempts[1] - attempts[0]).TotalMilliseconds;
          var delay2 = (attempts[2] - attempts[1]).TotalMilliseconds;

          delay2.Should().BeGreaterThan(delay1 * 1.5);
      }

      [Fact]
      public async Task ExecuteAsync_RespectsMaxDelay()
      {
          var options = Options.Create(new RetryPolicyOptions
          {
              MaxAttempts = 5,
              InitialDelay = TimeSpan.FromMilliseconds(10),
              MaxDelay = TimeSpan.FromMilliseconds(50),
              BackoffMultiplier = 2.0
          });
          var policy = new RetryPolicy(options, _logger.Object);

          var attempts = new List<DateTime>();

          var act = () => policy.ExecuteAsync<int>(ct =>
          {
              attempts.Add(DateTime.UtcNow);
              throw new ConnectionException("Failed");
          });

          await act.Should().ThrowAsync<ConnectionException>();

          var delay4 = (attempts[4] - attempts[3]).TotalMilliseconds;
          delay4.Should().BeLessThan(80);
      }

      [Fact]
      public async Task ExecuteAsync_DoesNotRetryValidationException()
      {
          var attempts = 0;

          var act = () => _sut.ExecuteAsync<int>(ct =>
          {
              attempts++;
              throw new ValidationException("field", "error");
          });

          await act.Should().ThrowAsync<ValidationException>();
          attempts.Should().Be(1);
      }
  }