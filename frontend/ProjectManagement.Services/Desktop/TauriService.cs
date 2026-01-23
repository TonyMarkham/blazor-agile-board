  using Microsoft.JSInterop;
  using Microsoft.Extensions.Logging;
  using System.Collections.Concurrent;

  namespace ProjectManagement.Services.Desktop;

  /// <summary>
  /// C# wrapper for Tauri IPC commands.
  /// Replaces desktop-interop.js with type-safe C# calls.
  /// Implements proper resource management and graceful degradation.
  /// </summary>
  public sealed class TauriService : ITauriService
  {
      private readonly IJSRuntime _js;
      private readonly ILogger<TauriService> _logger;
      private readonly ConcurrentDictionary<string, IAsyncDisposable> _subscriptions = new();
      private readonly SemaphoreSlim _initLock = new(1, 1);
      private bool _disposed;
      private bool? _isDesktopCached;

      // Tauri API paths
      private const string TauriInvokePath = "__TAURI__.core.invoke";

      // Tauri command names (must match Rust command function names)
      private const string CommandGetServerStatus = "get_server_status";
      private const string CommandGetWebSocketUrl = "get_websocket_url";
      private const string CommandWasmReady = "wasm_ready";
      private const string CommandRestartServer = "restart_server";
      private const string CommandExportDiagnostics = "export_diagnostics";

      // Event names (must match Rust event names in lib.rs)
      private const string EventServerStateChanged = "server-state-changed";

      // JS interop function names (must match desktop-detection.js)
      private const string JsCheckTauriAvailable = "checkTauriAvailable";
      private const string JsSetupTauriListener = "setupTauriListener";
      private const string JsUnlistenTauri = "unlistenTauri";

      public TauriService(IJSRuntime js, ILogger<TauriService> logger)
      {
          _js = js ?? throw new ArgumentNullException(nameof(js));
          _logger = logger ?? throw new ArgumentNullException(nameof(logger));
      }

      /// <summary>
      /// Checks if running in Tauri desktop environment.
      /// Result is cached. Returns false on any error (graceful degradation).
      /// </summary>
      public async Task<bool> IsDesktopAsync()
      {
          if (_isDesktopCached.HasValue)
              return _isDesktopCached.Value;

          await _initLock.WaitAsync();
          try
          {
              if (_isDesktopCached.HasValue)
                  return _isDesktopCached.Value;

              // Use named function instead of eval to avoid TypeLoadException
              var exists = await _js.InvokeAsync<bool>(JsCheckTauriAvailable);

              _isDesktopCached = exists;
              _logger.LogInformation("Desktop mode detected: {IsDesktop}", exists);
              return exists;
          }
          catch (Exception ex)
          {
              _logger.LogDebug(ex, "Tauri detection failed, assuming web mode");
              _isDesktopCached = false;
              return false;
          }
          finally
          {
              _initLock.Release();
          }
      }

      /// <summary>
      /// Gets current server status from Tauri backend.
      /// </summary>
      public async Task<ServerStatus> GetServerStatusAsync(CancellationToken ct = default)
      {
          ThrowIfDisposed();
          await EnsureDesktopAsync();

          return await InvokeTauriAsync<ServerStatus>(CommandGetServerStatus, ct);
      }

      /// <summary>
      /// Gets WebSocket URL for connecting to server.
      /// </summary>
      public async Task<string> GetWebSocketUrlAsync(CancellationToken ct = default)
      {
          ThrowIfDisposed();
          await EnsureDesktopAsync();

          return await InvokeTauriAsync<string>(CommandGetWebSocketUrl, ct);
      }
      
      /// <summary>
      /// Notifies Tauri that WASM is ready and requests current server status.
      /// This is the second part of the handshake - called AFTER subscribing to events.
      /// </summary>
      /// <remarks>
      /// The handshake protocol:
      /// 1. WASM subscribes to server-state-changed events
      /// 2. WASM calls NotifyReadyAsync (this method)
      /// 3. Tauri responds with current ServerStatus
      /// 4. If server already running, WASM has the port
      /// 5. If server still starting, WASM waits for event
      ///
      /// This eliminates the race condition where the server-ready event
      /// fires before WASM subscribes.
      /// </remarks>
      public async Task<ServerStatus> NotifyReadyAsync(CancellationToken ct = default)
      {
          ThrowIfDisposed();
          await EnsureDesktopAsync();

          _logger.LogDebug("Sending wasm_ready notification to Tauri");
          return await InvokeTauriAsync<ServerStatus>(CommandWasmReady, ct);
      }

      /// <summary>
      /// Subscribes to server state change events.
      /// Returns subscription ID for unsubscribing.
      /// </summary>
      public async Task<string> SubscribeToServerStateAsync(
          Func<ServerStateEvent, Task> callback,
          CancellationToken ct = default)
      {
          ThrowIfDisposed();
          await EnsureDesktopAsync();

          var subscriptionId = Guid.NewGuid().ToString();
          var handler = new TauriEventHandler<ServerStateEvent>(callback, _logger);
          var dotNetRef = DotNetObjectReference.Create(handler);

          try
          {
              await _js.InvokeVoidAsync(
                  JsSetupTauriListener,
                  ct,
                  dotNetRef,
                  subscriptionId,
                  EventServerStateChanged
              );

              var subscription = new TauriEventSubscription(
                  subscriptionId,
                  _js,
                  () => _subscriptions.TryRemove(subscriptionId, out _),
                  dotNetRef
              );

              _subscriptions[subscriptionId] = subscription;

              _logger.LogDebug("Created server state subscription: {Id}", subscriptionId);
              
              return subscriptionId;
          }
          catch
          {
              dotNetRef.Dispose();
              throw;
          }
      }

      /// <summary>
      /// Unsubscribes from server state events.
      /// </summary>
      public async Task UnsubscribeAsync(string subscriptionId)
      {
          if (_subscriptions.TryRemove(subscriptionId, out var subscription))
          {
              await subscription.DisposeAsync();
              _logger.LogDebug("Removed subscription: {Id}", subscriptionId);
          }
      }

      /// <summary>
      /// Requests server restart.
      /// </summary>
      public async Task RestartServerAsync(CancellationToken ct = default)
      {
          ThrowIfDisposed();
          await EnsureDesktopAsync();

          await InvokeTauriVoidAsync(CommandRestartServer, ct);
          _logger.LogInformation("Server restart requested");
      }

      /// <summary>
      /// Exports diagnostics bundle and returns file path.
      /// </summary>
      public async Task<string> ExportDiagnosticsAsync(CancellationToken ct = default)
      {
          ThrowIfDisposed();
          await EnsureDesktopAsync();

          var path = await InvokeTauriAsync<string>(CommandExportDiagnostics, ct);
          _logger.LogInformation("Diagnostics exported to: {Path}", path);
          return path;
      }

      private async Task<T> InvokeTauriAsync<T>(string command, CancellationToken ct)
      {
          return await _js.InvokeAsync<T>(
              TauriInvokePath,
              ct,
              command
          );
      }

      private async Task InvokeTauriVoidAsync(string command, CancellationToken ct)
      {
          await _js.InvokeVoidAsync(
              TauriInvokePath,
              ct,
              command
          );
      }

      private async Task EnsureDesktopAsync()
      {
          if (!await IsDesktopAsync())
          {
              throw new InvalidOperationException(
                  "This operation requires Tauri desktop environment");
          }
      }

      private void ThrowIfDisposed()
      {
          if (_disposed)
              throw new ObjectDisposedException(nameof(TauriService));
      }

      public async ValueTask DisposeAsync()
      {
          if (_disposed) return;
          _disposed = true;

          // Dispose all subscriptions asynchronously
          foreach (var kvp in _subscriptions)
          {
              try
              {
                  await kvp.Value.DisposeAsync();
              }
              catch (Exception ex)
              {
                  _logger.LogWarning(ex, "Error disposing subscription {Id}", kvp.Key);
              }
          }

          _subscriptions.Clear();
          _initLock.Dispose();

          _logger.LogDebug("TauriService disposed");
      }
  }

  /// <summary>
  /// Handles Tauri event callbacks from JavaScript.
  /// </summary>
  internal sealed class TauriEventHandler<T>
  {
      private readonly Func<T, Task> _callback;
      private readonly ILogger _logger;

      public TauriEventHandler(Func<T, Task> callback, ILogger logger)
      {
          _callback = callback;
          _logger = logger;
      }

      [JSInvokable]
      public async Task HandleEventAsync(T payload)
      {
          try
          {
              await _callback(payload);
          }
          catch (Exception ex)
          {
              _logger.LogError(ex, "Error in Tauri event handler");
          }
      }
  }