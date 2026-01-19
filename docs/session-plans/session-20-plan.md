# Session 20: Blazor Foundation - Implementation Plan

**Goal**: Create production-grade Blazor frontend with WebSocket client, state management, and resilience patterns

**Target Quality**: 9.25+/10 production-grade (matching Session 10 backend quality)

**Total Estimated Tokens**: ~190k (split across 7 sub-sessions)

**Sub-session Design Philosophy**:
- Each sub-session targets **10-35k tokens** (conservative)
- Historical overruns: 1.5-2.7x estimates → still fits in 50-75k context with room to spare
- Smaller context = better Claude performance + human sense of progress
- Each sub-session is a complete, testable deliverable

**Prerequisites**: Session 10 complete (backend with 166 tests passing)

---

## Quality Standards

This plan targets the same production-grade quality as Session 10:

| Requirement | Implementation |
|-------------|----------------|
| Circuit breaker | Client-side circuit breaker for server failures |
| Error boundaries | Catch and handle all exceptions gracefully |
| Retry logic | Exponential backoff with jitter |
| Structured logging | Correlation IDs carried through all operations |
| Optimistic updates | UI updates immediately, rolls back on failure |
| Thread safety | ConcurrentDictionary, Interlocked operations |
| Proper disposal | IAsyncDisposable, CancellationTokenSource cleanup |
| Type safety | Nullable enabled, exhaustive switch expressions |
| Validation | Client-side validation before sending to server |
| Test coverage | 100+ tests including property-based tests |

---

## Sub-Session Breakdown

This plan has been split into sub-sessions to fit within token budgets:

| Session | Scope | Est. Tokens | Files | Status |
|---------|-------|-------------|-------|--------|
| **[20.01](session-20.01-plan.md)** | Database FK constraint fixes (structural debt) | ~10k | ~3 | 🔜 Next |
| **[20.1](session-20.1-plan.md)** | Project structure, Protobuf, Domain models | ~30k | ~55 | Planned |
| **[20.2](session-20.2-plan.md)** | WebSocket client foundation | ~35k | ~9 | Planned |
| **[20.3](session-20.3-plan.md)** | Resilience patterns (circuit breaker, retry, health) | ~30k | ~7 | Planned |
| **[20.4](session-20.4-plan.md)** | State management with thread safety | ~30k | ~4 | Planned |
| **[20.5](session-20.5-plan.md)** | WASM host, error boundaries, observability | ~25k | ~8 | Planned |
| **[20.6](session-20.6-plan.md)** | Comprehensive test suite (100+ tests) | ~30k | ~15 | Planned |

---

## Session 20.01: Database FK Constraint Fixes

**Files Created:**
- `backend/crates/pm-db/migrations/20260119000001_add_work_item_fks.sql` - FK constraints

**Files Modified:**
- `backend/crates/pm-db/tests/work_item_repository_tests.rs` - FK tests
- `docs/database-relationships.md` - Documentation

**Verification:** `cargo test --workspace`

---

## Session 20.1: Foundation

**Files Created:**
- `frontend/ProjectManagement.sln` - Solution file
- `frontend/Directory.Build.props` - Shared build properties
- `frontend/Directory.Packages.props` - Central package management
- `frontend/ProjectManagement.Core/` - Models, interfaces, validation (~55 files)

**Verification:** `dotnet build frontend/ProjectManagement.sln`

---

## Session 20.2: WebSocket Client Foundation

**Files Created:**
- `frontend/ProjectManagement.Services/WebSocket/` - WebSocket implementation
- `WebSocketClient.cs`, `PendingRequest.cs`, `ConnectionHealthTracker.cs`

**Verification:** WebSocket connects and sends/receives protobuf messages

---

## Session 20.3: Resilience Patterns

**Files Created:**
- `frontend/ProjectManagement.Services/Resilience/`
- `CircuitBreaker.cs`, `RetryPolicy.cs`, `ReconnectionService.cs`

**Verification:** Circuit breaker state machine works, retry with exponential backoff

---

## Session 20.4: State Management

**Files Created:**
- `frontend/ProjectManagement.Services/State/`
- `WorkItemStore.cs`, `SprintStore.cs`, `OptimisticUpdate.cs`, `AppState.cs`

**Verification:** Optimistic updates apply and rollback correctly

---

## Session 20.5: WASM Host & Observability

**Files Created:**
- `frontend/ProjectManagement.Wasm/Program.cs` - DI setup
- `frontend/ProjectManagement.Components/` - Error boundaries, logging

**Verification:** WASM app loads and connects to backend

---

## Session 20.6: Comprehensive Test Suite

**Files Created:**
- `frontend/ProjectManagement.Core.Tests/` - Model and validation tests
- `frontend/ProjectManagement.Services.Tests/` - WebSocket, resilience, state tests

**Target:** 100+ tests including property-based tests

**Verification:** `dotnet test frontend/`

---

## Final File Count Summary

| Sub-Session | Files | Cumulative |
|-------------|-------|------------|
| 20.01 DB Fix | 3 | 3 |
| 20.1 Foundation | 55 | 58 |
| 20.2 WebSocket | 9 | 67 |
| 20.3 Resilience | 7 | 74 |
| 20.4 State | 4 | 78 |
| 20.5 WASM Host | 8 | 86 |
| 20.6 Tests | 15 | **101** |

---

## Production-Grade Checklist

| Requirement | Status |
|-------------|--------|
| Entity interface hierarchy (IEntity, IAuditable, etc.) | ✅ Planned |
| Exception hierarchy with correlation IDs | ✅ Planned |
| Validation framework with error messages | ✅ Planned |
| Circuit breaker matching pm-config thresholds | ✅ Planned |
| Retry with exponential backoff + jitter | ✅ Planned |
| Reconnection with subscription rehydration | ✅ Planned |
| Per-ping latency tracking | ✅ Planned |
| Optimistic updates with rollback | ✅ Planned |
| Thread-safe state with ConcurrentDictionary | ✅ Planned |
| Property-based tests for converters | ✅ Planned |
| 100+ unit/integration tests | ✅ Planned |

---

## Key Alignments with Backend

These values are aligned with `pm-config` crate defaults:

| Setting | Value | pm-config Source |
|---------|-------|------------------|
| `MaxTitleLength` | 200 | `DEFAULT_MAX_TITLE_LENGTH` |
| `MaxDescriptionLength` | 10000 | `DEFAULT_MAX_DESCRIPTION_LENGTH` |
| `RetryPolicy.MaxDelay` | 5s | `DEFAULT_MAX_DELAY_SECS` |
| `CircuitBreaker.FailureThreshold` | 5 | `DEFAULT_FAILURE_THRESHOLD` |
| `CircuitBreaker.OpenDuration` | 30s | `DEFAULT_OPEN_DURATION_SECS` |
| `CircuitBreaker.HalfOpenSuccessThreshold` | 3 | `DEFAULT_HALF_OPEN_SUCCESS_THRESHOLD` |
| `HeartbeatInterval` | 30s | `DEFAULT_HEARTBEAT_INTERVAL_SECS` |

---

*Update this document after each sub-session with completion status.*
