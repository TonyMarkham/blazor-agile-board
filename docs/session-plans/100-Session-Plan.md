# Session 100: LLM-Callable CLI for Work Item CRUD

## Production-Grade Goal

This session adds an LLM-accessible interface to the project management system:

- REST API layer in pm-server that triggers WebSocket broadcasts
- CLI binary (pm-cli) for command-line access to work items
- Dedicated LLM user account for AI assistants
- Real-time updates: CLI changes appear instantly in Blazor UI

---

## Architecture

```
+----------+     HTTP/JSON     +---------------------------+
|  pm-cli  | ----------------> |        pm-server          |
+----------+                   |  +-------+  +----------+  |
                               |  | REST  |  | WebSocket|  |
                               |  | API   |  | Handler  |  |
                               |  +---+---+  +----+-----+  |
                               |      |           |        |
                               |      +-----+-----+        |
                               |            v              |
                               |    +--------------+       |
                               |    |  Broadcast   |-----> Blazor clients
                               |    +--------------+       |
                               +---------------------------+
```

**Key Concepts:**
- REST API handlers reuse existing pm-ws validation and broadcast infrastructure
- CLI outputs JSON for easy parsing by LLMs and scripts
- WebSocket broadcasts ensure all connected clients see changes immediately

---

## Security Context

**This is currently a desktop-only application.** The REST API is designed for local CLI and LLM access on the same machine as the Tauri desktop app.

**Intentional design decisions:**
- **Permissive CORS**: The API allows requests from any origin by default. This is intentional for desktop use where the CLI runs locally. For future web deployment, set `PM_API_RESTRICT_CORS=true` to enable restrictive CORS.
- **No authentication required**: Requests without an `X-User-Id` header fall back to the configured LLM user. This simplifies CLI usage in a single-user desktop context.
- **Localhost binding**: The server binds to `127.0.0.1` by default, preventing external network access.

When/if this becomes a multi-user web application, additional security measures (JWT auth on REST endpoints, stricter CORS, rate limiting) should be added.

---

## Sub-Session Breakdown

This plan has been split into sub-sessions to fit within token budgets:

| Session | Scope | Est. Tokens | Status |
|---------|-------|-------------|--------|
| **[100.1](100.1-Session-Plan.md)** | API Foundation (Config, Errors, Extractors, AppState) | ~40k | Pending |
| **[100.2](100.2-Session-Plan.md)** | REST API Handlers (Work Items, Projects, Comments, Routes) | ~50k | Pending |
| **[100.3](100.3-Session-Plan.md)** | CLI Implementation (pm-cli crate, HTTP client, commands) | ~40k | Pending |

---

## Session 100.1: API Foundation

**Scope:** Set up the infrastructure needed by all API handlers

**Files Created:**
- `pm-config/src/api_config.rs` - LLM user configuration
- `pm-server/src/api/error.rs` - REST API error types
- `pm-server/src/api/extractors.rs` - User ID extraction
- `pm-server/src/api/mod.rs` - API module

**Files Modified:**
- `Cargo.toml` (root) - Add clap, reqwest, pm-cli to workspace
- `pm-ws/Cargo.toml` - Add pm-config dependency
- `pm-config/src/lib.rs` - Export ApiConfig
- `pm-ws/src/app_state.rs` - Add api_config field

**Verification:** `cargo check -p pm-server`

---

## Session 100.2: REST API Handlers

**Scope:** Implement REST endpoints that broadcast via WebSocket

**Files Created:**
- `pm-server/src/api/work_items.rs` - Full CRUD for work items
- `pm-server/src/api/projects.rs` - Project list/get
- `pm-server/src/api/comments.rs` - Comment CRUD

**Files Modified:**
- `pm-server/src/api/mod.rs` - Export new modules
- `pm-server/src/routes.rs` - Add REST API routes
- `pm-server/src/main.rs` - Add api module, ensure LLM user

**Verification:** `cargo check -p pm-server && cargo test -p pm-server`

---

## Session 100.3: CLI Implementation

**Scope:** Build the command-line interface for LLM integration

**Files Created:**
- `pm-cli/Cargo.toml` - CLI crate manifest
- `pm-cli/src/main.rs` - CLI entry point with clap
- `pm-cli/src/client.rs` - HTTP client wrapper

**Files Modified:**
- `justfile` - Add CLI build/run commands

**Verification:** `cargo build -p pm-cli && ./target/debug/pm --help`

---

## Pre-Implementation Checklist

Before starting **any** sub-session:

- [ ] `cargo test --workspace` passes
- [ ] `just dev` starts the application successfully
- [ ] Database has at least one project (for testing)

---

## Files Summary

### Create (10 files)

| File | Purpose |
|------|---------|
| `pm-config/src/api_config.rs` | LLM user ID and name configuration |
| `pm-server/src/api/mod.rs` | API module exports |
| `pm-server/src/api/error.rs` | REST API error types with HTTP status |
| `pm-server/src/api/extractors.rs` | User ID extraction from headers |
| `pm-server/src/api/work_items.rs` | Work item CRUD handlers |
| `pm-server/src/api/projects.rs` | Project read handlers |
| `pm-server/src/api/comments.rs` | Comment CRUD handlers |
| `pm-cli/Cargo.toml` | CLI crate manifest |
| `pm-cli/src/main.rs` | CLI entry point |
| `pm-cli/src/client.rs` | HTTP client implementation |

### Modify (6 files)

| File | Change |
|------|--------|
| `Cargo.toml` (root) | Add clap, reqwest, pm-cli member |
| `pm-ws/Cargo.toml` | Add pm-config dependency |
| `pm-config/src/lib.rs` | Export ApiConfig |
| `pm-ws/src/app_state.rs` | Add api_config field |
| `pm-server/src/routes.rs` | Add REST API routes |
| `pm-server/src/main.rs` | Add api module, ensure LLM user |
| `justfile` | Add CLI commands |

---

## Usage Examples

After completing all sessions, the CLI enables:

```bash
# List all projects
pm project list --pretty

# Create a work item (shows in Blazor UI immediately!)
pm work-item create \
  --project-id <uuid> \
  --type story \
  --title "Implement user authentication" \
  --description "Add OAuth2 login flow" \
  --pretty

# List work items with filtering
pm work-item list <project-id> --type story --status in_progress

# Update a work item
pm work-item update <work-item-id> \
  --status done \
  --version 1 \
  --pretty

# Add a comment
pm comment create \
  --work-item-id <uuid> \
  --content "This is ready for review"
```

---

## Final Verification

After all three sub-sessions are complete:

```bash
# Build everything
just check-backend
just build-rs-cli

# Start the server
cargo run -p pm-server

# In another terminal, start Blazor
just dev

# In a third terminal, test CLI
pm project list --pretty
pm work-item create --project-id <uuid> --type task --title "Test from CLI"

# Verify the new task appears in the Blazor UI!
```

---

## What You'll Learn

This session teaches several production patterns:

1. **REST + WebSocket Integration** - REST endpoints that trigger real-time broadcasts
2. **Axum Extractors** - Custom request extractors for user authentication
3. **Error Response Design** - Consistent JSON error format with HTTP status codes
4. **CLI Design with Clap** - Type-safe argument parsing with derive macros
5. **HTTP Client Patterns** - Clean HTTP client wrapper for API calls
6. **Optimistic Locking** - Version-based conflict detection for updates
