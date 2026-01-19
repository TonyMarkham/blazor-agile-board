# Session 20.6: Comprehensive Test Suite

**Parent Plan**: [session-20-plan.md](session-20-plan.md)
**Target**: ~30k tokens
**Prerequisites**: Session 20.5 complete (WASM host & observability)

---

## Scope

**Goal**: 100+ tests covering all components

**Estimated Tokens**: ~30k

### Test Project Structure

```
frontend/
├── ProjectManagement.Core.Tests/
│   ├── Converters/
│   │   └── ProtoConverterTests.cs
│   ├── Validation/
│   │   └── ValidatorTests.cs
│   └── Models/
│       └── WorkItemTests.cs
├── ProjectManagement.Services.Tests/
│   ├── WebSocket/
│   │   ├── WebSocketClientTests.cs
│   │   ├── PendingRequestTests.cs
│   │   └── ConnectionHealthTrackerTests.cs
│   ├── Resilience/
│   │   ├── CircuitBreakerTests.cs
│   │   ├── RetryPolicyTests.cs
│   │   └── ReconnectionServiceTests.cs
│   ├── State/
│   │   ├── WorkItemStoreTests.cs
│   │   └── OptimisticUpdateTests.cs
│   ├── Mocks/
│   │   └── MockWebSocketConnection.cs
│   └── PropertyTests/
│       ├── ProtoConverterPropertyTests.cs
│       └── CircuitBreakerPropertyTests.cs
```

### Test Categories (Target: 100+ tests)

| Category | Test Count | Focus |
|----------|------------|-------|
| ProtoConverter | 20 | All entity conversions, edge cases |
| Validators | 15 | All validation rules |
| WebSocketClient | 20 | Connect, send, receive, timeout |
| CircuitBreaker | 15 | State transitions, thread safety |
| RetryPolicy | 10 | Backoff, jitter, max attempts |
| WorkItemStore | 15 | CRUD, optimistic updates, rollback |
| Property Tests | 10 | Random input fuzzing |
| **Total** | **105** | |

### Sample Test Files

```csharp
// CircuitBreakerTests.cs
namespace ProjectManagement.Services.Tests.Resilience;

public class CircuitBreakerTests
{
    private readonly CircuitBreaker _sut;
    private readonly Mock<ILogger<CircuitBreaker>> _logger;

    public CircuitBreakerTests()
    {
        _logger = new Mock<ILogger<CircuitBreaker>>();
        var options = Options.Create(new CircuitBreakerOptions
        {
            FailureThreshold = 3,
            OpenDuration = TimeSpan.FromMilliseconds(100),
            HalfOpenSuccessThreshold = 2,
            FailureWindow = TimeSpan.FromSeconds(60)
        });
        _sut = new CircuitBreaker(options, _logger.Object);
    }

    [Fact]
    public void InitialState_IsClosed()
    {
        _sut.State.Should().Be(CircuitState.Closed);
    }

    [Fact]
    public void AllowRequest_WhenClosed_ReturnsTrue()
    {
        _sut.AllowRequest().Should().BeTrue();
    }

    [Fact]
    public void RecordFailure_BelowThreshold_StaysClosed()
    {
        _sut.RecordFailure();
        _sut.RecordFailure();

        _sut.State.Should().Be(CircuitState.Closed);
    }

    [Fact]
    public void RecordFailure_AtThreshold_OpensCircuit()
    {
        _sut.RecordFailure();
        _sut.RecordFailure();
        _sut.RecordFailure();

        _sut.State.Should().Be(CircuitState.Open);
    }

    [Fact]
    public void AllowRequest_WhenOpen_ReturnsFalse()
    {
        // Open the circuit
        for (int i = 0; i < 3; i++)
            _sut.RecordFailure();

        _sut.AllowRequest().Should().BeFalse();
    }

    [Fact]
    public async Task AllowRequest_AfterOpenDuration_TransitionsToHalfOpen()
    {
        // Open the circuit
        for (int i = 0; i < 3; i++)
            _sut.RecordFailure();

        // Wait for open duration
        await Task.Delay(150);

        _sut.AllowRequest().Should().BeTrue();
        _sut.State.Should().Be(CircuitState.HalfOpen);
    }

    [Fact]
    public void RecordSuccess_InHalfOpen_ClosesAfterThreshold()
    {
        // Open then transition to half-open
        for (int i = 0; i < 3; i++)
            _sut.RecordFailure();

        // Force half-open by manipulating time (or use test options)
        // ... implementation depends on design

        _sut.RecordSuccess();
        _sut.RecordSuccess();

        _sut.State.Should().Be(CircuitState.Closed);
    }

    [Fact]
    public void RecordFailure_InHalfOpen_ReopensCircuit()
    {
        // Setup half-open state...

        _sut.RecordFailure();

        _sut.State.Should().Be(CircuitState.Open);
    }

    [Fact]
    public async Task ExecuteAsync_WhenCircuitOpen_ThrowsCircuitOpenException()
    {
        // Open the circuit
        for (int i = 0; i < 3; i++)
            _sut.RecordFailure();

        var act = () => _sut.ExecuteAsync(ct => Task.FromResult(42));

        await act.Should().ThrowAsync<CircuitOpenException>();
    }

    [Fact]
    public async Task ExecuteAsync_RecordsSuccessOnCompletion()
    {
        await _sut.ExecuteAsync(ct => Task.FromResult(42));

        // State should still be closed with 0 failures
        _sut.State.Should().Be(CircuitState.Closed);
    }

    [Fact]
    public async Task ExecuteAsync_RecordsFailureOnException()
    {
        var act = () => _sut.ExecuteAsync<int>(
            ct => throw new IOException("Network error"));

        await act.Should().ThrowAsync<IOException>();

        // Should have recorded 1 failure
        // (verify by recording 2 more and checking state)
        _sut.RecordFailure();
        _sut.RecordFailure();
        _sut.State.Should().Be(CircuitState.Open);
    }

    [Fact]
    public async Task ExecuteAsync_DoesNotRecordValidationExceptionAsFailure()
    {
        var act = () => _sut.ExecuteAsync<int>(
            ct => throw new ValidationException("title", "Required"));

        await act.Should().ThrowAsync<ValidationException>();

        // Should NOT have recorded as failure
        _sut.RecordFailure();
        _sut.RecordFailure();
        _sut.State.Should().Be(CircuitState.Closed); // Still closed, only 2 failures
    }
}

// ProtoConverterPropertyTests.cs
namespace ProjectManagement.Services.Tests.PropertyTests;

public class ProtoConverterPropertyTests
{
    [Property]
    public Property WorkItem_RoundTrip_PreservesData()
    {
        return Prop.ForAll(
            Arb.Generate<Guid>().ToArbitrary(),
            Arb.Generate<string>().Where(s => !string.IsNullOrEmpty(s)).ToArbitrary(),
            (id, title) =>
            {
                var original = new WorkItem
                {
                    Id = id,
                    ItemType = WorkItemType.Task,
                    ProjectId = Guid.NewGuid(),
                    Title = title,
                    Status = "backlog",
                    Priority = "medium",
                    Version = 1,
                    CreatedAt = DateTime.UtcNow,
                    UpdatedAt = DateTime.UtcNow,
                    CreatedBy = Guid.NewGuid(),
                    UpdatedBy = Guid.NewGuid()
                };

                var proto = ProtoConverter.ToProto(original);
                var roundTripped = ProtoConverter.ToDomain(proto);

                return roundTripped.Id == original.Id
                    && roundTripped.Title == original.Title
                    && roundTripped.ItemType == original.ItemType;
            });
    }

    [Property]
    public Property Timestamp_RoundTrip_WithinOneSecond()
    {
        return Prop.ForAll(
            Arb.Generate<DateTime>()
                .Where(d => d > DateTime.UnixEpoch && d < DateTime.UtcNow.AddYears(100))
                .ToArbitrary(),
            timestamp =>
            {
                var workItem = new WorkItem
                {
                    Id = Guid.NewGuid(),
                    ItemType = WorkItemType.Task,
                    ProjectId = Guid.NewGuid(),
                    Title = "Test",
                    CreatedAt = timestamp,
                    UpdatedAt = timestamp,
                    CreatedBy = Guid.NewGuid(),
                    UpdatedBy = Guid.NewGuid()
                };

                var proto = ProtoConverter.ToProto(workItem);
                var roundTripped = ProtoConverter.ToDomain(proto);

                var diff = Math.Abs((roundTripped.CreatedAt - timestamp).TotalSeconds);
                return diff <= 1; // Within 1 second due to Unix timestamp precision
            });
    }
}
```

### Success Criteria for 20.6

- [ ] 100+ tests written
- [ ] All tests pass: `dotnet test frontend/`
- [ ] Property-based tests for converters
- [ ] Circuit breaker state machine fully tested
- [ ] WebSocket client tested with mocks
- [ ] State store tested with optimistic updates

---

## Final File Count Summary

| Sub-Session | Files | Cumulative |
|-------------|-------|------------|
| 20.1 Foundation | 55 | 55 |
| 20.2 WebSocket | 9 | 64 |
| 20.3 Resilience | 7 | 71 |
| 20.4 State | 4 | 75 |
| 20.5 WASM Host | 8 | 83 |
| 20.6 Tests | 15 | **98** |

---

## Production-Grade Checklist

| Requirement | Status |
|-------------|--------|
| Entity interface hierarchy (IEntity, IAuditable, etc.) | ✅ |
| Generic store interfaces (IEntityStore, IProjectScopedStore, ISprintStore) | ✅ |
| Exception hierarchy with user-safe messages | ✅ |
| Circuit breaker (Closed/Open/HalfOpen) | ✅ |
| Retry with exponential backoff + jitter (using Random.Shared) | ✅ |
| Reconnection with subscription rehydration | ✅ |
| Structured logging with correlation IDs | ✅ |
| Thread-safe state management | ✅ |
| CancellationToken on all async ops | ✅ |
| Proper IDisposable/IAsyncDisposable | ✅ |
| Ping/pong latency tracking (per-message correlation) | ✅ |
| Timestamp precision documented (Unix seconds) | ✅ |
| Input validation before sending | ✅ |
| Connection health monitoring | ✅ |
| Error boundaries in UI | ✅ |
| 100+ comprehensive tests | ✅ |
| Property-based tests | ✅ |
| No TODOs in production code | ✅ |

---

## Definition of Done

Session 20 is complete when:

- [ ] Solution builds: `dotnet build frontend/ProjectManagement.sln` (zero warnings)
- [ ] Tests pass: `dotnet test frontend/` (100+ tests, all green)
- [ ] WASM app runs: `dotnet run --project frontend/ProjectManagement.Wasm`
- [ ] WebSocket connects to backend (pm-server)
- [ ] Circuit breaker protects against cascading failures
- [ ] Retry logic handles transient failures
- [ ] Reconnection handles disconnects automatically
- [ ] State management with optimistic updates works
- [ ] Error boundaries display user-friendly messages
- [ ] All code follows project conventions
- [ ] No TODOs in production code
- [ ] Structured logging throughout

**Target Score**: 9.25+/10 production-grade

---

## Comparison with Session 10 Backend

| Feature | Session 10 (Rust) | Session 20 (C#) |
|---------|-------------------|-----------------|
| Circuit Breaker | ✅ | ✅ |
| Error Boundary | ✅ | ✅ |
| Structured Logging | ✅ | ✅ |
| Request Context | ✅ | ✅ (Correlation ID) |
| Message Validation | ✅ | ✅ |
| Retry Logic | ✅ | ✅ |
| Health Monitoring | ✅ | ✅ |
| Thread Safety | ✅ | ✅ |
| Test Count | 166 | 100+ |
| Production Score | 9.6/10 | 9.25+/10 (target) |
