# ADR-0006: Single-Tenant Desktop-First Architecture

## Status
Accepted

## Context
The original architecture (ADR-0002) assumed a multi-tenant SaaS deployment from day one, with per-tenant SQLite databases managed by a single server process. This added significant complexity:

- Connection pool management per active tenant
- Tenant resolution via JWT claims on every request
- Complex migration tooling to update all tenant databases
- Authentication required even for single-user scenarios

After reviewing priorities, we determined that:
1. The primary use case is a desktop application for individual users/teams
2. Multi-tenant SaaS is a future goal, not an immediate requirement
3. Simpler architecture enables faster iteration and easier debugging
4. Desktop deployment via Tauri provides excellent cross-platform support

## Decision
We adopt a **single-tenant, desktop-first architecture** with the principle:

> **One process = one tenant**

### Desktop Deployment Model
The application runs as a Tauri desktop app with pm-server as an embedded sidecar:

```
my-project/
├── ProjectManager.app (or .exe)
└── .pm/
    ├── config.toml      # Server configuration
    ├── data.db          # SQLite database (single tenant)
    └── logs/            # Application logs
```

### Key Characteristics
- **No authentication by default**: Desktop mode assumes the local user is authorized
- **Single SQLite database**: All data in `.pm/data.db` relative to working directory
- **Sidecar process**: pm-server starts with the Tauri app, stops when app closes
- **Local-first**: Works offline, no external dependencies
- **Configuration**: `.pm/config.toml` with sensible defaults (server starts without config file)

### Path to Multi-Tenant SaaS
The architecture supports future SaaS deployment without rewriting:

1. **Session 100 (future)**: Add SaaS orchestrator that spawns pm-server instances per tenant
2. Each tenant gets dedicated process with isolated database
3. Orchestrator handles routing, lifecycle, and resource management
4. Enable authentication (`auth.enabled = true`) for SaaS mode
5. Core pm-server code remains unchanged

## Consequences

### Positive
- **Simplicity**: No tenant resolution, connection pooling, or multi-tenant complexity
- **Fast development**: Can build and test without authentication infrastructure
- **Debuggability**: Single database file, single process, straightforward state
- **Offline capable**: Desktop app works without network connectivity
- **Data ownership**: Users own their data locally (privacy-friendly)
- **Clear upgrade path**: Same pm-server binary works for desktop and SaaS
- **Resource isolation**: Each tenant process has dedicated memory/CPU (SaaS mode)

### Negative
- **Resource overhead in SaaS**: Each tenant requires a separate process (mitigated by lazy spawning)
- **No cross-tenant features**: Can't query across tenants (rarely needed, security benefit)
- **Orchestrator required**: SaaS deployment needs additional infrastructure layer

### Supersedes
This ADR supersedes [ADR-0002](0002-per-tenant-sqlite-databases_superseded.md) (Per-Tenant SQLite Databases), which described a multi-tenant server managing multiple tenant databases in a single process.

### Related Decisions
- [ADR-0001](0001-plugin-architecture-with-table-injection.md): Table injection still applies (pm_* prefix)
- [ADR-0004](0004-rust-axum-backend.md): pm-server runs as Tauri sidecar
- [ADR-0005](0005-websocket-with-protobuf.md): WebSocket simplified (no multi-tenant channels)
