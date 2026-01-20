  using FluentAssertions;
  using ProjectManagement.Core.Models;
  using ProjectManagement.Core.Validation;
  using Xunit;

  namespace ProjectManagement.Core.Tests.Validation;

  public class CreateWorkItemRequestValidatorTests
  {
      private readonly CreateWorkItemRequestValidator _sut = new();

      [Fact]
      public void Validate_ValidRequest_ReturnsSuccess()
      {
          var request = new CreateWorkItemRequest
          {
              ItemType = WorkItemType.Task,
              Title = "Valid Task",
              ProjectId = Guid.NewGuid()
          };

          var result = _sut.Validate(request);

          result.IsValid.Should().BeTrue();
      }

      [Fact]
      public void Validate_MissingTitle_ReturnsError()
      {
          var request = new CreateWorkItemRequest
          {
              ItemType = WorkItemType.Task,
              Title = "",
              ProjectId = Guid.NewGuid()
          };

          var result = _sut.Validate(request);

          result.IsValid.Should().BeFalse();
          result.Errors.Should().ContainSingle(e => e.Field == "title");
      }

      [Fact]
      public void Validate_TitleTooLong_ReturnsError()
      {
          var request = new CreateWorkItemRequest
          {
              ItemType = WorkItemType.Task,
              Title = new string('x', 201),
              ProjectId = Guid.NewGuid()
          };

          var result = _sut.Validate(request);

          result.IsValid.Should().BeFalse();
          result.Errors.Should().ContainSingle(e => e.Field == "title");
      }

      [Fact]
      public void Validate_EmptyProjectId_ReturnsError()
      {
          var request = new CreateWorkItemRequest
          {
              ItemType = WorkItemType.Task,
              Title = "Valid Task",
              ProjectId = Guid.Empty
          };

          var result = _sut.Validate(request);

          result.IsValid.Should().BeFalse();
          result.Errors.Should().ContainSingle(e => e.Field == "projectId");
      }

      [Fact]
      public void Validate_DescriptionTooLong_ReturnsError()
      {
          var request = new CreateWorkItemRequest
          {
              ItemType = WorkItemType.Task,
              Title = "Valid Task",
              ProjectId = Guid.NewGuid(),
              Description = new string('x', 10001)
          };

          var result = _sut.Validate(request);

          result.IsValid.Should().BeFalse();
          result.Errors.Should().ContainSingle(e => e.Field == "description");
      }

      [Fact]
      public void Validate_ProjectWithParent_ReturnsError()
      {
          var request = new CreateWorkItemRequest
          {
              ItemType = WorkItemType.Project,
              Title = "Valid Project",
              ProjectId = Guid.NewGuid(),
              ParentId = Guid.NewGuid()
          };

          var result = _sut.Validate(request);

          result.IsValid.Should().BeFalse();
          result.Errors.Should().ContainSingle(e => e.Field == "parentId");
      }
  }