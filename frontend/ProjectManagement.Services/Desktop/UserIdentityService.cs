  using System.Text.Json;
  using Microsoft.Extensions.Logging;
  using Microsoft.JSInterop;
  using ProjectManagement.Core.Models;

  namespace ProjectManagement.Services.Desktop;

  /// <summary>
  /// Manages persistent user identity for desktop mode.
  /// Uses Tauri's file system API for cross-platform storage.
  /// </summary>
  public sealed class UserIdentityService : IAsyncDisposable
  {
      private readonly IJSRuntime _js;
      private readonly ILogger<UserIdentityService> _logger;
      private UserIdentity? _cachedIdentity;
      private bool _disposed;
      private readonly SemaphoreSlim _lock = new(1, 1);

      // Retry configuration
      private const int MaxRetries = 3;
      private const int RetryDelayInitialMs = 100;
      private const int RetryDelayMediumMs = 500;
      private const int RetryDelayMaxMs = 1000;
      private static readonly TimeSpan[] RetryDelays = {
          TimeSpan.FromMilliseconds(RetryDelayInitialMs),
          TimeSpan.FromMilliseconds(RetryDelayMediumMs),
          TimeSpan.FromMilliseconds(RetryDelayMaxMs)
      };

      // Tauri command names
      private const string TauriCommandLoadIdentity = "load_user_identity";
      private const string TauriCommandSaveIdentity = "save_user_identity";
      private const string TauriCommandBackupCorrupted = "backup_corrupted_user_identity";
      private const string TauriInvokePath = "__TAURI__.core.invoke";

      public UserIdentityService(IJSRuntime js, ILogger<UserIdentityService> logger)
      {
          _js = js ?? throw new ArgumentNullException(nameof(js));
          _logger = logger ?? throw new ArgumentNullException(nameof(logger));
      }

      /// <summary>
      /// Gets the current user identity, loading from disk if not cached.
      /// Thread-safe with lock to prevent concurrent loads.
      /// </summary>
      public async Task<UserIdentity?> GetCurrentUserAsync(CancellationToken ct = default)
      {
          ThrowIfDisposed();

          if (_cachedIdentity != null)
              return _cachedIdentity;

          await _lock.WaitAsync(ct);
          try
          {
              // Double-check after acquiring lock
              if (_cachedIdentity != null)
                  return _cachedIdentity;

              return await LoadExistingUserAsync(ct);
          }
          finally
          {
              _lock.Release();
          }
      }

      /// <summary>
      /// Loads existing user identity from persistent storage with retry logic.
      /// Returns null if file doesn't exist.
      /// Attempts recovery if file is corrupted.
      /// </summary>
      public async Task<UserIdentity?> LoadExistingUserAsync(CancellationToken ct = default)
      {
          ThrowIfDisposed();

          Exception? lastException = null;

          for (int attempt = 0; attempt <= MaxRetries; attempt++)
          {
              try
              {
                  if (attempt > 0)
                  {
                      var delay = RetryDelays[Math.Min(attempt - 1, RetryDelays.Length - 1)];
                      _logger.LogDebug("Retry attempt {Attempt} after {Delay}ms", attempt, delay.TotalMilliseconds);
                      await Task.Delay(delay, ct);
                  }

                  var result = await _js.InvokeAsync<UserIdentityResult>(
                      TauriInvokePath,
                      ct,
                      TauriCommandLoadIdentity
                  );

                  if (result == null)
                  {
                      _logger.LogWarning("Null result from load_user_identity");
                      continue;
                  }

                  if (!string.IsNullOrEmpty(result.Error))
                  {
                      _logger.LogWarning("Error from Tauri: {Error}", result.Error);
                      return await HandleCorruptedFileAsync(ct);
                  }

                  if (result.User == null)
                  {
                      _logger.LogInformation("No existing user identity found (first launch)");
                      return null;
                  }

                  // Migrate if needed
                  var user = UserIdentity.Migrate(result.User);
                  if (user.SchemaVersion != result.User.SchemaVersion)
                  {
                      _logger.LogInformation("Migrated user from v{Old} to v{New}",
                          result.User.SchemaVersion, user.SchemaVersion);
                      await SaveUserInternalAsync(user, ct);
                  }

                  _cachedIdentity = user;
                  _logger.LogInformation("Loaded existing user: {UserId}", _cachedIdentity.Id);
                  return _cachedIdentity;
              }
              catch (OperationCanceledException)
              {
                  throw;
              }
              catch (JsonException ex)
              {
                  _logger.LogWarning(ex, "Corrupted user.json detected");
                  return await HandleCorruptedFileAsync(ct);
              }
              catch (JSException ex) when (attempt < MaxRetries)
              {
                  lastException = ex;
                  _logger.LogWarning(ex, "JS interop failed, attempt {Attempt}/{MaxRetries}",
                      attempt + 1, MaxRetries + 1);
              }
              catch (Exception ex)
              {
                  lastException = ex;
                  _logger.LogError(ex, "Unexpected error loading user identity");
                  break;
              }
          }

          _logger.LogError(lastException, "Failed to load user identity after {MaxRetries} retries", MaxRetries);
          return null;
      }

      /// <summary>
      /// Creates a new user identity and persists it.
      /// Thread-safe - prevents concurrent creation.
      /// </summary>
      public async Task<UserIdentity> CreateNewUserAsync(
          string? name,
          string? email,
          CancellationToken ct = default)
      {
          ThrowIfDisposed();

          await _lock.WaitAsync(ct);
          try
          {
              // Validate input
              var validation = Core.Validation.RegistrationValidator.Validate(name, email);
              if (!validation.IsValid)
              {
                  throw new ArgumentException(string.Join("; ", validation.Errors));
              }

              var user = UserIdentity.Create(name, email);
              await SaveUserInternalAsync(user, ct);
              _cachedIdentity = user;

              _logger.LogInformation("Created new user identity: {UserId}", user.Id);
              return user;
          }
          finally
          {
              _lock.Release();
          }
      }

      /// <summary>
      /// Persists user identity using atomic write pattern with retry.
      /// </summary>
      private async Task SaveUserInternalAsync(UserIdentity user, CancellationToken ct)
      {
          Exception? lastException = null;

          for (int attempt = 0; attempt <= MaxRetries; attempt++)
          {
              try
              {
                  if (attempt > 0)
                  {
                      var delay = RetryDelays[Math.Min(attempt - 1, RetryDelays.Length - 1)];
                      await Task.Delay(delay, ct);
                  }

                  await _js.InvokeVoidAsync(
                      TauriInvokePath,
                      ct,
                      TauriCommandSaveIdentity,
                      new { user }
                  );

                  _logger.LogDebug("User identity saved successfully");
                  return;
              }
              catch (OperationCanceledException)
              {
                  throw;
              }
              catch (JSException ex) when (attempt < MaxRetries)
              {
                  lastException = ex;
                  _logger.LogWarning(ex, "Save failed, attempt {Attempt}/{MaxRetries}",
                      attempt + 1, MaxRetries + 1);
              }
          }

          _logger.LogError(lastException, "Failed to save user identity");
          throw new InvalidOperationException("Could not persist user identity", lastException);
      }

      /// <summary>
      /// Handles corrupted user.json by backing up and forcing re-registration.
      /// </summary>
      private async Task<UserIdentity?> HandleCorruptedFileAsync(CancellationToken ct)
      {
          try
          {
              await _js.InvokeVoidAsync(
                  TauriInvokePath,
                  ct,
                  TauriCommandBackupCorrupted
              );

              _logger.LogWarning("Backed up corrupted user.json, user must re-register");
          }
          catch (Exception ex)
          {
              _logger.LogError(ex, "Failed to backup corrupted file");
          }

          return null; // Force re-registration
      }

      private void ThrowIfDisposed()
      {
          if (_disposed)
              throw new ObjectDisposedException(nameof(UserIdentityService));
      }

      public async ValueTask DisposeAsync()
      {
          if (_disposed) return;
          _disposed = true;

          _cachedIdentity = null;
          _lock.Dispose();

          await ValueTask.CompletedTask;
      }
  }