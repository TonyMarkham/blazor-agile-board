using Bunit;
using FluentAssertions;
using Microsoft.AspNetCore.Components;
using Microsoft.Extensions.DependencyInjection;
using Moq;
using ProjectManagement.Components.Dependencies;
using ProjectManagement.Core.Interfaces;
using ProjectManagement.Core.Models;
using ProjectManagement.Core.ViewModels;
using Radzen;

namespace ProjectManagement.Components.Tests.Dependencies;

public class DependencyRowTests : BunitContext
{
    public DependencyRowTests()
    {
        // Register Radzen services
        Services.AddScoped<DialogService>();
        Services.AddScoped<NotificationService>();
        Services.AddScoped<TooltipService>();
        Services.AddScoped<ContextMenuService>();

        // Set JSInterop to loose mode
        JSInterop.Mode = JSRuntimeMode.Loose;
    }

    [Fact]
    public void DependencyRow_DisablesDeleteButton_WhenPending()
    {
        // Arrange
        var dependency = CreateDependency(DependencyType.Blocks);
        var store = new Mock<IDependencyStore>();
        store.Setup(s => s.IsPending(dependency.Id)).Returns(true);

        Services.AddScoped(_ => store.Object);

        // Act
        var cut = Render<DependencyRow>(parameters => parameters
            .Add(p => p.Dependency, dependency)
            .Add(p => p.CurrentWorkItemId, dependency.BlockingItemId)
            .Add(p => p.WorkItemLookup, _ => null)
            .Add(p => p.OnRemove, EventCallback.Factory.Create<Guid>(this, _ => { })));

        // Assert
        var deleteButton = cut.Find("button.delete-btn");
        deleteButton.HasAttribute("disabled").Should().BeTrue();
    }

    [Fact]
    public void DependencyRow_UsesFallbackTitle_WhenLookupMissing()
    {
        // Arrange
        var dependency = CreateDependency(DependencyType.Blocks);
        var store = new Mock<IDependencyStore>();
        store.Setup(s => s.IsPending(dependency.Id)).Returns(false);

        Services.AddScoped(_ => store.Object);

        var expected = dependency.BlockedItemId.ToString()[..8];

        // Act
        var cut = Render<DependencyRow>(parameters => parameters
            .Add(p => p.Dependency, dependency)
            .Add(p => p.CurrentWorkItemId, dependency.BlockingItemId)
            .Add(p => p.WorkItemLookup, _ => null)
            .Add(p => p.OnRemove, EventCallback.Factory.Create<Guid>(this, _ => { })));

        // Assert
        cut.Markup.Should().Contain(expected);
    }

    [Fact]
    public void DependencyRow_ShowsBlocksLabelAndClass_ForBlocksType()
    {
        // Arrange
        var dependency = CreateDependency(DependencyType.Blocks);
        var store = new Mock<IDependencyStore>();
        store.Setup(s => s.IsPending(dependency.Id)).Returns(false);

        Services.AddScoped(_ => store.Object);

        // Act
        var cut = Render<DependencyRow>(parameters => parameters
            .Add(p => p.Dependency, dependency)
            .Add(p => p.CurrentWorkItemId, dependency.BlockingItemId)
            .Add(p => p.WorkItemLookup, _ => null)
            .Add(p => p.OnRemove, EventCallback.Factory.Create<Guid>(this, _ => { })));

        // Assert
        var typeBadge = cut.Find(".dep-type");
        typeBadge.TextContent.Should().Be("Blocks");
        typeBadge.ClassList.Should().Contain("blocks");
    }

    [Fact]
    public void DependencyRow_ShowsRelatesLabelAndClass_ForRelatesType()
    {
        // Arrange
        var dependency = CreateDependency(DependencyType.RelatesTo);
        var store = new Mock<IDependencyStore>();
        store.Setup(s => s.IsPending(dependency.Id)).Returns(false);

        Services.AddScoped(_ => store.Object);

        // Act
        var cut = Render<DependencyRow>(parameters => parameters
            .Add(p => p.Dependency, dependency)
            .Add(p => p.CurrentWorkItemId, dependency.BlockingItemId)
            .Add(p => p.WorkItemLookup, _ => null)
            .Add(p => p.OnRemove, EventCallback.Factory.Create<Guid>(this, _ => { })));

        // Assert
        var typeBadge = cut.Find(".dep-type");
        typeBadge.TextContent.Should().Be("Relates");
        typeBadge.ClassList.Should().Contain("relates");
    }

    private static Dependency CreateDependency(DependencyType type) => new()
    {
        Id = Guid.NewGuid(),
        BlockingItemId = Guid.NewGuid(),
        BlockedItemId = Guid.NewGuid(),
        Type = type,
        CreatedAt = DateTime.UtcNow,
        CreatedBy = Guid.NewGuid()
    };
}
