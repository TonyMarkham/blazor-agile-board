using ProjectManagement.Core.Exceptions;

namespace ProjectManagement.Services.WebSocket;

using Pm = Core.Proto;

internal sealed class PendingRequest : IDisposable
{
    private readonly CancellationTokenRegistration _registration;

    private readonly CancellationTokenSource _timeoutCts;
    private bool _disposed;

    public PendingRequest(string messageId, TimeSpan timeout, CancellationToken externalCt)
    {
        MessageId = messageId;
        SentAt = DateTime.UtcNow;
        Timeout = timeout;
        CompletionSource = new TaskCompletionSource<Pm.WebSocketMessage>(
            TaskCreationOptions.RunContinuationsAsynchronously);

        _timeoutCts = CancellationTokenSource.CreateLinkedTokenSource(externalCt);
        _timeoutCts.CancelAfter(timeout);

        _registration = _timeoutCts.Token.Register(() =>
        {
            CompletionSource.TrySetException(
                new RequestTimeoutException(messageId, timeout));
        });
    }

    public string MessageId { get; }
    public DateTime SentAt { get; }
    public TimeSpan Timeout { get; }
    public TaskCompletionSource<Pm.WebSocketMessage> CompletionSource { get; }

    public void Dispose()
    {
        if (_disposed) return;
        _disposed = true;

        _registration.Dispose();
        _timeoutCts.Dispose();
    }

    public void Complete(Pm.WebSocketMessage response)
    {
        CompletionSource.TrySetResult(response);
    }

    public void Fail(Exception ex)
    {
        CompletionSource.TrySetException(ex);
    }
}