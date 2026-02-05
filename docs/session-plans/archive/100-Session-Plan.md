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
| **[100.1](100.1-Session-Plan.md)** | API Foundation (Config, Errors, Extractors, AppState) | ~40k | ✅ Complete |
| **[100.2](100.2-Session-Plan.md)** | REST API Handlers (Work Items, Projects, Comments, Routes) | ~50k | ✅ Complete |
| **[100.3](100.3-Session-Plan.md)** | CLI Implementation (pm-cli crate, HTTP client, commands) | ~40k | ✅ Complete |

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

## Session 100.2: REST API Handlers ✅

**Scope:** Implement REST endpoints that broadcast via WebSocket

**Files Created:**
- `pm-server/src/lib.rs` - Library crate for test access
- `pm-server/src/api/work_items/` - Full CRUD for work items (9 files: handler + DTOs)
- `pm-server/src/api/projects/` - Project list/get (5 files: handler + DTOs)
- `pm-server/src/api/comments/` - Comment CRUD (7 files: handler + DTOs)
- `pm-server/src/api/delete_response.rs` - Shared delete response type
- `pm-server/tests/common/mod.rs` - Test infrastructure
- `pm-server/tests/api_work_items_tests.rs` - 2 integration tests
- `pm-server/tests/api_projects_tests.rs` - 5 integration tests
- `pm-server/tests/api_comments_tests.rs` - 7 integration tests

**Files Modified:**
- `pm-server/src/api/mod.rs` - Export new modules
- `pm-server/src/routes.rs` - Add 10 REST API routes with CORS
- `pm-server/src/main.rs` - Add api module, ensure_llm_user()

**Tests:** 14 integration tests (all passing)

**Verification:** ✅ `cargo check -p pm-server && cargo test -p pm-server`

---

## Session 100.3: CLI Implementation ✅

**Scope:** Build the command-line interface for LLM integration

**Files Created:**
- `pm-cli/Cargo.toml` - CLI crate manifest
- `pm-cli/src/lib.rs` - Library entry point
- `pm-cli/src/main.rs` - CLI entry point
- `pm-cli/src/cli.rs` - CLI struct
- `pm-cli/src/commands.rs` - Commands enum
- `pm-cli/src/project_commands.rs` - Project commands
- `pm-cli/src/work_item_commands.rs` - Work item commands
- `pm-cli/src/comment_commands.rs` - Comment commands
- `pm-cli/src/client/` - HTTP client module (3 files)
- `pm-cli/tests/client_integration_tests.rs` - Integration tests

**Files Modified:**
- `Cargo.toml` (root) - Add wiremock, register pm-cli
- `justfile` - Add 8 CLI commands

**Tests:** 12 tests passing (4 unit + 8 integration)

**Verification:** ✅ `cargo build -p pm-cli && ./target/debug/pm --help`

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

## Final Verification ✅

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

## Session 100 Summary ✅

**Completion Date:** 2026-02-05

**What Was Built:**

1. **API Foundation (100.1)**
   - ✅ LLM user configuration (ApiConfig)
   - ✅ REST API error types with HTTP status codes
   - ✅ Axum extractors for user authentication (UserId)
   - ✅ AppState extended with api_config

2. **REST API Handlers (100.2)**
   - ✅ Full CRUD for work items (9 endpoints)
   - ✅ Project list/get endpoints (2 endpoints)
   - ✅ Comment CRUD endpoints (7 endpoints)
   - ✅ RESTful route registration with CORS
   - ✅ LLM user initialization
   - ✅ 14 integration tests (all passing)

3. **CLI Implementation (100.3)**
   - ✅ Type-safe CLI with clap derive macros
   - ✅ HTTP client wrapper (9 API methods)
   - ✅ JSON output for LLM parsing (--pretty flag)
   - ✅ Integration with pm-config for server defaults
   - ✅ 8 justfile commands
   - ✅ 12 tests (4 unit + 8 integration, all passing)

**Files Created:** 35+ files
**Files Modified:** 8 files
**Tests Added:** 26 tests (all passing)
**New Commands:** 8 justfile commands

**Key Improvements:**
- ErrorLocation on all error variants for better debugging
- Config integration for single source of truth
- Modular code organization
- Comprehensive test coverage

---

## What You'll Learn

This session teaches several production patterns:

1. **REST + WebSocket Integration** - REST endpoints that trigger real-time broadcasts
2. **Axum Extractors** - Custom request extractors for user authentication
3. **Error Response Design** - Consistent JSON error format with HTTP status codes
4. **CLI Design with Clap** - Type-safe argument parsing with derive macros
5. **HTTP Client Patterns** - Clean HTTP client wrapper for API calls
6. **Optimistic Locking** - Version-based conflict detection for updates
