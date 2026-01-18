# Project State Summary

**Last Updated**: 2026-01-17
**Purpose**: Quick context for new Claude sessions. Read this first.

---

## TL;DR

**What**: Agile project management app (JIRA clone) with Blazor WASM frontend + Rust backend.

**Current Phase**: Backend ~70% complete, transitioning from multi-tenant to single-tenant desktop-first architecture. Frontend not started.

**Architecture Pivot**: Originally designed for multi-tenant SaaS, now simplified to one-process-per-tenant for desktop-first development (see `ProjectManager.md`).

---

## Architecture (Simplified)

```
┌─────────────────────────────────────┐
│         Tauri App (thin shell)      │
│  ┌───────────────────────────────┐  │
│  │     WebView (Blazor WASM)     │  │
│  │   ws://localhost:8080  ───────┼──┼──┐
│  └───────────────────────────────┘  │  │
└─────────────────────────────────────┘  │
                                         │
┌────────────────────────────────────────┼┐
│         pm-server (sidecar)            ││
│  ┌──────────┐  ┌──────────┐           ││
│  │  Axum    │◄─┤ WebSocket│◄──────────┘│
│  │  Server  │  │ Handler  │            │
│  └────┬─────┘  └──────────┘            │
│       │                                │
│  ┌────▼─────┐                          │
│  │  SQLite  │  .pm/data.db             │
│  └──────────┘  (adjacent to exe)       │
└────────────────────────────────────────┘
```

**Core Principle**: One `pm-server` process = one tenant. Multi-tenancy via multiple processes, not routing.

---

## What Exists

### Backend (Rust) - `backend/`

| Crate | Status | Description |
|-------|--------|-------------|
| `pm-core` | ✅ Complete | Domain models (WorkItem, Sprint, Comment, etc.) |
| `pm-db` | ✅ Complete | SQLite repositories, migrations, 60+ tests |
| `pm-proto` | ✅ Complete | Protobuf messages, code generation |
| `pm-auth` | ✅ Complete | JWT validation (HS256/RS256), rate limiting |
| `pm-config` | ✅ Complete | Config loading (TOML, env vars, CLI) - **not yet used by pm-server** |
| `pm-ws` | 🔶 Partial | WebSocket infra exists, handlers framework exists, **not wired to DB** |
| `pm-server` | 🔶 Partial | Server runs, WebSocket works, **no database connection** |

### Desktop App - `desktop/`

| Component | Status | Description |
|-----------|--------|-------------|
| `src-tauri/` | ✅ Scaffolded | Tauri app shell, sidecar config for pm-server |

### Frontend (Blazor) - `frontend/`

| Component | Status | Description |
|-----------|--------|-------------|
| All | ❌ Not started | No Blazor code exists yet |

---

## What's Missing (Gap Analysis)

### Critical Path to Working App

1. **Integrate pm-config into pm-server** (Session 05)
   - pm-server has its own config.rs (env-only, requires JWT)
   - pm-config crate exists but isn't used
   - Need to unify config and make auth optional for desktop

2. **Wire database to pm-server** (Session 10)
   - Add `SqlitePool` to `AppState`
   - Create pool at startup in `main.rs`
   - Run migrations on startup
   - Pass pool to WebSocket handlers

3. **Wire handlers to process messages**
   - Handler framework exists (`pm-ws/src/handlers/`)
   - Need to connect message dispatch to actual handlers
   - Need broadcast channel for real-time events

3. **Build Blazor frontend**
   - ProjectManagement.Core (models)
   - ProjectManagement.Services (WebSocket client)
   - ProjectManagement.Components (UI - Radzen)
   - ProjectManagement.Wasm (host)

4. **Tauri integration**
   - Configure pm-server as sidecar
   - Auto-start/stop lifecycle
   - Port discovery

---

## Key Files to Read

| File | Purpose |
|------|---------|
| `ProjectManager.md` | **Current architecture** - simplified single-tenant design |
| `CLAUDE.md` | Project conventions, commands, code patterns |
| `docs/implementation-plan-v2.md` | **Current implementation plan** (7 sessions) |
| `docs/database-schema.md` | Complete database schema |
| `docs/websocket-protocol.md` | WebSocket message format |
| `docs/implementation-plan-revised.md` | ~~Original session plan~~ (outdated, multi-tenant) |
| `docs/adr/` | Architecture Decision Records |

---

## Session History

| Session | Status | What Was Built |
|---------|--------|----------------|
| 10 | ✅ Done | Database, migrations, repositories, protobuf |
| 15 | ✅ Done | 60 integration tests, race condition fix |
| 20 | ✅ Done | WebSocket infrastructure, connection handling |
| 25 | ✅ Done | WebSocket integration tests |
| 30.1 | ✅ Done | Schema changes, new repositories |
| 30.2 | ✅ Done | Handler framework, protobuf extensions |
| 30.3 | ✅ Done | Repository refactor, model updates |
| **Refactor** | ✅ Done | **Simplified to single-tenant architecture** |

**Next**: Wire database to server, then build frontend.

---

## Git History (Recent)

```
abebb48 refactor: Remove multi-tenant code for single-tenant desktop architecture
aa51105 feat: Add Tauri desktop app and simplify architecture
8e73572 feat: Session 30.3 prerequisites - Repository refactor and model updates
4d23f38 feat: Session 30.2 - Handler framework and protobuf extensions
da370d7 feat: Session 30.1 - Schema infrastructure and clippy cleanup
4711671 feat: Session 25 - Comprehensive WebSocket integration tests
bbfbbd7 feat: Session 20 - Production-grade WebSocket infrastructure
```

---

## Quick Commands

```bash
# Build everything
cd backend && cargo build --workspace

# Run tests
cargo test --workspace

# Run server (no DB wired yet)
cargo run --bin pm-server

# Tauri dev (once wired)
cd desktop && cargo tauri dev
```

---

## Codebase Structure

```
blazor-agile-board/
├── PROJECT_STATE.md          # THIS FILE - read first
├── ProjectManager.md         # Current simplified architecture
├── CLAUDE.md                 # Project conventions
├── backend/
│   ├── Cargo.toml            # Workspace root
│   ├── crates/
│   │   ├── pm-core/          # Domain models
│   │   ├── pm-db/            # Repositories + migrations
│   │   ├── pm-proto/         # Protobuf
│   │   ├── pm-auth/          # JWT + rate limiting
│   │   ├── pm-config/        # Configuration
│   │   └── pm-ws/            # WebSocket (partially wired)
│   └── pm-server/            # Main binary
├── desktop/
│   └── src-tauri/            # Tauri app shell
├── frontend/                 # Empty - Blazor not started
├── proto/
│   └── messages.proto        # Protobuf definitions
└── docs/
    ├── database-schema.md
    ├── websocket-protocol.md
    ├── implementation-plan-revised.md  # Outdated
    └── adr/                  # Architecture decisions
```

---

## What Changed in Simplification

| Before (Multi-tenant) | After (Single-tenant) |
|-----------------------|-----------------------|
| `TenantConnectionManager` with pool cache | Single `SqlitePool` |
| `TenantContext` in handlers | Direct pool access |
| `tenant_id` in `HandlerContext` | Removed |
| Per-tenant broadcast channels | Single broadcast channel |
| JWT required with tenant extraction | Optional auth, user-only |
| Complex routing by tenant | One process = one DB |

---

## Next Steps

See `docs/implementation-plan-v2.md` for the full plan. Summary:

| Session | Focus |
|---------|-------|
| **05** | Integrate pm-config, make auth optional |
| **10** | Wire database to server, implement handlers |
| **20** | Blazor foundation & WebSocket client |
| **30** | Work item UI with Radzen |
| **40** | Tauri desktop integration |
| **50** | Sprints & comments |
| **60** | Time tracking & dependencies |
| **70** | Polish, activity logging, docs |

**Start with Session 05**: Make pm-server use pm-config crate, enable auth-optional mode for desktop.

---

*This document should be updated after each major session.*
