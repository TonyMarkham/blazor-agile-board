namespace ProjectManagement.Core.Models;

/// <summary>
/// Represents a single field change in a work item update.
/// Used for tracking changes and displaying in activity feeds.
/// </summary>
public sealed record FieldChange(
    string FieldName,
    string? OldValue,
    string? NewValue)
{
    /// <summary>
    /// Human-readable description of the change.
    /// </summary>
    public string Description => (OldValue, NewValue) switch
    {
        (null, not null) => $"Set {FieldName} to '{NewValue}'",
        (not null, null) => $"Cleared {FieldName}",
        _ => $"Changed {FieldName} from '{OldValue}' to '{NewValue}'"
    };
}