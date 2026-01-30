# Blazor Agile Board

Blazor + Rust agile board with real-time collaboration, sprints, and LLM-friendly database design.

## Overview

A production-grade agile project management system built with Blazor WebAssembly and Rust. Designed as a plugin for SaaS platforms with real-time collaboration via WebSocket and an LLM-friendly database structure.

## Features

### Core Features (in active development)
- **Work Item Hierarchy**: Project → Epic → Story → Task
- **Sprint Management**: Plan, start, and complete sprints
- **Real-time Collaboration**: WebSocket + Protobuf updates
- **Time Tracking**: Running timers and manual time entry
- **Dependency Management**: Task dependency tracking
- **Comments & Activity**: Comment threads and activity log feed
- **LLM Integration**: Self-documenting database context

## Architecture

### Tech Stack
- **Frontend**: Blazor WebAssembly with Radzen UI components
- **Backend**: Rust + Axum web framework
- **Database**: SQLite with `pm_*` table prefix
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
├── docs/                         # Architecture and guides
│   ├── adr/                      # Architecture Decision Records
│   ├── database-schema.md
│   ├── websocket-protocol.md
│   ├── backend-architecture.md
│   ├── frontend-architecture.md
│   ├── llm-integration-guide.md
│   ├── implementation-plan-v2.md
│   ├── GETTING_STARTED.md
│   ├── USER_GUIDE.md
│   ├── DEPLOYMENT_GUIDE.md
│   ├── API_DOCUMENTATION.md
│   └── TROUBLESHOOTING.md
├── backend/                      # Rust workspace
│   ├── crates/
│   │   ├── pm-core/              # Domain models and business logic
│   │   ├── pm-db/                # Database layer with SQLx
│   │   ├── pm-api/               # REST API (planned)
│   │   ├── pm-ws/                # WebSocket server
│   │   ├── pm-auth/              # JWT authentication
│   │   └── pm-proto/             # Protobuf message definitions
│   └── pm-server/                # Main binary
└── frontend/                     # Blazor WebAssembly
    ├── ProjectManagement.Core/        # Models, DTOs, interfaces
    ├── ProjectManagement.Services/    # Business logic, WebSocket client
    ├── ProjectManagement.Components/  # Razor Class Library (RCL)
    └── ProjectManagement.Wasm/        # Standalone WASM host
```

## Implementation Plan

The project is being built in focused sessions (see implementation plan):

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

**Current Phase**: Implementation in progress

Refer to the session plans in `docs/session-plans/` for current scope.

## Documentation

- [Architecture Decision Records](docs/adr/README.md)
- [Database Schema](docs/database-schema.md)
- [WebSocket Protocol](docs/websocket-protocol.md)
- [Backend Architecture](docs/backend-architecture.md)
- [Frontend Architecture](docs/frontend-architecture.md)
- [LLM Integration Guide](docs/llm-integration-guide.md)
- [Implementation Plan](docs/implementation-plan-v2.md)
- [Features Roadmap](docs/features-roadmap.md)

## License

MIT License (see [LICENSE](LICENSE))

## Contributing

This project is currently in initial development. Contribution guidelines will be added after v1.0 release.

---

**Built with**: Blazor WebAssembly • Rust • Axum • SQLite • Radzen • Protocol Buffers
