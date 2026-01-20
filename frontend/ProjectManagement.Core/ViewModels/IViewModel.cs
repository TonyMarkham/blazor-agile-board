namespace ProjectManagement.Core.ViewModels;

/// <summary>
/// Base interface for all view models.
/// View models combine immutable domain models with transient UI state.
/// </summary>
/// <typeparam name="TModel">The underlying domain model type</typeparam>
public interface IViewModel<out TModel> where TModel : class
{
    /// <summary>
    /// The underlying domain model (from server/store).
    /// This is immutable - UI changes create new ViewModels.
    /// </summary>
    TModel Model { get; }

    /// <summary>
    /// True when this item has pending changes being synced to the server.
    /// Used for optimistic UI feedback (shimmer effect, disabled buttons, etc.)
    /// </summary>
    bool IsPendingSync { get; }
}