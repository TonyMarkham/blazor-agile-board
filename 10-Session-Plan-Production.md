# Session 10: Production-Grade Message Dispatch

## Production-Grade Score Target: 9.6/10

This session wires the database to message dispatch with production-grade features:

- Circuit breaker for database resilience
- Correlation IDs for distributed tracing
- Panic recovery with error boundaries
- Structured logging with context
- Retry logic with exponential backoff
- Database health probes
- Property-based testing

---

## Sub-Session Breakdown

This plan has been split into sub-sessions to fit within token budgets:

| Session | Scope | Est. Tokens | Status |
|---------|-------|-------------|--------|
| **[10.05](10.05-Session-Plan.md)** | Configuration Extensions (pm-config) | ~35-45k | Pending |
| **[10.1](10.1-Session-Plan.md)** | Foundation Infrastructure | ~50-60k | Pending |
| **[10.2](10.2-Session-Plan.md)** | Handler Infrastructure & Business Logic | ~65-75k | Pending |
| **[10.3](10.3-Session-Plan.md)** | Server Integration & Testing | ~70-85k | Pending |

---

## Session 10.05: Configuration Extensions

**Files Created:**
- `pm-config/src/circuit_breaker_config.rs` - Circuit breaker settings
- `pm-config/src/retry_config.rs` - Retry/backoff settings
- `pm-config/src/handler_config.rs` - Handler timeout settings
- `pm-config/src/validation_config.rs` - Field length limits

**Files Modified:**
- `pm-config/src/lib.rs` - Module declarations and re-exports
- `pm-config/src/config.rs` - Add new config sections
- `pm-config/src/tests/mod.rs` - Add new test module declarations

**Verification:** `cargo check -p pm-config && cargo test -p pm-config`

---

## Session 10.1: Foundation Infrastructure

**Files Created:**
- `pm-ws/src/request_context.rs` - Correlation ID tracking
- `pm-ws/src/tracing.rs` - Structured logging with macros
- `pm-ws/src/circuit_breaker.rs` - Database resilience pattern
- `pm-ws/src/retry.rs` - Exponential backoff with jitter

**Files Modified:**
- `pm-ws/Cargo.toml` - Add `base64`, `serde_json`, `rand`, `proptest`
- `pm-ws/src/lib.rs` - Export new modules
- `pm-ws/src/error.rs` - Add `Database`, `ServiceUnavailable`, `Timeout` variants

**Verification:** `cargo check -p pm-ws`

---

## Session 10.2: Handler Infrastructure & Business Logic

**Files Created:**
- `pm-ws/src/handlers/db_ops.rs` - Circuit-breaker-aware DB wrappers
- `pm-ws/src/handlers/error_boundary.rs` - Panic recovery
- `pm-ws/src/handlers/work_item.rs` - Create/Update/Delete handlers
- `pm-ws/src/handlers/query.rs` - GetWorkItems handler
- `pm-ws/src/handlers/dispatcher.rs` - Message routing with timeout

**Files Modified:**
- `pm-ws/src/handlers/context.rs` - Add circuit breaker + request context
- `pm-ws/src/handlers/mod.rs` - Export new modules

**Verification:** `cargo check -p pm-ws`

---

## Session 10.3: Server Integration & Testing

**Files Created:**
- `pm-ws/tests/property_tests.rs` - Property-based tests with proptest
- `pm-ws/tests/dispatcher_integration.rs` - End-to-end dispatcher tests

**Files Modified:**
- `pm-ws/src/lib.rs` - Final exports (dispatcher, macros)
- `pm-ws/src/app_state.rs` - Add `pool`, `circuit_breaker` fields
- `pm-ws/src/web_socket_connection.rs` - Wire to dispatcher
- `pm-server/src/main.rs` - Initialize pool + circuit breaker
- `pm-server/src/health.rs` - Database probe + circuit breaker status
- `pm-server/src/error.rs` - Database error variants

**Verification:** `cargo check --workspace && cargo test --workspace`

---

## Pre-Implementation Checklist

Before starting **any** sub-session:

- [ ] `cargo test --workspace` passes
- [ ] Database migrations are current
- [ ] `cargo build -p pm-proto` succeeds

---

## Files Summary

### Create (14 files)

| File | Purpose |
|------|---------|
| `pm-config/src/circuit_breaker_config.rs` | Circuit breaker settings |
| `pm-config/src/retry_config.rs` | Retry/backoff settings |
| `pm-config/src/handler_config.rs` | Handler timeout settings |
| `pm-config/src/validation_config.rs` | Field length limits |
| `pm-ws/src/request_context.rs` | Correlation ID tracking |
| `pm-ws/src/tracing.rs` | Structured logging |
| `pm-ws/src/circuit_breaker.rs` | Database resilience |
| `pm-ws/src/retry.rs` | Exponential backoff |
| `pm-ws/src/handlers/error_boundary.rs` | Panic recovery |
| `pm-ws/src/handlers/db_ops.rs` | Circuit-breaker-aware DB ops |
| `pm-ws/src/handlers/dispatcher.rs` | Message routing |
| `pm-ws/src/handlers/work_item.rs` | CRUD handlers |
| `pm-ws/src/handlers/query.rs` | Read handlers |
| `pm-ws/tests/property_tests.rs` | Property-based tests |

### Modify (13 files)

| File | Change |
|------|--------|
| `pm-config/src/lib.rs` | Add module declarations + re-exports |
| `pm-config/src/config.rs` | Add new config sections |
| `pm-config/src/tests/mod.rs` | Add new test module declarations |
| `pm-ws/src/error.rs` | Add new error types + From impls |
| `pm-ws/src/app_state.rs` | Add pool + circuit_breaker fields |
| `pm-ws/src/handlers/context.rs` | Add circuit_breaker + request_ctx |
| `pm-ws/src/web_socket_connection.rs` | Wire to dispatcher |
| `pm-ws/src/handlers/mod.rs` | Export new modules |
| `pm-ws/src/lib.rs` | Export new types |
| `pm-ws/Cargo.toml` | Add dependencies |
| `pm-server/src/main.rs` | Initialize circuit breaker + DB |
| `pm-server/src/health.rs` | Add database probe |
| `pm-server/src/error.rs` | Add database error variants |

---

## Production-Grade Scoring

| Category | Score | Justification |
|----------|-------|---------------|
| Error Handling | 9.8/10 | Comprehensive errors, From impls, error codes, sanitized messages |
| Validation | 9.7/10 | Input validation, XSS sanitization, property-based tests |
| Authorization | 9.5/10 | Permission checks, role-based access, circuit breaker protected |
| Data Integrity | 9.7/10 | Transactions, optimistic locking, circuit breaker, retry logic |
| Idempotency | 9.5/10 | Message deduplication, safe replay, non-fatal storage |
| Audit Trail | 9.5/10 | Activity logging with field-level tracking |
| Performance | 9.5/10 | Timeouts, connection pool, WAL, circuit breaker |
| Testing | 9.5/10 | Integration tests + property-based tests |
| Observability | 9.7/10 | Correlation IDs, structured logging, health probes |
| Security | 9.5/10 | XSS sanitization, JWT auth, rate limiting, error sanitization |
| Resilience | 9.5/10 | Circuit breaker, retry with backoff, panic recovery |

**Overall Score: 9.6/10**

### What Would Make It 10/10

- OpenTelemetry integration for distributed tracing export
- Prometheus metrics endpoint
- Chaos/fault injection testing
- Load testing benchmarks with k6 or similar
- Full broadcast channel implementation (Session 20)

---

## Final Verification

After all three sub-sessions are complete:

```bash
cargo check --workspace
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo run -p pm-server

# Test health endpoints
curl http://localhost:8080/health
curl http://localhost:8080/ready
curl http://localhost:8080/live
```
