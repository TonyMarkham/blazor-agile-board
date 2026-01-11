# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Blazor Agile Board is a production-grade agile project management system built with **Blazor WebAssembly** (frontend) and **Rust + Axum** (backend). It's designed as a plugin for multi-tenant SaaS platforms with per-tenant SQLite databases and real-time collaboration via WebSocket + Protocol Buffers.

**Current Status**: Planning phase complete. Implementation begins with Session 10 (backend foundation).

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

**Backend (Rust):**
- No code exists yet. Backend will be created in Session 10.
- Once created:
  - Build: `cd backend && cargo build --workspace`
  - Test: `cargo test --workspace`
  - Run: `cargo run --bin pm-server`
  - Migrations: Use SQLx CLI (`sqlx migrate run --database-url <tenant_db_url>`)

**Frontend (Blazor):**
- No code exists yet. Frontend will be created in Session 30.
- Once created:
  - Build: `dotnet build frontend/ProjectManagement.sln`
  - Test: `dotnet test`
  - Run standalone: `dotnet run --project frontend/ProjectManagement.Wasm`

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

**Rust:**
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

- **No code exists yet**: This is purely planning phase. Backend starts in Session 10, frontend in Session 30.
- **Token budgets matter**: Sessions are designed to fit within 100k token limits. Don't try to build everything at once.
- **Follow the session plan**: `docs/implementation-plan-revised.md` has the roadmap. Build incrementally.
- **WebSocket is not optional**: This is a WebSocket-first architecture. REST API is secondary (Session 60).
- **Test as you build**: Each session includes integration tests. Don't skip testing.
- **Refer to docs**: Don't guess at schema or protocol. All decisions are documented in `docs/`.
