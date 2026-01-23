using Microsoft.JSInterop;

namespace ProjectManagement.Services.Desktop;

/// <summary>
/// Manages cleanup of Tauri event subscription.
/// </summary>
internal sealed class TauriEventSubscription : IAsyncDisposable
{
    // JS function name - must match desktop-detection.js
    private const string JsUnlistenTauri = "unlistenTauri";

    private readonly string _subscriptionId;
    private readonly IJSRuntime _js;
    private readonly Action _onDispose;
    private readonly IDisposable _dotNetRef;
    private bool _disposed;

    public TauriEventSubscription(
        string subscriptionId,
        IJSRuntime js,
        Action onDispose,
        IDisposable dotNetRef)
    {
        _subscriptionId = subscriptionId;
        _js = js;
        _onDispose = onDispose;
        _dotNetRef = dotNetRef;
    }

    public async ValueTask DisposeAsync()
    {
        if (_disposed) return;
        _disposed = true;

        // Use named function instead of eval to avoid TypeLoadException
        try
        {
            await _js.InvokeAsync<bool>(JsUnlistenTauri, _subscriptionId);
        }
        catch
        {
            // Best effort cleanup - subscription may already be gone
        }

        _dotNetRef.Dispose();
        _onDispose();
    }
}