using Bunit;
using FluentAssertions;
using Microsoft.AspNetCore.Components;
using Microsoft.AspNetCore.Components.Web;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Logging.Abstractions;
using Moq;
using ProjectManagement.Components.Shared;
using ProjectManagement.Core.Interfaces;
using ProjectManagement.Core.Models;
using ProjectManagement.Services.State;
using Radzen;
using Radzen.Blazor;

namespace ProjectManagement.Components.Tests.Shared;

public class SharedComponentTests : BunitContext
{
    public SharedComponentTests()
    {
        // Register Radzen services
        Services.AddScoped<DialogService>();
        Services.AddScoped<NotificationService>();
        Services.AddScoped<TooltipService>();
        Services.AddScoped<ContextMenuService>();

        // Set JSInterop to loose mode (bUnit provides a mock)
        JSInterop.Mode = JSRuntimeMode.Loose;
    }

    #region OfflineBanner Tests

    private AppState CreateMockAppState(ConnectionState state)
    {
        var mockClient = new Mock<IWebSocketClient>();
        mockClient.Setup(c => c.State).Returns(state);
        mockClient.Setup(c => c.Health).Returns(Mock.Of<IConnectionHealth>());

        var mockWorkItemStore = new Mock<IWorkItemStore>();
        var mockSprintStore = new Mock<ISprintStore>();
        var mockLogger = NullLogger<AppState>.Instance;

        return new AppState(
            mockClient.Object,
            mockWorkItemStore.Object,
            mockSprintStore.Object,
            mockLogger);
    }

    [Fact]
    public void OfflineBanner_DoesNotRender_WhenConnected()
    {
        // Arrange
        var appState = CreateMockAppState(ConnectionState.Connected);
        Services.AddSingleton(appState);

        // Act
        var cut = Render<OfflineBanner>();

        // Assert
        cut.Markup.Trim().Should().BeEmpty();
    }

    [Fact]
    public void OfflineBanner_Renders_WhenDisconnected()
    {
        // Arrange
        var appState = CreateMockAppState(ConnectionState.Disconnected);
        Services.AddSingleton(appState);

        // Act
        var cut = Render<OfflineBanner>();

        // Assert
        cut.Markup.Should().Contain("offline-banner");
        cut.Markup.Should().Contain("You're offline");
        cut.Markup.Should().Contain("role=\"alert\"");
    }

    [Fact]
    public void OfflineBanner_ShowsSpinner_WhenReconnecting()
    {
        // Arrange
        var appState = CreateMockAppState(ConnectionState.Reconnecting);
        Services.AddSingleton(appState);

        // Act
        var cut = Render<OfflineBanner>();

        // Assert
        cut.Markup.Should().Contain("offline-banner");
        // Check for the progress bar component
        cut.FindComponents<RadzenProgressBarCircular>().Should().HaveCount(1);
    }

    [Fact]
    public void OfflineBanner_HasAriaLivePolite()
    {
        // Arrange
        var appState = CreateMockAppState(ConnectionState.Disconnected);
        Services.AddSingleton(appState);

        // Act
        var cut = Render<OfflineBanner>();

        // Assert
        cut.Markup.Should().Contain("aria-live=\"polite\"");
    }

    #endregion

    #region EmptyState Tests

    [Fact]
    public void EmptyState_RendersTitle()
    {
        // Act
        var cut = Render<EmptyState>(parameters => parameters
            .Add(p => p.Title, "No items found"));

        // Assert
        cut.Markup.Should().Contain("No items found");
        cut.Markup.Should().Contain("role=\"status\"");
    }

    [Fact]
    public void EmptyState_RendersDescription_WhenProvided()
    {
        // Act
        var cut = Render<EmptyState>(parameters => parameters
            .Add(p => p.Title, "No items")
            .Add(p => p.Description, "Create your first item to get started"));

        // Assert
        cut.Markup.Should().Contain("Create your first item to get started");
    }

    [Fact]
    public void EmptyState_DoesNotRenderDescription_WhenNull()
    {
        // Act
        var cut = Render<EmptyState>(parameters => parameters
            .Add(p => p.Title, "No items")
            .Add(p => p.Description, null));

        // Assert
        cut.Markup.Should().NotContain("empty-state-description");
    }

    [Fact]
    public void EmptyState_RendersActionButton_WhenShowActionTrue()
    {
        // Act
        var cut = Render<EmptyState>(parameters => parameters
            .Add(p => p.Title, "No items")
            .Add(p => p.ShowAction, true)
            .Add(p => p.ActionText, "Create Item")
            .Add(p => p.OnAction, EventCallback.Factory.Create(this, () => { })));

        // Assert
        var buttons = cut.FindComponents<RadzenButton>();
        buttons.Should().HaveCount(1);
    }

    [Fact]
    public void EmptyState_DoesNotRenderButton_WhenShowActionFalse()
    {
        // Act
        var cut = Render<EmptyState>(parameters => parameters
            .Add(p => p.Title, "No items")
            .Add(p => p.ShowAction, false));

        // Assert
        cut.FindComponents<RadzenButton>().Should().BeEmpty();
    }

    [Fact]
    public async Task EmptyState_InvokesCallback_WhenButtonClicked()
    {
        // Arrange
        var clicked = false;

        var cut = Render<EmptyState>(parameters => parameters
            .Add(p => p.Title, "No items")
            .Add(p => p.ShowAction, true)
            .Add(p => p.OnAction, EventCallback.Factory.Create(this, () => clicked = true)));

        // Act
        var button = cut.FindComponent<RadzenButton>();
        await cut.InvokeAsync(() => button.Instance.Click.InvokeAsync(null));

        // Assert
        clicked.Should().BeTrue();
    }

    [Fact]
    public void EmptyState_UsesCustomIcon()
    {
        // Act
        var cut = Render<EmptyState>(parameters => parameters
            .Add(p => p.Title, "No results")
            .Add(p => p.Icon, "search_off"));

        // Assert
        cut.Markup.Should().Contain("search_off");
    }

    #endregion

    #region LoadingButton Tests

    [Fact]
    public void LoadingButton_RendersText()
    {
        // Act
        var cut = Render<LoadingButton>(parameters => parameters
            .Add(p => p.Text, "Save"));

        // Assert
        var button = cut.FindComponent<RadzenButton>();
        button.Instance.Text.Should().Be("Save");
    }

    [Fact]
    public void LoadingButton_ShowsLoadingText_WhenBusy()
    {
        // Act
        var cut = Render<LoadingButton>(parameters => parameters
            .Add(p => p.Text, "Save")
            .Add(p => p.LoadingText, "Saving...")
            .Add(p => p.IsBusy, true));

        // Assert
        var button = cut.FindComponent<RadzenButton>();
        button.Instance.Text.Should().Be("Saving...");
        button.Instance.IsBusy.Should().BeTrue();
    }

    [Fact]
    public void LoadingButton_IsDisabled_WhenBusy()
    {
        // Act
        var cut = Render<LoadingButton>(parameters => parameters
            .Add(p => p.Text, "Save")
            .Add(p => p.IsBusy, true));

        // Assert
        var button = cut.FindComponent<RadzenButton>();
        button.Instance.Disabled.Should().BeTrue();
    }

    [Fact]
    public void LoadingButton_IsDisabled_WhenDisabledParameter()
    {
        // Act
        var cut = Render<LoadingButton>(parameters => parameters
            .Add(p => p.Text, "Save")
            .Add(p => p.Disabled, true));

        // Assert
        var button = cut.FindComponent<RadzenButton>();
        button.Instance.Disabled.Should().BeTrue();
    }

    [Fact]
    public void LoadingButton_IsDisabled_WhenDisconnected()
    {
        // Act
        var cut = Render<LoadingButton>(parameters => parameters
            .Add(p => p.Text, "Save")
            .Add(p => p.ConnectionState, ConnectionState.Disconnected));

        // Assert
        var button = cut.FindComponent<RadzenButton>();
        button.Instance.Disabled.Should().BeTrue();
    }

    [Fact]
    public void LoadingButton_IsEnabled_WhenConnected()
    {
        // Act
        var cut = Render<LoadingButton>(parameters => parameters
            .Add(p => p.Text, "Save")
            .Add(p => p.ConnectionState, ConnectionState.Connected));

        // Assert
        var button = cut.FindComponent<RadzenButton>();
        button.Instance.Disabled.Should().BeFalse();
    }

    [Fact]
    public async Task LoadingButton_InvokesCallback_WhenClicked()
    {
        // Arrange
        var clicked = false;

        var cut = Render<LoadingButton>(parameters => parameters
            .Add(p => p.Text, "Save")
            .Add(p => p.OnClick, EventCallback.Factory.Create<MouseEventArgs>(
                this, _ => clicked = true)));

        // Act
        var button = cut.FindComponent<RadzenButton>();
        await cut.InvokeAsync(() => button.Instance.Click.InvokeAsync(new MouseEventArgs()));

        // Assert
        clicked.Should().BeTrue();
    }

    [Fact]
    public void LoadingButton_ShowsOfflineTooltip_WhenDisconnected()
    {
        // Act
        var cut = Render<LoadingButton>(parameters => parameters
            .Add(p => p.Text, "Save")
            .Add(p => p.ConnectionState, ConnectionState.Disconnected));

        // Assert
        cut.Markup.Should().Contain("Offline - action unavailable");
    }

    #endregion

    #region DebouncedTextBox Tests

    [Fact]
    public void DebouncedTextBox_RendersWithValue()
    {
        // Act
        var cut = Render<DebouncedTextBox>(parameters => parameters
            .Add(p => p.Value, "test value")
            .Add(p => p.Placeholder, "Enter text..."));

        // Assert
        var textBox = cut.FindComponent<RadzenTextBox>();
        textBox.Should().NotBeNull();
    }

    [Fact]
    public void DebouncedTextBox_RendersPlaceholder()
    {
        // Act
        var cut = Render<DebouncedTextBox>(parameters => parameters
            .Add(p => p.Placeholder, "Search..."));

        // Assert
        var textBox = cut.FindComponent<RadzenTextBox>();
        textBox.Instance.Placeholder.Should().Be("Search...");
    }

    [Fact]
    public void DebouncedTextBox_IsDisabled_WhenDisabledParameter()
    {
        // Act
        var cut = Render<DebouncedTextBox>(parameters => parameters
            .Add(p => p.Disabled, true));

        // Assert
        var textBox = cut.FindComponent<RadzenTextBox>();
        textBox.Instance.Disabled.Should().BeTrue();
    }

    [Fact]
    public async Task DebouncedTextBox_FiresCallback_AfterDebounceDelay()
    {
        // Arrange
        string? receivedValue = null;

        var cut = Render<DebouncedTextBox>(parameters => parameters
            .Add(p => p.DebounceMs, 50)
            .Add(p => p.ValueChanged, EventCallback.Factory.Create<string>(this, v =>
            {
                receivedValue = v;
            })));

        var textBox = cut.FindComponent<RadzenTextBox>();

        // Act - simulate input
        await cut.InvokeAsync(() => textBox.Instance.Change.InvokeAsync("test"));

        // Wait for debounce to complete
        await Task.Delay(100);

        // Assert - should have received the final value
        receivedValue.Should().Be("test");
    }


    [Fact]
    public async Task DebouncedTextBox_ImmediateCallback_WhenDebounceIsZero()
    {
        // Arrange
        var values = new List<string>();

        var cut = Render<DebouncedTextBox>(parameters => parameters
            .Add(p => p.DebounceMs, 0)
            .Add(p => p.ValueChanged, EventCallback.Factory.Create<string>(this, v => values.Add(v))));

        var textBox = cut.FindComponent<RadzenTextBox>();

        // Act
        await cut.InvokeAsync(() => textBox.Instance.Change.InvokeAsync("test"));
        await Task.Delay(20); // Small delay for async completion

        // Assert
        values.Should().Contain("test");
    }

    #endregion

    #region ConfirmDialog Tests

    [Fact]
    public void ConfirmDialog_RendersMessage()
    {
        // Act
        var cut = Render<ConfirmDialog>(parameters => parameters
            .Add(p => p.Message, "Are you sure?"));

        // Assert
        cut.Markup.Should().Contain("Are you sure?");
    }

    [Fact]
    public void ConfirmDialog_RendersWarning_WhenProvided()
    {
        // Act
        var cut = Render<ConfirmDialog>(parameters => parameters
            .Add(p => p.Message, "Delete item?")
            .Add(p => p.WarningMessage, "This action cannot be undone"));

        // Assert
        cut.Markup.Should().Contain("This action cannot be undone");
        cut.FindComponents<RadzenAlert>().Should().HaveCount(1);
    }

    [Fact]
    public void ConfirmDialog_DoesNotRenderWarning_WhenNull()
    {
        // Act
        var cut = Render<ConfirmDialog>(parameters => parameters
            .Add(p => p.Message, "Confirm?")
            .Add(p => p.WarningMessage, null));

        // Assert
        cut.FindComponents<RadzenAlert>().Should().BeEmpty();
    }

    [Fact]
    public void ConfirmDialog_RendersCustomButtonText()
    {
        // Act
        var cut = Render<ConfirmDialog>(parameters => parameters
            .Add(p => p.Message, "Delete?")
            .Add(p => p.ConfirmText, "Delete")
            .Add(p => p.CancelText, "Keep"));

        // Assert
        var buttons = cut.FindComponents<RadzenButton>();
        buttons.Should().HaveCount(2);
        buttons[0].Instance.Text.Should().Be("Keep");
        buttons[1].Instance.Text.Should().Be("Delete");
    }

    [Fact]
    public async Task ConfirmDialog_InvokesOnConfirm()
    {
        // Arrange
        var confirmed = false;

        var cut = Render<ConfirmDialog>(parameters => parameters
            .Add(p => p.Message, "Confirm?")
            .Add(p => p.OnConfirm, EventCallback.Factory.Create(this, () => confirmed = true)));

        // Act
        var confirmButton = cut.FindComponents<RadzenButton>()
            .First(b => b.Instance.Text == "Confirm");
        await cut.InvokeAsync(() => confirmButton.Instance.Click.InvokeAsync(null));

        // Assert
        confirmed.Should().BeTrue();
    }

    [Fact]
    public async Task ConfirmDialog_InvokesOnCancel()
    {
        // Arrange
        var cancelled = false;

        var cut = Render<ConfirmDialog>(parameters => parameters
            .Add(p => p.Message, "Confirm?")
            .Add(p => p.OnCancel, EventCallback.Factory.Create(this, () => cancelled = true)));

        // Act
        var cancelButton = cut.FindComponents<RadzenButton>()
            .First(b => b.Instance.Text == "Cancel");
        await cut.InvokeAsync(() => cancelButton.Instance.Click.InvokeAsync(null));

        // Assert
        cancelled.Should().BeTrue();
    }

    [Fact]
    public void ConfirmDialog_DisablesCancelButton_WhenBusy()
    {
        // Act
        var cut = Render<ConfirmDialog>(parameters => parameters
            .Add(p => p.Message, "Confirm?")
            .Add(p => p.IsBusy, true));

        // Assert
        var cancelButton = cut.FindComponents<RadzenButton>()
            .First(b => b.Instance.Text == "Cancel");
        cancelButton.Instance.Disabled.Should().BeTrue();
    }

    [Fact]
    public void ConfirmDialog_ShowsSpinnerOnConfirmButton_WhenBusy()
    {
        // Act
        var cut = Render<ConfirmDialog>(parameters => parameters
            .Add(p => p.Message, "Confirm?")
            .Add(p => p.IsBusy, true));

        // Assert
        var confirmButton = cut.FindComponents<RadzenButton>()
            .First(b => b.Instance.Text == "Confirm");
        confirmButton.Instance.IsBusy.Should().BeTrue();
    }

    #endregion

    #region ProjectDetailSkeleton Tests

    [Fact]
    public void ProjectDetailSkeleton_RendersWithDefaultRowCount()
    {
        // Act
        var cut = Render<ProjectDetailSkeleton>();

        // Assert
        cut.Markup.Should().Contain("role=\"status\"");
        cut.Markup.Should().Contain("aria-busy=\"true\"");
        cut.FindComponents<RadzenSkeleton>().Count.Should().BeGreaterThan(5);
    }

    [Fact]
    public void ProjectDetailSkeleton_RendersCustomRowCount()
    {
        // Act
        var cut = Render<ProjectDetailSkeleton>(parameters => parameters
            .Add(p => p.RowCount, 3));

        // Assert
        cut.Markup.Should().Contain("role=\"status\"");
    }

    [Fact]
    public void ProjectDetailSkeleton_HasAccessibleLabel()
    {
        // Act
        var cut = Render<ProjectDetailSkeleton>();

        // Assert
        cut.Markup.Should().Contain("aria-label=\"Loading project details\"");
        cut.Markup.Should().Contain("Loading project details...");
    }

    [Fact]
    public void ProjectDetailSkeleton_RendersHeaderSkeleton()
    {
        // Act
        var cut = Render<ProjectDetailSkeleton>();

        // Assert - should have circle skeleton (check markup for circle shape)
        cut.Markup.Should().Contain("32px"); // Circle size
    }

    [Fact]
    public void ProjectDetailSkeleton_RendersTabsSkeleton()
    {
        // Act
        var cut = Render<ProjectDetailSkeleton>();

        // Assert - should have multiple skeleton tabs
        cut.Markup.Should().Contain("80px"); // Tab width
    }

    #endregion
}
