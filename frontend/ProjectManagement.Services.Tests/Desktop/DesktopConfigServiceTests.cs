  using FluentAssertions;
  using Microsoft.Extensions.Logging;
  using Moq;
  using ProjectManagement.Services.Desktop;

  namespace ProjectManagement.Services.Tests.Desktop;

  public class DesktopConfigServiceTests
  {
      private readonly Mock<ITauriService> _mockTauriService;
      private readonly Mock<ILogger<DesktopConfigService>> _mockLogger;
      private readonly DesktopConfigService _sut;

      public DesktopConfigServiceTests()
      {
          _mockTauriService = new Mock<ITauriService>();
          _mockLogger = new Mock<ILogger<DesktopConfigService>>();
          _sut = new DesktopConfigService(_mockTauriService.Object, _mockLogger.Object);
      }

      [Fact]
      public async Task WaitForServerAsync_WhenServerAlreadyRunning_ReturnsUrlImmediately()
      {
          // Arrange
          var expectedUrl = "ws://127.0.0.1:54321/ws";
          var runningStatus = new ServerStatus
          {
              State = "running",
              Port = 54321,
              WebsocketUrl = expectedUrl,
              IsHealthy = true,
              Pid = 12345
          };

          _mockTauriService
              .Setup(x => x.SubscribeToServerStateAsync(
                  It.IsAny<Func<ServerStateEvent, Task>>(),
                  It.IsAny<CancellationToken>()))
              .ReturnsAsync("sub-123");

          _mockTauriService
              .Setup(x => x.NotifyReadyAsync(It.IsAny<CancellationToken>()))
              .ReturnsAsync(runningStatus);

          _mockTauriService
              .Setup(x => x.UnsubscribeAsync("sub-123"))
              .Returns(Task.CompletedTask);

          // Act
          var result = await _sut.WaitForServerAsync(TimeSpan.FromSeconds(5));

          // Assert
          result.Should().Be(expectedUrl);
          _mockTauriService.Verify(
              x => x.SubscribeToServerStateAsync(
                  It.IsAny<Func<ServerStateEvent, Task>>(),
                  It.IsAny<CancellationToken>()),
              Times.Once);
          _mockTauriService.Verify(
              x => x.NotifyReadyAsync(It.IsAny<CancellationToken>()),
              Times.Once);
      }

      [Fact]
      public async Task WaitForServerAsync_WhenServerNotReady_WaitsForEvent()
      {
          // Arrange
          var expectedUrl = "ws://127.0.0.1:54321/ws";
          var startingStatus = new ServerStatus
          {
              State = "starting",
              Port = null,
              WebsocketUrl = null,
              IsHealthy = false,
              Pid = null
          };
          var runningStatus = new ServerStatus
          {
              State = "running",
              Port = 54321,
              WebsocketUrl = expectedUrl,
              IsHealthy = true,
              Pid = 12345
          };

          Func<ServerStateEvent, Task>? capturedCallback = null;

          _mockTauriService
              .Setup(x => x.SubscribeToServerStateAsync(
                  It.IsAny<Func<ServerStateEvent, Task>>(),
                  It.IsAny<CancellationToken>()))
              .Callback<Func<ServerStateEvent, Task>, CancellationToken>((cb, _) => capturedCallback = cb)
              .ReturnsAsync("sub-123");

          _mockTauriService
              .Setup(x => x.NotifyReadyAsync(It.IsAny<CancellationToken>()))
              .ReturnsAsync(startingStatus);

          _mockTauriService
              .Setup(x => x.GetServerStatusAsync(It.IsAny<CancellationToken>()))
              .ReturnsAsync(runningStatus);

          _mockTauriService
              .Setup(x => x.UnsubscribeAsync("sub-123"))
              .Returns(Task.CompletedTask);

          // Act
          var waitTask = _sut.WaitForServerAsync(TimeSpan.FromSeconds(5));

          // Simulate server becoming ready
          await Task.Delay(50);
          capturedCallback.Should().NotBeNull();
          await capturedCallback!(new ServerStateEvent { State = "running" });

          var result = await waitTask;

          // Assert
          result.Should().Be(expectedUrl);
      }

      [Fact]
      public async Task WaitForServerAsync_WhenTimeout_ThrowsTimeoutException()
      {
          // Arrange
          var startingStatus = new ServerStatus
          {
              State = "starting",
              Port = null,
              WebsocketUrl = null,
              IsHealthy = false,
              Pid = null
          };

          _mockTauriService
              .Setup(x => x.SubscribeToServerStateAsync(
                  It.IsAny<Func<ServerStateEvent, Task>>(),
                  It.IsAny<CancellationToken>()))
              .ReturnsAsync("sub-123");

          _mockTauriService
              .Setup(x => x.NotifyReadyAsync(It.IsAny<CancellationToken>()))
              .ReturnsAsync(startingStatus);

          _mockTauriService
              .Setup(x => x.UnsubscribeAsync("sub-123"))
              .Returns(Task.CompletedTask);

          // Act
          var act = () => _sut.WaitForServerAsync(TimeSpan.FromMilliseconds(100));

          // Assert
          await act.Should().ThrowAsync<TimeoutException>()
              .WithMessage("Server startup timed out");
      }

      [Fact]
      public async Task WaitForServerAsync_WhenServerFailed_ThrowsInvalidOperationException()
      {
          // Arrange
          var failedStatus = new ServerStatus
          {
              State = "failed",
              Port = null,
              WebsocketUrl = null,
              IsHealthy = false,
              Pid = null,
              Error = "Port already in use"
          };

          _mockTauriService
              .Setup(x => x.SubscribeToServerStateAsync(
                  It.IsAny<Func<ServerStateEvent, Task>>(),
                  It.IsAny<CancellationToken>()))
              .ReturnsAsync("sub-123");

          _mockTauriService
              .Setup(x => x.NotifyReadyAsync(It.IsAny<CancellationToken>()))
              .ReturnsAsync(failedStatus);

          _mockTauriService
              .Setup(x => x.UnsubscribeAsync("sub-123"))
              .Returns(Task.CompletedTask);

          // Act
          var act = () => _sut.WaitForServerAsync(TimeSpan.FromSeconds(5));

          // Assert
          await act.Should().ThrowAsync<InvalidOperationException>()
              .WithMessage("Port already in use");
      }

      [Fact]
      public async Task WaitForServerAsync_AlwaysUnsubscribes_EvenOnError()
      {
          // Arrange
          _mockTauriService
              .Setup(x => x.SubscribeToServerStateAsync(
                  It.IsAny<Func<ServerStateEvent, Task>>(),
                  It.IsAny<CancellationToken>()))
              .ReturnsAsync("sub-123");

          _mockTauriService
              .Setup(x => x.NotifyReadyAsync(It.IsAny<CancellationToken>()))
              .ThrowsAsync(new Exception("Network error"));

          _mockTauriService
              .Setup(x => x.UnsubscribeAsync("sub-123"))
              .Returns(Task.CompletedTask);

          // Act
          var act = () => _sut.WaitForServerAsync(TimeSpan.FromSeconds(5));

          // Assert
          await act.Should().ThrowAsync<Exception>();
          _mockTauriService.Verify(x => x.UnsubscribeAsync("sub-123"), Times.Once);
      }

      [Fact]
      public async Task IsDesktopModeAsync_DelegatesToTauriService()
      {
          // Arrange
          _mockTauriService
              .Setup(x => x.IsDesktopAsync())
              .ReturnsAsync(true);

          // Act
          var result = await _sut.IsDesktopModeAsync();

          // Assert
          result.Should().BeTrue();
          _mockTauriService.Verify(x => x.IsDesktopAsync(), Times.Once);
      }
  }