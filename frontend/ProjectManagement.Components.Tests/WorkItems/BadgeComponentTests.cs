using Bunit;
using FluentAssertions;
using Microsoft.Extensions.DependencyInjection;
using ProjectManagement.Components.WorkItems;
using ProjectManagement.Core.Models;
using Radzen;
using Radzen.Blazor;

namespace ProjectManagement.Components.Tests.WorkItems;

public class BadgeComponentTests : BunitContext
{
    public BadgeComponentTests()
    {
        // Register Radzen services
        Services.AddScoped<DialogService>();
        Services.AddScoped<NotificationService>();
        Services.AddScoped<TooltipService>();
        Services.AddScoped<ContextMenuService>();

        JSInterop.Mode = JSRuntimeMode.Loose;
    }

    #region WorkItemTypeIcon Tests

    [Theory]
    [InlineData(WorkItemType.Project, "folder")]
    [InlineData(WorkItemType.Epic, "rocket_launch")]
    [InlineData(WorkItemType.Story, "description")]
    [InlineData(WorkItemType.Task, "task_alt")]
    public void WorkItemTypeIcon_RendersCorrectIcon(WorkItemType type, string expectedIcon)
    {
        // Act
        var cut = Render<WorkItemTypeIcon>(parameters => parameters
            .Add(p => p.Type, type));

        // Assert
        cut.Markup.Should().Contain(expectedIcon);
    }

    [Theory]
    [InlineData(WorkItemType.Project, "Project")]
    [InlineData(WorkItemType.Epic, "Epic")]
    [InlineData(WorkItemType.Story, "Story")]
    [InlineData(WorkItemType.Task, "Task")]
    public void WorkItemTypeIcon_HasCorrectTitle(WorkItemType type, string expectedTitle)
    {
        // Act
        var cut = Render<WorkItemTypeIcon>(parameters => parameters
            .Add(p => p.Type, type));

        // Assert
        cut.Markup.Should().Contain($"title=\"{expectedTitle}\"");
    }

    [Theory]
    [InlineData(WorkItemType.Project, "Project")]
    [InlineData(WorkItemType.Epic, "Epic")]
    [InlineData(WorkItemType.Story, "Story")]
    [InlineData(WorkItemType.Task, "Task")]
    public void WorkItemTypeIcon_HasAccessibleText(WorkItemType type, string expectedText)
    {
        // Act
        var cut = Render<WorkItemTypeIcon>(parameters => parameters
            .Add(p => p.Type, type));

        // Assert
        cut.Markup.Should().Contain("visually-hidden");
        cut.Markup.Should().Contain(expectedText);
    }

    [Fact]
    public void WorkItemTypeIcon_AppliesCustomSize()
    {
        // Act
        var cut = Render<WorkItemTypeIcon>(parameters => parameters
            .Add(p => p.Type, WorkItemType.Story)
            .Add(p => p.Size, "2rem"));

        // Assert
        cut.Markup.Should().Contain("font-size: 2rem");
    }

    [Theory]
    [InlineData(WorkItemType.Project, "var(--rz-primary)")]
    [InlineData(WorkItemType.Epic, "#9c27b0")]
    [InlineData(WorkItemType.Story, "#2196f3")]
    [InlineData(WorkItemType.Task, "#4caf50")]
    public void WorkItemTypeIcon_HasCorrectColor(WorkItemType type, string expectedColor)
    {
        // Act
        var cut = Render<WorkItemTypeIcon>(parameters => parameters
            .Add(p => p.Type, type));

        // Assert
        cut.Markup.Should().Contain($"color: {expectedColor}");
    }

    [Fact]
    public void WorkItemTypeIcon_HasCssClass()
    {
        // Act
        var cut = Render<WorkItemTypeIcon>(parameters => parameters
            .Add(p => p.Type, WorkItemType.Story));

        // Assert
        cut.Markup.Should().Contain("work-item-type-icon");
    }

    #endregion

    #region WorkItemStatusBadge Tests

    [Theory]
    [InlineData("backlog", "Backlog")]
    [InlineData("todo", "To Do")]
    [InlineData("in_progress", "In Progress")]
    [InlineData("review", "Review")]
    [InlineData("done", "Done")]
    public void WorkItemStatusBadge_RendersCorrectText(string status, string expectedText)
    {
        // Act
        var cut = Render<WorkItemStatusBadge>(parameters => parameters
            .Add(p => p.Status, status));

        // Assert
        var badge = cut.FindComponent<RadzenBadge>();
        badge.Instance.Text.Should().Be(expectedText);
    }

    [Theory]
    [InlineData("backlog", BadgeStyle.Secondary)]
    [InlineData("todo", BadgeStyle.Info)]
    [InlineData("in_progress", BadgeStyle.Warning)]
    [InlineData("review", BadgeStyle.Primary)]
    [InlineData("done", BadgeStyle.Success)]
    public void WorkItemStatusBadge_HasCorrectStyle(string status, BadgeStyle expectedStyle)
    {
        // Act
        var cut = Render<WorkItemStatusBadge>(parameters => parameters
            .Add(p => p.Status, status));

        // Assert
        var badge = cut.FindComponent<RadzenBadge>();
        badge.Instance.BadgeStyle.Should().Be(expectedStyle);
    }

    [Fact]
    public void WorkItemStatusBadge_IsPillByDefault()
    {
        // Act
        var cut = Render<WorkItemStatusBadge>(parameters => parameters
            .Add(p => p.Status, "backlog"));

        // Assert
        var badge = cut.FindComponent<RadzenBadge>();
        badge.Instance.IsPill.Should().BeTrue();
    }

    [Fact]
    public void WorkItemStatusBadge_CanDisablePill()
    {
        // Act
        var cut = Render<WorkItemStatusBadge>(parameters => parameters
            .Add(p => p.Status, "backlog")
            .Add(p => p.IsPill, false));

        // Assert
        var badge = cut.FindComponent<RadzenBadge>();
        badge.Instance.IsPill.Should().BeFalse();
    }

    [Theory]
    [InlineData("backlog", "Backlog")]
    [InlineData("todo", "To Do")]
    [InlineData("in_progress", "In Progress")]
    public void WorkItemStatusBadge_HasAccessibleTitle(string status, string expectedText)
    {
        // Act
        var cut = Render<WorkItemStatusBadge>(parameters => parameters
            .Add(p => p.Status, status));

        // Assert
        cut.Markup.Should().Contain($"Status: {expectedText}");
    }

    [Fact]
    public void WorkItemStatusBadge_HandlesUnknownStatus()
    {
        // Act
        var cut = Render<WorkItemStatusBadge>(parameters => parameters
            .Add(p => p.Status, "custom_status"));

        // Assert
        var badge = cut.FindComponent<RadzenBadge>();
        badge.Instance.Text.Should().Be("custom_status");
        badge.Instance.BadgeStyle.Should().Be(BadgeStyle.Light);
    }

    #endregion

    #region PriorityBadge Tests

    [Theory]
    [InlineData("critical", "priority_high")]
    [InlineData("high", "keyboard_arrow_up")]
    [InlineData("medium", "remove")]
    [InlineData("low", "keyboard_arrow_down")]
    public void PriorityBadge_RendersCorrectIcon(string priority, string expectedIcon)
    {
        // Act
        var cut = Render<PriorityBadge>(parameters => parameters
            .Add(p => p.Priority, priority));

        // Assert
        cut.Markup.Should().Contain(expectedIcon);
    }

    [Theory]
    [InlineData("critical", "Critical")]
    [InlineData("high", "High")]
    [InlineData("medium", "Medium")]
    [InlineData("low", "Low")]
    public void PriorityBadge_RendersLabel_WhenShowLabelTrue(string priority, string expectedLabel)
    {
        // Act
        var cut = Render<PriorityBadge>(parameters => parameters
            .Add(p => p.Priority, priority)
            .Add(p => p.ShowLabel, true));

        // Assert
        cut.Markup.Should().Contain(expectedLabel);
    }

    [Fact]
    public void PriorityBadge_DoesNotRenderLabel_WhenShowLabelFalse()
    {
        // Act
        var cut = Render<PriorityBadge>(parameters => parameters
            .Add(p => p.Priority, "high")
            .Add(p => p.ShowLabel, false));

        // Assert
        // Should not have a span with the label text (just the icon)
        cut.Markup.Should().NotContain(">High<");
    }

    [Theory]
    [InlineData("critical", "#d32f2f")]
    [InlineData("high", "#f57c00")]
    [InlineData("medium", "#1976d2")]
    [InlineData("low", "#388e3c")]
    public void PriorityBadge_HasCorrectColor(string priority, string expectedColor)
    {
        // Act
        var cut = Render<PriorityBadge>(parameters => parameters
            .Add(p => p.Priority, priority));

        // Assert
        cut.Markup.Should().Contain($"color: {expectedColor}");
    }

    [Theory]
    [InlineData("critical", "Critical")]
    [InlineData("high", "High")]
    [InlineData("medium", "Medium")]
    [InlineData("low", "Low")]
    public void PriorityBadge_HasAccessibleTitle(string priority, string expectedText)
    {
        // Act
        var cut = Render<PriorityBadge>(parameters => parameters
            .Add(p => p.Priority, priority));

        // Assert
        cut.Markup.Should().Contain($"Priority: {expectedText}");
    }

    [Fact]
    public void PriorityBadge_AppliesCustomSize()
    {
        // Act
        var cut = Render<PriorityBadge>(parameters => parameters
            .Add(p => p.Priority, "high")
            .Add(p => p.Size, "1.5rem"));

        // Assert
        cut.Markup.Should().Contain("font-size: 1.5rem");
    }

    [Fact]
    public void PriorityBadge_HandlesUnknownPriority()
    {
        // Act
        var cut = Render<PriorityBadge>(parameters => parameters
            .Add(p => p.Priority, "urgent"));

        // Assert
        cut.Markup.Should().Contain("remove"); // Default icon
        cut.Markup.Should().Contain("urgent"); // Uses raw value as display text
    }

    [Fact]
    public void PriorityBadge_ShowsLabelByDefault()
    {
        // Act
        var cut = Render<PriorityBadge>(parameters => parameters
            .Add(p => p.Priority, "medium"));

        // Assert
        cut.Markup.Should().Contain("Medium");
    }

    [Fact]
    public void PriorityBadge_HasPriorityBadgeClass()
    {
        // Act
        var cut = Render<PriorityBadge>(parameters => parameters
            .Add(p => p.Priority, "high"));

        // Assert
        cut.Markup.Should().Contain("priority-badge");
    }

    #endregion
}
