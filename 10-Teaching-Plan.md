# Teaching Plan: Session 10 Code-Along Walkthrough

## Overview

We'll work through the Session 10 implementation together step-by-step. I'll explain each piece as we build it, keeping chunks digestible and verifying as we go.

---

## Teaching Structure

For each sub-session, I'll follow this pattern:

1. **Context Check** - Verify prerequisites pass (`cargo check`)
2. **Concept Brief** - 2-3 sentences explaining what we're building and why
3. **Code Walkthrough** - Show code in chunks, explain each significant part
4. **Verify** - Run `cargo check` or `cargo test` after each major step
5. **Checkpoint** - Summarize what we built before moving on

---

## Session Breakdown

### Session 10.1: Foundation Infrastructure (4 teaching units)

| Unit | Topic | What We Build | Key Learning |
|------|-------|---------------|--------------|
| 1 | Dependencies | Update Cargo.toml | Setting up a Rust project |
| 2 | Request Context | `request_context.rs`, `tracing.rs` | Correlation IDs, structured logging |
| 3 | Circuit Breaker | `circuit_breaker.rs` | Resilience pattern, state machines |
| 4 | Error & Retry | `error.rs` updates, `retry.rs` | Error handling, exponential backoff |

### Session 10.2: Handler Infrastructure (5 teaching units)

| Unit | Topic | What We Build | Key Learning |
|------|-------|---------------|--------------|
| 5 | Handler Context | Update `context.rs` | Composing infrastructure into handlers |
| 6 | DB Operations | `db_ops.rs` | Wrapping DB calls with resilience |
| 7 | Error Boundary | `error_boundary.rs` | Panic recovery, defensive programming |
| 8 | Work Item Handlers | `work_item.rs` | CRUD operations, validation, audit logging |
| 9 | Query & Dispatch | `query.rs`, `dispatcher.rs` | Message routing, timeouts |

### Session 10.3: Server Integration (4 teaching units)

| Unit | Topic | What We Build | Key Learning |
|------|-------|---------------|--------------|
| 10 | Server Updates | `main.rs`, `error.rs`, `health.rs` | Wiring infrastructure together |
| 11 | AppState & Connection | `app_state.rs`, `web_socket_connection.rs` | Connection lifecycle |
| 12 | Property Tests | `property_tests.rs` | Property-based testing with proptest |
| 13 | Integration Tests | `dispatcher_integration.rs` | End-to-end testing |

---

## Workflow

For each teaching unit:

```
1. I explain what we're building (brief)
2. I show you the code to write
3. I explain key sections inline
4. You write/paste the code
5. We run cargo check/test
6. I summarize the key patterns
```

---

## Prerequisites

Before starting, we need to verify:
- [ ] Existing codebase compiles (`cargo check --workspace`)
- [ ] Database migrations exist in `pm-db/migrations`
- [ ] Proto definitions built (`cargo build -p pm-proto`)

---

## Starting Point

We'll begin with **Unit 1: Dependencies** in Session 10.1, updating `pm-ws/Cargo.toml` with the new dependencies we need.

---

## Verification Commands

After each sub-session:
- 10.1: `cargo check -p pm-ws`
- 10.2: `cargo check -p pm-ws && cargo check --workspace`
- 10.3: `cargo test --workspace && cargo run -p pm-server`

Final verification:
```bash
curl http://localhost:8080/health
curl http://localhost:8080/ready
```
