using Microsoft.Extensions.Logging;

namespace ProjectManagement.Services.Logging;

/// <summary>
///     Logger provider that adds correlation IDs to all log messages.
/// </summary>
public sealed class CorrelationIdLoggerProvider : ILoggerProvider
{
    private readonly LogLevel _minLevel;

    public CorrelationIdLoggerProvider(LogLevel minLevel = LogLevel.Debug)
    {
        _minLevel = minLevel;
    }

    public ILogger CreateLogger(string categoryName)
    {
        return new CorrelationIdLogger(categoryName, _minLevel);
    }

    public void Dispose()
    {
    }
}