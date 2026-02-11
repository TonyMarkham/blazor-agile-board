# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Blazor Agile Board is a production-grade agile project management system built with **Blazor WebAssembly** (frontend) and **Rust + Axum** (backend). It's designed as a plugin for multi-tenant SaaS platforms with per-tenant SQLite databases and real-time collaboration via WebSocket + Protocol Buffers.

**Current Status**: Implementation in progress. See `docs/session-plans/` for active scope.

## Architecture

### Multi-Tenant Design
- **Per-tenant SQLite databases**: Each tenant has a dedicated `main.db` file at `/data/tenants/{tenant_id}/main.db`
- **Table injection**: Plugin tables are injected into tenant databases with `pm_*` prefix (e.g., `pm_work_items`, `pm_sprints`)
- **Connection management**: Rust backend maintains cached connection pools per active tenant
- **Tenant resolution**: JWT claims contain `tenant_id` and `user_id` for authentication and data isolation

### Communication Protocol
- **WebSocket-first architecture**: All CRUD operations happen over WebSocket, not REST
- **Protocol Buffers**: Binary serialization for efficient real-time updates
- **Bidirectional**: Clients send commands, server broadcasts events to subscribed clients
- **Optional REST API**: Read-only endpoints for LLM integration and bulk data loading (Session 60)

### Backend Structure (Rust)
```
backend/
├── crates/
│   ├── pm-core/        # Domain models and business logic
│   ├── pm-db/          # SQLx repositories and migrations
│   ├── pm-api/         # REST API (optional, LLM queries)
│   ├── pm-ws/          # WebSocket server with broadcast channels
│   ├── pm-auth/        # JWT validation and tenant extraction
│   └── pm-proto/       # Protobuf generated code
└── pm-server/          # Main binary
```

**CRITICAL: Cargo Workspace Architecture**

This project uses a **Cargo workspace** with centralized dependency management:

1. **All dependencies are defined ONCE in the root `Cargo.toml`**:
   - External crates go in `[workspace.dependencies]` with version numbers
   - Internal workspace crates also listed in `[workspace.dependencies]` with paths
   - Example from root `Cargo.toml`:
     ```toml
     [workspace.dependencies]
     axum = { version = "0.8.8", features = ["ws"] }
     sqlx = { version = "0.8.6", features = ["runtime-tokio-rustls", "sqlite", "uuid", "chrono"] }
     pm-core = { path = "backend/crates/pm-core" }
     pm-db = { path = "backend/crates/pm-db" }
     ```

2. **Member crates reference dependencies WITHOUT version numbers**:
   - In `backend/crates/pm-db/Cargo.toml`:
     ```toml
     [dependencies]
     sqlx = { workspace = true }
     uuid = { workspace = true }
     pm-core = { workspace = true }
     ```
   - Note: Uses `{ workspace = true }` syntax, NOT version numbers

3. **NEVER add dependencies directly to member crate `Cargo.toml` files with version numbers**:
   - ❌ WRONG: `sqlx = "0.8.6"` in `backend/crates/pm-db/Cargo.toml`
   - ❌ WRONG: `pm-core = { path = "../pm-core" }` in a member crate
   - ✅ CORRECT: Add to root `Cargo.toml` `[workspace.dependencies]`, then reference with `{ workspace = true }`

4. **When adding a new dependency:**
   - Step 1: Add it to root `Cargo.toml` under `[workspace.dependencies]` with version/features
   - Step 2: Reference it in the member crate's `Cargo.toml` with `{ workspace = true }`
   - Step 3: Never specify versions or paths in member crates

5. **Benefits of this pattern**:
   - All crates use identical dependency versions (no version conflicts)
   - Single source of truth for all dependencies
   - Easier to audit and upgrade dependencies across the entire workspace
   - Enforces consistent feature flags across the workspace

**Key Backend Concepts:**
- `TenantConnectionManager`: Manages per-tenant SQLite connection pools with lazy loading
- Repository pattern: Each entity (WorkItem, Sprint, Comment, etc.) has a dedicated repository in `pm-db/src/repositories/`
- WebSocket broadcast channels: Per-tenant channels for real-time event distribution
- Subscription model: Clients explicitly subscribe to projects/sprints they want updates for

### Frontend Structure (Blazor/.NET)
```
frontend/
├── ProjectManagement.Core/         # Models, DTOs, interfaces (no UI)
├── ProjectManagement.Services/     # Business logic, API clients, WebSocket
├── ProjectManagement.Components/   # Razor Class Library (RCL) - reusable UI
└── ProjectManagement.Wasm/         # Standalone WASM host
```

**Key Frontend Concepts:**
- Razor Class Library (RCL): Makes the entire UI portable as a plugin
- Radzen UI components for professional UI out of the box
- State management for local-first optimistic updates
- WebSocket client maintains subscription state

## Database Schema

All tables use `pm_*` prefix and follow these conventions:
- **UUID primary keys** (TEXT type) for distributed/offline creation
- **Audit columns**: `created_at`, `updated_at`, `created_by`, `updated_by` on all tables
- **Soft deletes**: `deleted_at` column for audit trail preservation
- **Foreign keys enabled**: `PRAGMA foreign_keys = ON`

### Core Tables
- `pm_work_items`: Polymorphic table for Projects/Epics/Stories/Tasks (hierarchy in single table)
  - `item_type`: 'project', 'epic', 'story', 'task'
  - `parent_id`: Self-referential hierarchy
  - `project_id`: Denormalized for query performance
  - `position`: Integer for drag-and-drop ordering
- `pm_sprints`: Sprint planning with start/end dates and velocity tracking
- `pm_comments`: Comment threads on work items
- `pm_time_entries`: Time tracking with running timers
- `pm_dependencies`: 2-way dependency tracking with cycle detection
- `pm_activity_log`: Complete audit trail for all changes
- `pm_swim_lanes`: Customizable Kanban board lanes
- `pm_llm_context`: Self-documenting schema for LLM agents

See `docs/database-schema.md` for complete schema with indexes and foreign key relationships.

## Development Commands

**IMPORTANT: Use `just` for all build tasks.** All commands are defined in the `justfile` at the repository root.

Run `just help` to see all available commands.

### Quick Start

```bash
# First time setup
just restore          # Restore all dependencies (frontend + backend)
just check            # Restore + build + test everything

# Development workflow
just dev              # Build and run Tauri desktop app
just test             # Run all tests (backend + frontend)
just clean            # Clean all build artifacts
```

### Frontend (Blazor/.NET)

**Solution-level commands (work on all projects):**

- **Restore packages**: `just restore-frontend` (always run first after pulling changes)
- **Build all**: `just build-frontend` (Debug) or `just build-frontend-release` (Release)
- **Test all**: `just test-frontend`
- **Test verbose**: `just test-frontend-verbose` (detailed test output)
- **Test coverage**: `just test-frontend-coverage` (outputs to `./coverage/`)
- **Clean all**: `just clean-frontend`
- **Full check**: `just check-frontend` (restore → build → test)

**Individual project commands:**

- **Build one project**: `just build-cs-core`, `just build-cs-services`, `just build-cs-components`, `just build-cs-wasm`
  - Optional config parameter: `just build-cs-core Release`
- **Publish WASM**: `just publish-wasm` (Debug) or `just publish-wasm Release`
- **Watch mode**: `just watch-cs-core`, `just watch-cs-services`, `just watch-cs-components`, `just watch-cs-wasm`
  - Auto-rebuilds project on file changes

**Test commands:**

- **Run one test project**: `just test-cs-core`, `just test-cs-services`, `just test-cs-components`
- **Filter tests**: `just test-cs-filter "Converters"` (matches namespace/class/method)
- **List tests**: `just list-tests-cs` (shows all test names without running)
- **Watch tests**: `just watch-test-cs-core`, `just watch-test-cs-services`, `just watch-test-cs-components`
  - Auto-runs tests on file changes (TDD workflow)

**Test Examples:**
```bash
# Run only validator tests
just test-cs-filter "Validation"

# Run only property-based tests
just test-cs-filter "PropertyTests"

# Run only a specific test class
just test-cs-filter "ProtoConverterTests"

# TDD workflow - watch and auto-test on save
just watch-test-cs-core
```

**Test Project Structure:**
- `ProjectManagement.Core.Tests`: Converters, validators, property-based tests (FsCheck)
- `ProjectManagement.Services.Tests`: Service layer, mocking (Moq), resilience tests
- `ProjectManagement.Components.Tests`: Blazor component tests (bUnit), ViewModels

All tests use xUnit with FluentAssertions.

### Backend (Rust)

**Workspace-level commands (work on all packages):**

- **Restore dependencies**: `just restore-backend` (fetch all cargo dependencies)
- **Check**: `just check-backend` (fast compile check without codegen)
- **Clippy**: `just clippy-backend` (lint with clippy, fails on warnings)
- **Build all**: `just build-backend` (Debug) or `just build-backend-release` (Release)
- **Test all**: `just test-backend` or `just test-backend-verbose` (with output)
- **Clean all**: `just clean-backend`
- **Full check**: `just check-backend-full` (check → clippy → test)

**Individual package commands:**

Packages: `server`, `core`, `db`, `auth`, `proto`, `ws`, `config`

- **Check one package**: `just check-rs-<package>`
  - Example: `just check-rs-db`
- **Clippy one package**: `just clippy-rs-<package>`
  - Example: `just clippy-rs-auth`
- **Build one package**: `just build-rs-<package>` or `just build-rs-<package>-release`
  - Example: `just build-rs-server`, `just build-rs-core-release`
- **Test one package**: `just test-rs-<package>`
  - Example: `just test-rs-ws`
- **Watch one package**: `just watch-rs-<package>` (auto-check on changes)
  - Example: `just watch-rs-db`
- **Watch test**: `just watch-test-rs-<package>` (auto-test on changes)
  - Example: `just watch-test-rs-core`

**Common workflows:**

```bash
# Quick syntax check before committing
just check-backend

# Run clippy on the whole workspace
just clippy-backend

# TDD workflow - watch and auto-test on save
just watch-test-rs-db

# Work on one package with fast feedback
just watch-rs-auth

# Build and run the server
just build-rs-server
cargo run --bin pm-server  # after just setup-config
```

**Database migrations:**
- Use SQLx CLI: `sqlx migrate run --database-url <tenant_db_url>`

**Rust Package Structure:**
- `pm-server`: Main binary, Axum HTTP server
- `pm-core`: Domain models and business logic
- `pm-db`: SQLx repositories and database layer
- `pm-auth`: JWT validation and tenant extraction
- `pm-proto`: Protobuf message definitions
- `pm-ws`: WebSocket server implementation
- `pm-config`: Configuration loading and validation

### Combined Workflows

- **Full check**: `just check` (restore + check + clippy + test everything)
- **Quick check**: `just check-all` (fast compile check for all code)
- **Lint**: `just lint` (run clippy on Rust code)
- **Development build**: `just build-dev` (parallel build of backend + frontend)
- **Production build**: `just build-release`
- **Run Tauri desktop**: `just dev`
- **Run all tests**: `just test` (backend + frontend)
- **Clean everything**: `just clean`
- **Restore all**: `just restore` (frontend NuGet + backend cargo)

### IMPORTANT Build Notes

**C# Frontend:**
- The solution file is `frontend/ProjectManagement.slnx` (XML format, not `.sln`)
- **Always run `just restore-frontend` before building after pulling changes**
- Use `dotnet build` for compilation checks, `dotnet publish` for deployable output
- The WASM project publishes to `desktop/frontend/` by default (configured in `.csproj`)
- Protobuf files (`proto/messages.proto`) are compiled into `ProjectManagement.Core` automatically
- All projects follow standard .NET conventions: `bin/`, `obj/`, Debug/Release configs
- Test commands are prefixed with `test-cs-` to distinguish from Rust tests

**Rust Backend:**
- Workspace uses Cargo workspace with 7 packages (see Rust Package Structure above)
- **Always run `just restore-backend` or `just restore` after pulling changes**
- Use `cargo check` for fast syntax validation, `cargo build` for full compilation
- Clippy is configured to fail on warnings (`-D warnings`)
- All package names use `pm-*` prefix except the main `pm-server` binary
- Test commands are prefixed with `test-rs-` to distinguish from C# tests
- Individual package commands use pattern: `<action>-rs-<package>`

**All commands:**
- All project paths and configuration are defined as variables at the top of `justfile`
- Use `just help` to see all available commands
- Commands are organized by technology (C#/Rust) and scope (workspace/individual)

### Common Issues

**C# Frontend:**
- **"Package not found"**: Run `just restore-frontend`
- **"Project not found"**: Ensure project name matches directory name exactly
- **"proto file not found"**: Ensure `proto/messages.proto` exists
- **Tests fail to run**: Ensure `just restore-frontend` was run first
- **Want to work on one project**: Use `just watch-cs-<project>` (e.g., `just watch-cs-services`)

**Rust Backend:**
- **COMMON MISTAKE: Adding dependencies with version numbers to member crates**: Always add dependencies to root `Cargo.toml` `[workspace.dependencies]` first, then reference with `{ workspace = true }` in member crates. NEVER put version numbers in member crate `Cargo.toml` files.
- **"Could not find crate"**: Run `just restore-backend` or `just restore`
- **Clippy warnings**: Fix all warnings before committing (configured with `-D warnings`)
- **"Cannot find package"**: Check package name uses `pm-*` prefix (e.g., `pm-core` not `core`)
- **Want to work on one package**: Use `just watch-rs-<package>` (e.g., `just watch-rs-db`)
- **Slow builds**: Use `just check-rs-<package>` for fast syntax validation
- **Dependency version conflicts**: Check that all workspace members use `{ workspace = true }` and versions are only in root `Cargo.toml`

**General:**
- **Build fails after git pull**: Run `just restore` then `just build-dev`
- **Don't know what command to use**: Run `just help` to see all available commands
- **Want fast feedback**: Use watch commands for auto-rebuild/test on file changes

## Implementation Plan

The project is being built in focused sessions with token budgets:

| Session | Focus | Files |
|---------|-------|-------|
| **10** | Foundation: Database, migrations, repositories, protobuf | ~30 files |
| **20** | WebSocket: Connection handling, broadcast channels, subscriptions | ~15 files |
| **30** | Work Items: Backend handlers + Frontend UI (Kanban board) | ~25 files |
| **40** | Sprints & Comments: Sprint planning UI, comment threads | ~20 files |
| **50** | Time Tracking & Dependencies: Running timers, dependency graph | ~20 files |
| **60** | REST API for LLMs: Read-only endpoints for AI assistants | ~10 files |
| **70** | Activity Logging & Polish: Complete audit trail, error handling | ~15 files |

Sessions numbered 10, 20, 30... to leave room for incremental work.

See `docs/implementation-plan-revised.md` for detailed breakdown of each session.

## Key Architecture Decisions (ADRs)

Important decisions documented in `docs/adr/`:

1. **ADR-0001**: Plugin architecture with table injection into tenant databases (not separate DB per plugin)
2. **ADR-0002**: Per-tenant SQLite databases for complete data isolation and LLM accessibility
3. **ADR-0003**: Blazor WebAssembly with Radzen components for professional UI
4. **ADR-0004**: Rust + Axum backend for performance and type safety
5. **ADR-0005**: WebSocket + Protobuf as primary protocol (not REST-first)

## LLM Integration Design

This project is explicitly designed to be LLM-friendly:
- **Self-documenting database**: `pm_llm_context` table contains schema docs, query patterns, business rules
- **Single-file tenant context**: All tenant data (platform + plugin) in one SQLite file
- **Semantic naming**: Descriptive column names and table structures
- **Complete audit trail**: `pm_activity_log` provides full change history
- **Read-only REST API**: Optional endpoints for LLM queries without authentication complexity (Session 60)

See `docs/llm-integration-guide.md` for LLM query patterns and examples.

## Code Conventions

**Logging:**
- **Always use `tracing` for all logging** - Never import from `log` crate in new code
- **Structured logging**: Use field syntax for context: `tracing::info!(user_id = %id, "User logged in")`
- **Instrument functions**: Use `#[tracing::instrument]` for automatic span tracking
- **Span hierarchy**: Wrap related operations in `info_span!()` / `debug_span!()` for context
- **Log levels**:
  - `error!`: Unrecoverable errors, system failures
  - `warn!`: Recoverable errors, suspicious behavior
  - `info!`: High-level user actions, lifecycle events
  - `debug!`: Detailed request/response, state changes
  - `trace!`: Very verbose, performance-sensitive paths (not in production)
- **Legacy code**: Existing `log::` imports will be migrated to `tracing` over time via `tracing-log` bridge

**Example:**
```rust
use tracing::{info, debug, instrument};

#[instrument(skip(db), fields(user_id = %user.id))]
async fn create_work_item(user: &User, db: &Database) -> Result<WorkItem> {
    debug!("Creating work item");
    let item = db.insert(...).await?;
    info!(item_id = %item.id, "Work item created");
    Ok(item)
}
```

**Rust:**
- **CRITICAL: Use workspace dependencies** - All dependencies defined in root `Cargo.toml` `[workspace.dependencies]`, member crates reference with `{ workspace = true }` syntax. NEVER add version numbers to member crates.
- Use `sqlx::query_as!` macro for compile-time SQL validation
- Repository methods return `Result<T, DbError>` for consistent error handling
- UUID type: Use `uuid::Uuid` in Rust models, stored as TEXT in SQLite
- Timestamps: Store as INTEGER (Unix timestamp), use `chrono::DateTime<Utc>` in code
- Foreign key references to platform tables (users, teams): Assume they exist but don't enforce at DB level

**Blazor/C#:**
- Models in `.Core` project match Rust backend models exactly (field names, types)
- DTOs for API communication separate from domain models
- Services inject `IProjectManagementService` interface
- WebSocket client auto-reconnects on disconnect
- Optimistic UI updates with server confirmation rollback pattern

## Important Design Principles

1. **Real-time first**: Build with WebSocket from day one, not as an afterthought
2. **Per-tenant isolation**: Never leak data across tenants; validate tenant_id from JWT
3. **Soft deletes everywhere**: Preserve audit trail; use `deleted_at IS NULL` in queries
4. **Protobuf message versioning**: Field numbers are permanent; add new fields, don't reuse numbers
5. **LLM-friendly schema**: Prioritize clarity and completeness over normalization when it helps AI understanding
6. **Single polymorphic work item table**: Don't create separate tables for Project/Epic/Story/Task

## Testing Strategy

- **Backend**: Integration tests using in-memory SQLite databases
- **Repository tests**: CRUD operations, foreign key constraints, soft delete behavior
- **WebSocket tests**: Message encoding/decoding, subscription filtering, broadcast delivery
- **Protobuf tests**: Serialization round-trips, backward compatibility
- **Frontend**: Component tests with bUnit, WebSocket mock for UI testing

## Documentation

All architectural documentation lives in `docs/`:
- `database-schema.md`: Complete schema with all tables, columns, indexes
- `websocket-protocol.md`: Message format, subscription model, event types
- `backend-architecture.md`: Crate structure, dependency graph
- `frontend-architecture.md`: Project structure, component hierarchy
- `implementation-plan-revised.md`: Session-by-session implementation guide
- `features-roadmap.md`: v1.0 features and future roadmap
- `llm-integration-guide.md`: How LLMs can query the database
- `adr/`: Architecture Decision Records for key decisions

## Git Workflow

- Main branch: `main`
- Current untracked file: `docs/__Workflow__.md` (document the workflow for this project)
- License: MIT (LICENSE file exists)
- Standard commit message format: Brief summary in present tense

## Notes for Future Claude Instances

- **CRITICAL: Cargo workspace dependencies**: This project uses centralized dependency management. ALL dependencies must be defined in the root `Cargo.toml` `[workspace.dependencies]` section. Member crates reference them with `{ workspace = true }` syntax. NEVER add version numbers to member crate `Cargo.toml` files. This is a frequent mistake - always check the root `Cargo.toml` first.
- **Token budgets matter**: Sessions are designed to fit within 50k token limits. Don't try to build everything at once.
- **Follow the session plan**: `docs/implementation-plan-v2.md` has the roadmap. Build incrementally.
- **WebSocket is not optional**: This is a WebSocket-first architecture. REST API is secondary (Session 60).
- **Test as you build**: Each session includes unit and integration tests. Don't skip testing.
- **Refer to docs**: Don't guess at schema or protocol. All decisions are documented in `docs/` and `docs/adr`/.
- **Use justfile commands**: All build commands are defined in `justfile`. Use `just help` to see available commands. Never run raw cargo/dotnet commands when a justfile command exists.
