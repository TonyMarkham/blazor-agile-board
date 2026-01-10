# Blazor Agile Board

Blazor + Rust agile board with real-time collaboration, sprints, and LLM-friendly multi-tenant database.

## Overview

A production-grade agile project management system built with Blazor WebAssembly and Rust. Designed as a plugin for multi-tenant SaaS platforms with per-tenant data isolation, real-time collaboration via WebSocket, and an LLM-friendly database structure.

## Features

### v1.0 Core Features
- **Work Item Hierarchy**: Project → Epic → Story → Task
- **Sprint Management**: Plan, start, and complete sprints with velocity tracking
- **Real-time Collaboration**: WebSocket + Protobuf for instant updates across all connected clients
- **Time Tracking**: Running timers and manual time entry
- **Dependency Management**: 2-way dependency tracking with cycle detection
- **Comments & Activity**: Full audit trail with comment threads
- **Swim Lanes**: Customizable Kanban board (backlog, in-progress, in-review, done)
- **LLM Integration**: Self-documenting database for AI-assisted project management

## Architecture

### Tech Stack
- **Frontend**: Blazor WebAssembly with Radzen UI components
- **Backend**: Rust + Axum web framework
- **Database**: Per-tenant SQLite with `pm_*` table prefix
- **Communication**: WebSocket with Protocol Buffers
- **Authentication**: JWT tokens

### Key Design Decisions
- **Plugin Architecture**: Razor Class Library for portability (ADR-0001)
- **Per-tenant SQLite**: Complete data isolation with table injection (ADR-0002)
- **Radzen Components**: Professional UI out of the box (ADR-0003)
- **Rust Backend**: High performance, type safety, matches host platform (ADR-0004)
- **WebSocket-First**: Real-time collaboration from day one (ADR-0005)

## Project Structure

```
blazor-agile-board/
├── docs/                    # Architecture documentation
│   ├── adr/                # Architecture Decision Records
│   ├── database-schema.md
│   ├── websocket-protocol.md
│   ├── backend-architecture.md
│   ├── frontend-architecture.md
│   ├── llm-integration-guide.md
│   └── implementation-plan-revised.md
├── backend/                # Rust workspace (to be created in Session 10)
│   ├── crates/
│   │   ├── pm-core/       # Domain models and business logic
│   │   ├── pm-db/         # Database layer with SQLx
│   │   ├── pm-api/        # REST API (optional, LLM queries only)
│   │   ├── pm-ws/         # WebSocket server
│   │   ├── pm-auth/       # JWT authentication
│   │   └── pm-proto/      # Protobuf message definitions
│   └── pm-server/         # Main binary
└── frontend/               # Blazor WebAssembly (to be created in Session 30)
    ├── Bab.Core/          # Shared models and interfaces
    ├── Bab.Components/    # Razor Class Library
    └── Bab.Wasm/          # Standalone WASM host
```

## Implementation Plan

The project is being built in focused sessions:

| Session | Focus | Estimated Tokens |
|---------|-------|------------------|
| **10** | Foundation, Database & Protobuf | 100k |
| **20** | WebSocket Infrastructure | 90k |
| **30** | Work Items (Backend + Frontend) | 95k |
| **40** | Sprints & Comments | 85k |
| **50** | Time Tracking & Dependencies | 80k |
| **60** | REST API for LLMs | 70k |
| **70** | Activity Logging & Polish | 75k |

Sessions are numbered 10, 20, 30... to leave room for incremental work.

## Database Schema

All tables use `pm_*` prefix to avoid collisions with host platform:

- `pm_work_items` - Polymorphic table (project/epic/story/task)
- `pm_sprints` - Sprint planning and tracking
- `pm_comments` - Comment threads on work items
- `pm_time_entries` - Time tracking with running timers
- `pm_dependencies` - Task dependencies and blockers
- `pm_activity_log` - Complete audit trail
- `pm_swim_lanes` - Kanban board lanes
- `pm_llm_context` - Self-documenting schema for LLM agents

See [docs/database-schema.md](docs/database-schema.md) for complete schema.

## LLM Integration

The database is designed to be easily queryable by LLMs for project management assistance:

- **Self-documenting**: `pm_llm_context` table contains schema docs, query patterns, and business rules
- **Complete audit trail**: `pm_activity_log` tracks all changes for context
- **Descriptive names**: Clear, semantic column names
- **Read-only REST API**: Optional endpoints for LLM queries (Session 60)

See [docs/llm-integration-guide.md](docs/llm-integration-guide.md) for details.

## Development Status

**Current Phase**: Planning Complete ✅

All architectural decisions have been documented. Implementation begins with Session 10.

## Documentation

- [Architecture Decision Records](docs/adr/README.md)
- [Database Schema](docs/database-schema.md)
- [WebSocket Protocol](docs/websocket-protocol.md)
- [Backend Architecture](docs/backend-architecture.md)
- [Frontend Architecture](docs/frontend-architecture.md)
- [LLM Integration Guide](docs/llm-integration-guide.md)
- [Implementation Plan](docs/implementation-plan-revised.md)
- [Features Roadmap](docs/features-roadmap.md)

## License

MIT License (to be added)

## Contributing

This project is currently in initial development. Contribution guidelines will be added after v1.0 release.

---

**Built with**: Blazor WebAssembly • Rust • Axum • SQLite • Radzen • Protocol Buffers
