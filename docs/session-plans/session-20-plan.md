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
| **[20.01](session-20.01-plan.md)** | Database FK constraint fixes (structural debt) | ~10k | 4 | ✅ Complete |
| **[20.1](session-20.1-plan.md)** | Project structure, Protobuf, Domain models | ~30k | ~55 | ✅ Complete |
| **[20.2](session-20.2-plan.md)** | WebSocket client foundation | ~35k | 7 | ✅ Complete |
| **[20.3](session-20.3-plan.md)** | Resilience patterns (circuit breaker, retry, health) | ~30k | 8 | ✅ Complete |
| **[20.4](session-20.4-plan.md)** | State management with thread safety | ~30k | 4 | ✅ Complete |
| **[20.5](session-20.5-plan.md)** | WASM host, error boundaries, observability | ~25k | ~8 | Planned |
| **[20.6](session-20.6-plan.md)** | Comprehensive test suite (100+ tests) | ~30k | ~15 | Planned |

---

## Session 20.01: Database FK Constraint Fixes ✅

**Status**: Complete (2026-01-19) - Out of sequence, before Session 20.2

**Reason for Out-of-Sequence Completion**: Originally deferred as non-blocking for frontend development, but completed early to resolve structural database debt.

**What Was Accomplished**:
- ✅ Added FK constraints: `pm_work_items.sprint_id` → `pm_sprints.id` (ON DELETE SET NULL)
- ✅ Added FK constraints: `pm_work_items.assignee_id` → `users.id` (ON DELETE SET NULL)
- ✅ Handled circular dependency between `pm_work_items` ↔ `pm_sprints`
- ✅ 4 new FK constraint tests (SET NULL and CASCADE behaviors)
- ✅ Complete FK documentation in `database-relationships.md`

**Key Lesson**: SQLite auto-updates FK references during table renames. Original migration plan (rename approach) failed catastrophically. Solution: Drop and recreate tables with FKs pointing to final names.

**Files Modified/Created**:
- `backend/crates/pm-db/migrations/20260119194912_add_work_item_fks.sql` - Migration (drop/recreate approach)
- `backend/crates/pm-db/tests/work_item_repository_tests.rs` - 4 FK tests
- `backend/crates/pm-db/README.md` - Workflow documentation rewrite
- `docs/database-relationships.md` - Complete FK documentation

**Test Results**: ✅ 157 tests passing (4 new FK tests)

**Time**: ~2 hours (including debugging migration failures)

---

## Session 20.1: Foundation ✅

**Status**: Complete (2026-01-19) - Commit c5cf698

**Files Created:**
- `frontend/ProjectManagement.sln` - Solution file
- `frontend/Directory.Build.props` - Shared build properties
- `frontend/Directory.Packages.props` - Central package management
- `frontend/ProjectManagement.Core/` - Models, interfaces, validation (~55 files)

**Actual Implementation:**
- .NET 10.0 with latest stable packages (Protobuf 3.33.4, Radzen 8.6.2)
- Shared proto file from monorepo root
- Type aliases in ProtoConverter to avoid naming collisions
- 445KB generated Protobuf C# code

**Verification:** ✅ `dotnet build frontend/ProjectManagement.sln` - 0 warnings, 0 errors

---

## Session 20.2: WebSocket Client Foundation ✅

**Status**: Complete (2026-01-19)

**Files Created:**
- `frontend/ProjectManagement.Services/WebSocket/WebSocketOptions.cs` - Configuration with constants
- `frontend/ProjectManagement.Services/WebSocket/PendingRequest.cs` - Request/response correlation with timeout
- `frontend/ProjectManagement.Services/WebSocket/IWebSocketConnection.cs` - Modern .NET abstraction (ValueTask, ValueWebSocketReceiveResult)
- `frontend/ProjectManagement.Services/WebSocket/BrowserWebSocketConnection.cs` - Production implementation
- `frontend/ProjectManagement.Services/WebSocket/WebSocketClient.cs` - Core client with thread safety, heartbeat, disposal
- `frontend/ProjectManagement.Services/WebSocket/ConnectionHealthTracker.cs` - Per-message latency tracking
- `frontend/ProjectManagement.Services/_Imports.cs` - Global using statements

**Files Modified:**
- `frontend/ProjectManagement.Core/Models/ConnectionState.cs` - Moved from Exceptions to Models (proper location)
- `docs/session-plans/session-20.2-plan.md` - Fixed all namespace issues, modern .NET types, dependency order

**Key Fixes During Implementation:**
- Fixed protobuf namespace from `ProjectManagement.Protos` to `ProjectManagement.Core.Proto`
- Updated to modern .NET WebSocket types (ValueTask, ValueWebSocketReceiveResult)
- Moved ConnectionState enum from Exceptions to Models namespace
- Fixed dependency order (ConnectionHealthTracker before WebSocketClient)
- Two-constructor pattern for internal test seam (public + internal)
- Added stub implementations for unimplemented IWebSocketClient methods

**Verification:** ✅ `dotnet build frontend/ProjectManagement.slnx` - 0 warnings, 0 errors

---

## Session 20.3: Resilience Patterns ✅

**Status**: Complete (2026-01-19)

**Files Created:**
- `frontend/ProjectManagement.Services/Resilience/CircuitBreakerOptions.cs` - Configuration with constants
- `frontend/ProjectManagement.Services/Resilience/CircuitState.cs` - State enum
- `frontend/ProjectManagement.Services/Resilience/CircuitBreaker.cs` - Thread-safe circuit breaker
- `frontend/ProjectManagement.Services/Resilience/RetryPolicyOptions.cs` - Configuration with constants
- `frontend/ProjectManagement.Services/Resilience/RetryPolicy.cs` - Retry with exponential backoff + jitter
- `frontend/ProjectManagement.Services/Resilience/ReconnectionOptions.cs` - Configuration with constants
- `frontend/ProjectManagement.Services/Resilience/ReconnectionService.cs` - Auto-reconnect with subscription rehydration
- `frontend/ProjectManagement.Services/Resilience/ResilientWebSocketClient.cs` - Decorator wrapper

**Key Features:**
- Circuit breaker with automatic state transitions (Closed → Open → HalfOpen → Closed)
- Retry policy with exponential backoff (100ms → 5s max) and jitter
- Reconnection service with up to 10 attempts and subscription rehydration
- Smart error filtering (validation errors don't trip circuit)
- Constants for all configuration values (no magic numbers)
- Thread-safe operations with proper locking
- Aligned with backend `pm-config` defaults

**Verification:** ✅ `dotnet build frontend/` - 0 warnings, 0 errors (4.5s)

---

## Session 20.4: State Management ✅

**Status**: Complete (2026-01-19)

**Files Created:**
- `frontend/ProjectManagement.Services/State/OptimisticUpdate.cs` - Generic pending update tracker
- `frontend/ProjectManagement.Services/State/WorkItemStore.cs` - Thread-safe store with optimistic updates
- `frontend/ProjectManagement.Services/State/SprintStore.cs` - Sprint state (local-only until Session 40)
- `frontend/ProjectManagement.Services/State/AppState.cs` - Root state container with event aggregation

**Key Features:**
- Thread-safe with ConcurrentDictionary (lock-free concurrency)
- Optimistic update pattern: apply locally → confirm → rollback on failure
- Temporary IDs for creates, replaced with server IDs on confirmation
- Event handlers skip items with pending updates to avoid overwrites
- Soft delete support (filter DeletedAt == null in all queries)
- Event aggregation via AppState.OnStateChanged
- Two-phase initialization (connect → load project)

**Verification:** ✅ `dotnet build frontend/` - 0 warnings, 0 errors (4.7s)

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
| 20.01 DB Fix | 4 | 4 |
| 20.1 Foundation | 55 | 59 |
| 20.2 WebSocket | 9 | 68 |
| 20.3 Resilience | 8 | 76 |
| 20.4 State | 4 | 80 |
| 20.5 WASM Host | 8 | 88 |
| 20.6 Tests | 15 | **103** |

---

## Production-Grade Checklist

| Requirement | Status |
|-------------|--------|
| Entity interface hierarchy (IEntity, IAuditable, etc.) | ✅ Planned |
| Exception hierarchy with correlation IDs | ✅ Planned |
| Validation framework with error messages | ✅ Planned |
| Circuit breaker matching pm-config thresholds | ✅ Complete |
| Retry with exponential backoff + jitter | ✅ Complete |
| Reconnection with subscription rehydration | ✅ Complete |
| Per-ping latency tracking | ✅ Planned |
| Optimistic updates with rollback | ✅ Complete |
| Thread-safe state with ConcurrentDictionary | ✅ Complete |
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
