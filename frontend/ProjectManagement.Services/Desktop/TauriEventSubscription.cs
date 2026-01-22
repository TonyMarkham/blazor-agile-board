using Microsoft.JSInterop;

namespace ProjectManagement.Services.Desktop;

/// <summary>
/// Manages cleanup of Tauri event subscription.
/// </summary>
internal sealed class TauriEventSubscription : IAsyncDisposable
{
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

        // Properly await JS cleanup
        try
        {
            await _js.InvokeVoidAsync(
                "eval",
                $"window.__PM_UNLISTENERS__?.['{_subscriptionId}']?.()"
            );
        }
        catch { /* Best effort cleanup */ }

        _dotNetRef.Dispose();
        _onDispose();
    }
}