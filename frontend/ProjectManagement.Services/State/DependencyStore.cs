using System.Collections.Concurrent;
using Microsoft.Extensions.Logging;
using ProjectManagement.Core.Exceptions;
using ProjectManagement.Core.Interfaces;
using ProjectManagement.Core.Models;
using ProjectManagement.Services.Notifications;

namespace ProjectManagement.Services.State;

/// <summary>
/// Dependency store with optimistic updates.
/// Tracks both blocking and blocked relationships.
/// </summary>
public sealed class DependencyStore : IDependencyStore
{
    private readonly IWebSocketClient _client;
    private readonly IToastService _toast;
    private readonly ILogger<DependencyStore> _logger;
    private readonly Guid _currentUserId;

    // State: keyed by dependency ID
    private readonly ConcurrentDictionary<Guid, Dependency> _dependencies = new();
    private readonly ConcurrentDictionary<Guid, Dependency> _rollbackState = new();
    private readonly ConcurrentDictionary<Guid, bool> _pendingUpdates = new();
    private bool _disposed;

    public event Action? OnChanged;

    public DependencyStore(
        IWebSocketClient client,
        AppState appState,
        IToastService toast,
        ILogger<DependencyStore> logger)
    {
        _client = client ?? throw new ArgumentNullException(nameof(client));
        _currentUserId = appState.CurrentUser?.Id ?? throw new InvalidOperationException("CurrentUser not set");
        _toast = toast ?? throw new ArgumentNullException(nameof(toast));
        _logger = logger ?? throw new ArgumentNullException(nameof(logger));

        _client.OnDependencyCreated += HandleDependencyCreated;
        _client.OnDependencyDeleted += HandleDependencyDeleted;
    }

    public IReadOnlyList<Dependency> GetBlocking(Guid workItemId)
    {
        return _dependencies.Values
            .Where(d => d.BlockedItemId == workItemId && d.DeletedAt == null)
            .OrderByDescending(d => d.CreatedAt)
            .ToList();
    }

    public IReadOnlyList<Dependency> GetBlocked(Guid workItemId)
    {
        return _dependencies.Values
            .Where(d => d.BlockingItemId == workItemId && d.DeletedAt == null)
            .OrderByDescending(d => d.CreatedAt)
            .ToList();
    }

    public bool IsBlocked(Guid workItemId)
    {
        return _dependencies.Values
            .Any(d => d.BlockedItemId == workItemId
                      && d.DeletedAt == null
                      && d.Type == DependencyType.Blocks);
    }

    public int GetBlockingCount(Guid workItemId)
    {
        return _dependencies.Values
            .Count(d => d.BlockedItemId == workItemId
                        && d.DeletedAt == null
                        && d.Type == DependencyType.Blocks);
    }

    public bool IsPending(Guid dependencyId) => _pendingUpdates.ContainsKey(dependencyId);

    public async Task<Dependency> CreateAsync(CreateDependencyRequest request, CancellationToken ct)
    {
        ThrowIfDisposed();

        // Optimistic: Create temp dependency
        var tempId = Guid.NewGuid();
        var optimistic = new Dependency
        {
            Id = tempId,
            BlockingItemId = request.BlockingItemId,
            BlockedItemId = request.BlockedItemId,
            Type = request.Type,
            CreatedAt = DateTime.UtcNow,
            CreatedBy = _currentUserId,
        };

        _dependencies[tempId] = optimistic;
        _pendingUpdates[tempId] = true;
        NotifyChanged();

        try
        {
            var result = await _client.CreateDependencyAsync(request, ct);

            _dependencies.TryRemove(tempId, out _);
            _pendingUpdates.TryRemove(tempId, out _);
            _dependencies[result.Id] = result;

            NotifyChanged();
            _toast.ShowSuccess("Dependency added");
            _logger.LogInformation("Created dependency {DepId}: {Blocking} -> {Blocked}",
                result.Id, request.BlockingItemId, request.BlockedItemId);
            return result;
        }
        catch (ValidationException ex)
        {
            _dependencies.TryRemove(tempId, out _);
            _pendingUpdates.TryRemove(tempId, out _);
            NotifyChanged();

            _toast.ShowError(ex.UserMessage, "Validation Error");
            throw;
        }
        catch (Exception ex)
        {
            _dependencies.TryRemove(tempId, out _);
            _pendingUpdates.TryRemove(tempId, out _);
            NotifyChanged();

            _logger.LogWarning(ex, "Failed to create dependency {Blocking} -> {Blocked}",
                request.BlockingItemId, request.BlockedItemId);
            _toast.ShowError("Failed to create dependency. Please try again.");
            throw;
        }
    }

    public async Task DeleteAsync(Guid dependencyId, CancellationToken ct)
    {
        ThrowIfDisposed();

        if (!_dependencies.TryGetValue(dependencyId, out var existing))
        {
            return; // Already deleted
        }

        // Optimistic: Mark as deleted
        var optimistic = existing with { DeletedAt = DateTime.UtcNow };

        _rollbackState[dependencyId] = existing;
        _dependencies[dependencyId] = optimistic;
        _pendingUpdates[dependencyId] = true;
        NotifyChanged();

        try
        {
            await _client.DeleteDependencyAsync(dependencyId, ct);

            _dependencies.TryRemove(dependencyId, out _);
            _rollbackState.TryRemove(dependencyId, out _);
            _pendingUpdates.TryRemove(dependencyId, out _);

            NotifyChanged();
            _toast.ShowSuccess("Dependency removed");
            _logger.LogInformation("Deleted dependency {DepId}", dependencyId);
        }
        catch (ValidationException ex)
        {
            if (_rollbackState.TryRemove(dependencyId, out var rollback))
            {
                _dependencies[dependencyId] = rollback;
            }

            _pendingUpdates.TryRemove(dependencyId, out _);
            NotifyChanged();

            _toast.ShowError(ex.UserMessage, "Validation Error");
            throw;
        }
        catch (Exception ex)
        {
            if (_rollbackState.TryRemove(dependencyId, out var rollback))
            {
                _dependencies[dependencyId] = rollback;
            }

            _pendingUpdates.TryRemove(dependencyId, out _);
            NotifyChanged();

            _logger.LogWarning(ex, "Failed to delete dependency {DepId}", dependencyId);
            _toast.ShowError("Failed to remove dependency. Please try again.");
            throw;
        }
    }

    public async Task RefreshAsync(Guid workItemId, CancellationToken ct)
    {
        ThrowIfDisposed();

        var (blocking, blocked) = await _client.GetDependenciesAsync(workItemId, ct);

        // Add/update all fetched dependencies
        foreach (var dep in blocking.Concat(blocked))
        {
            _dependencies[dep.Id] = dep;
        }

        NotifyChanged();
    }

    private void HandleDependencyCreated(Dependency dependency)
    {
        if (_pendingUpdates.ContainsKey(dependency.Id)) return;

        _dependencies[dependency.Id] = dependency;
        NotifyChanged();
    }

    private void HandleDependencyDeleted(Guid dependencyId, Guid blockingItemId, Guid blockedItemId)
    {
        if (_pendingUpdates.ContainsKey(dependencyId)) return;

        _dependencies.TryRemove(dependencyId, out _);
        NotifyChanged();
    }

    private void NotifyChanged() => OnChanged?.Invoke();

    private void ThrowIfDisposed()
    {
        if (_disposed)
            throw new ObjectDisposedException(nameof(DependencyStore));
    }

    public void Dispose()
    {
        if (_disposed) return;
        _disposed = true;

        _client.OnDependencyCreated -= HandleDependencyCreated;
        _client.OnDependencyDeleted -= HandleDependencyDeleted;
    }
}