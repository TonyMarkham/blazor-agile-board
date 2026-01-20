using Microsoft.Extensions.Logging;

namespace ProjectManagement.Services.Logging;

/// <summary>
///     Logger that includes correlation IDs for request tracing.
/// </summary>
public sealed class CorrelationIdLogger : ILogger
{
    private static readonly AsyncLocal<string?> _correlationId = new();
    private readonly string _categoryName;
    private readonly LogLevel _minLevel;

    public CorrelationIdLogger(string categoryName, LogLevel minLevel = LogLevel.Debug)
    {
        _categoryName = categoryName;
        _minLevel = minLevel;
    }

    public static string CorrelationId
    {
        get => _correlationId.Value ?? Guid.NewGuid().ToString("N")[..8];
        set => _correlationId.Value = value;
    }

    public IDisposable? BeginScope<TState>(TState state) where TState : notnull
    {
        return null;
    }

    public bool IsEnabled(LogLevel logLevel)
    {
        return logLevel >= _minLevel;
    }

    public void Log<TState>(
        LogLevel logLevel,
        EventId eventId,
        TState state,
        Exception? exception,
        Func<TState, Exception?, string> formatter)
    {
        if (!IsEnabled(logLevel))
            return;

        var message = formatter(state, exception);
        var timestamp = DateTime.UtcNow.ToString("HH:mm:ss.fff");
        var level = logLevel.ToString()[..3].ToUpper();
        var category = _categoryName.Split('.').LastOrDefault() ?? _categoryName;

        Console.WriteLine($"[{timestamp}] [{level}] [{CorrelationId}] {category}: {message}");

        if (exception != null) Console.WriteLine($"  Exception: {exception.GetType().Name}: {exception.Message}");
    }
}