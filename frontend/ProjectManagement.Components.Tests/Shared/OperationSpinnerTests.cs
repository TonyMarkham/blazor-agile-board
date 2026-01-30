using Bunit;
using FluentAssertions;
using Microsoft.AspNetCore.Components;
using Microsoft.Extensions.DependencyInjection;
using ProjectManagement.Components.Shared;
using Radzen;
using Radzen.Blazor;
using Xunit;

namespace ProjectManagement.Components.Tests.Shared;

public class OperationSpinnerTests : BunitContext
{
    public OperationSpinnerTests()
    {
        Services.AddScoped<DialogService>();
        Services.AddScoped<NotificationService>();
        Services.AddScoped<TooltipService>();
        Services.AddScoped<ContextMenuService>();

        JSInterop.Mode = JSRuntimeMode.Loose;
    }

    [Fact]
    public void OperationSpinner_Renders_WhenVisible()
    {
        var cut = Render<OperationSpinner>(parameters => parameters
            .Add(p => p.IsVisible, true));

        cut.Markup.Should().Contain("operation-spinner");
        cut.Markup.Should().Contain("visible");

        var root = cut.Find("div.operation-spinner");
        root.HasAttribute("aria-busy").Should().BeTrue();
        (root.GetAttribute("aria-busy") ?? "true").Should().Be("true");

        cut.FindComponents<RadzenProgressBarCircular>().Should().HaveCount(1);
    }

    [Fact]
    public void OperationSpinner_DoesNotRenderMessage_WhenNull()
    {
        var cut = Render<OperationSpinner>(parameters => parameters
            .Add(p => p.IsVisible, true)
            .Add(p => p.Message, null));

        cut.Markup.Should().NotContain("spinner-message");
    }

    [Fact]
    public void OperationSpinner_ShowsCancelButton_WhenCallbackProvided()
    {
        var cut = Render<OperationSpinner>(parameters => parameters
            .Add(p => p.IsVisible, true)
            .Add(p => p.OnCancel, EventCallback.Factory.Create(this, () => { })));

        cut.FindComponents<RadzenButton>().Should().HaveCount(1);
    }
}