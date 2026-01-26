/// <summary>
/// Tracks child item progress for Epic and Story cards.
/// Computed from AppState cache, not fetched separately.
/// </summary>
public record ChildProgress
{
    /// <summary>
    /// Count of child items by status (e.g., "todo" -> 3, "done" -> 5)
    /// </summary>
    public IReadOnlyDictionary<string, int> ByStatus { get; init; } = new Dictionary<string, int>();

    /// <summary>
    /// Total number of child items
    /// </summary>
    public int Total { get; init; }

    /// <summary>
    /// Number of completed items (status = "done")
    /// </summary>
    public int Completed { get; init; }

    /// <summary>
    /// Progress percentage (0-100)
    /// </summary>
    public int Percentage => Total > 0 ? (Completed * 100) / Total : 0;

    /// <summary>
    /// Display string like "3/12"
    /// </summary>
    public string DisplayText => $"{Completed}/{Total}";

    /// <summary>
    /// True if there are any child items to show progress for
    /// </summary>
    public bool HasChildren => Total > 0;

    /// <summary>
    /// Empty progress (no children)
    /// </summary>
    public static ChildProgress Empty { get; } = new();
}