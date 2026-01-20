  using FluentAssertions;
  using ProjectManagement.Core.Models;
  using ProjectManagement.Core.Validation;
  using Xunit;

  namespace ProjectManagement.Core.Tests.Validation;

  public class UpdateWorkItemRequestValidatorTests
  {
      private readonly UpdateWorkItemRequestValidator _sut = new();

      [Fact]
      public void Validate_ValidRequest_ReturnsSuccess()
      {
          var request = new UpdateWorkItemRequest
          {
              WorkItemId = Guid.NewGuid(),
              Title = "Updated Title"
          };

          var result = _sut.Validate(request);

          result.IsValid.Should().BeTrue();
      }

      [Fact]
      public void Validate_EmptyWorkItemId_ReturnsError()
      {
          var request = new UpdateWorkItemRequest
          {
              WorkItemId = Guid.Empty,
              Title = "Updated Title"
          };

          var result = _sut.Validate(request);

          result.IsValid.Should().BeFalse();
          result.Errors.Should().ContainSingle(e => e.Field == "workItemId");
      }

      [Fact]
      public void Validate_EmptyTitle_ReturnsError()
      {
          var request = new UpdateWorkItemRequest
          {
              WorkItemId = Guid.NewGuid(),
              Title = ""
          };

          var result = _sut.Validate(request);

          result.IsValid.Should().BeFalse();
          result.Errors.Should().ContainSingle(e => e.Field == "title");
      }

      [Fact]
      public void Validate_TitleTooLong_ReturnsError()
      {
          var request = new UpdateWorkItemRequest
          {
              WorkItemId = Guid.NewGuid(),
              Title = new string('x', 201)
          };

          var result = _sut.Validate(request);

          result.IsValid.Should().BeFalse();
          result.Errors.Should().ContainSingle(e => e.Field == "title");
      }

      [Fact]
      public void Validate_DescriptionTooLong_ReturnsError()
      {
          var request = new UpdateWorkItemRequest
          {
              WorkItemId = Guid.NewGuid(),
              Description = new string('x', 10001)
          };

          var result = _sut.Validate(request);

          result.IsValid.Should().BeFalse();
          result.Errors.Should().ContainSingle(e => e.Field == "description");
      }

      [Fact]
      public void Validate_NullTitle_DoesNotValidate()
      {
          var request = new UpdateWorkItemRequest
          {
              WorkItemId = Guid.NewGuid(),
              Title = null
          };

          var result = _sut.Validate(request);

          result.IsValid.Should().BeTrue();
      }
  }