  using FluentAssertions;
  using Microsoft.Extensions.Logging;
  using Moq;
  using ProjectManagement.Core.Interfaces;
  using ProjectManagement.Core.Models;
  using ProjectManagement.Services.State;
  using Xunit;

  namespace ProjectManagement.Services.Tests.State;

  public class SprintStoreTests
  {
      private readonly SprintStore _sut;
      private readonly Mock<IWebSocketClient> _client;
      private readonly Mock<ILogger<SprintStore>> _logger;

      public SprintStoreTests()
      {
          _client = new Mock<IWebSocketClient>();
          _logger = new Mock<ILogger<SprintStore>>();
          _sut = new SprintStore(_client.Object, _logger.Object);
          
          // Mock Sprint WebSocket operations
          _client.Setup(c => c.CreateSprintAsync(It.IsAny<CreateSprintRequest>(), It.IsAny<CancellationToken>()))
              .ReturnsAsync((CreateSprintRequest req, CancellationToken ct) => new Sprint
              {
                  Id = Guid.NewGuid(),
                  ProjectId = req.ProjectId,
                  Name = req.Name,
                  Goal = req.Goal,
                  StartDate = req.StartDate,
                  EndDate = req.EndDate,
                  Status = SprintStatus.Planned,
                  Version = 1,
                  CreatedAt = DateTime.UtcNow,
                  UpdatedAt = DateTime.UtcNow,
                  CreatedBy = Guid.Empty,
                  UpdatedBy = Guid.Empty
              });

          _client.Setup(c => c.GetSprintsAsync(It.IsAny<Guid>(), It.IsAny<CancellationToken>()))
              .ReturnsAsync(new List<Sprint>());
      }

      [Fact]
      public async Task CreateAsync_CreatesSprintLocally()
      {
          var projectId = Guid.NewGuid();
          var request = new CreateSprintRequest
          {
              ProjectId = projectId,
              Name = "Sprint 1",
              Goal = "Complete MVP",
              StartDate = DateTime.UtcNow,
              EndDate = DateTime.UtcNow.AddDays(14)
          };

          var result = await _sut.CreateAsync(request);

          result.Name.Should().Be("Sprint 1");
          result.Status.Should().Be(SprintStatus.Planned);
          _sut.GetById(result.Id).Should().NotBeNull();
      }

      [Fact]
      public async Task StartSprintAsync_TransitionsToActive()
      {
          var sprint = await CreateTestSprint();

          var started = await _sut.StartSprintAsync(sprint.Id);

          started.Status.Should().Be(SprintStatus.Active);
      }

      [Fact]
      public async Task StartSprintAsync_WhenAlreadyActive_Throws()
      {
          var sprint = await CreateTestSprint();
          await _sut.StartSprintAsync(sprint.Id);

          var act = () => _sut.StartSprintAsync(sprint.Id);

          await act.Should().ThrowAsync<InvalidOperationException>();
      }

      [Fact]
      public async Task StartSprintAsync_WhenAnotherActive_Throws()
      {
          var projectId = Guid.NewGuid();
          var sprint1 = await CreateTestSprint(projectId);
          var sprint2 = await CreateTestSprint(projectId);
          await _sut.StartSprintAsync(sprint1.Id);

          var act = () => _sut.StartSprintAsync(sprint2.Id);

          await act.Should().ThrowAsync<InvalidOperationException>()
              .WithMessage("*already has an active sprint*");
      }

      [Fact]
      public async Task CompleteSprintAsync_TransitionsToCompleted()
      {
          var sprint = await CreateTestSprint();
          await _sut.StartSprintAsync(sprint.Id);

          var completed = await _sut.CompleteSprintAsync(sprint.Id);

          completed.Status.Should().Be(SprintStatus.Completed);
      }

      [Fact]
      public async Task CompleteSprintAsync_WhenNotActive_Throws()
      {
          var sprint = await CreateTestSprint();

          var act = () => _sut.CompleteSprintAsync(sprint.Id);

          await act.Should().ThrowAsync<InvalidOperationException>();
      }

      [Fact]
      public async Task GetActiveSprint_ReturnsActiveSprint()
      {
          var projectId = Guid.NewGuid();
          var sprint = await CreateTestSprint(projectId);
          await _sut.StartSprintAsync(sprint.Id);

          var active = _sut.GetActiveSprint(projectId);

          active.Should().NotBeNull();
          active!.Id.Should().Be(sprint.Id);
      }

      [Fact]
      public void GetActiveSprint_WhenNone_ReturnsNull()
      {
          var result = _sut.GetActiveSprint(Guid.NewGuid());

          result.Should().BeNull();
      }

      [Fact]
      public async Task DeleteAsync_SoftDeletesSprint()
      {
          var sprint = await CreateTestSprint();

          await _sut.DeleteAsync(sprint.Id);

          _sut.GetById(sprint.Id).Should().BeNull();
      }

      [Fact]
      public async Task GetByProject_FiltersCorrectly()
      {
          var projectId = Guid.NewGuid();
          var otherProjectId = Guid.NewGuid();

          var sprint1 = await CreateTestSprint(projectId);
          var sprint2 = await CreateTestSprint(projectId);
          var sprint3 = await CreateTestSprint(otherProjectId);

          var result = _sut.GetByProject(projectId);

          result.Should().HaveCount(2);
          result.Should().Contain(s => s.Id == sprint1.Id);
          result.Should().Contain(s => s.Id == sprint2.Id);
          result.Should().NotContain(s => s.Id == sprint3.Id);
      }

      private async Task<Sprint> CreateTestSprint(Guid? projectId = null)
      {
          return await _sut.CreateAsync(new CreateSprintRequest
          {
              ProjectId = projectId ?? Guid.NewGuid(),
              Name = $"Sprint {Guid.NewGuid().ToString()[..8]}",
              StartDate = DateTime.UtcNow,
              EndDate = DateTime.UtcNow.AddDays(14)
          });
      }
  }