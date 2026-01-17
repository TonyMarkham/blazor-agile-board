# Project Manager - Simplified Architecture

**Purpose**: This document describes the simplified single-tenant architecture for the Project Manager application. It supersedes the multi-tenant complexity in `docs/ARCHITECTURE.md` for the current implementation phase.

**Status**: Desktop-first development with path to SaaS deployment.

---

## Core Principle: One Process, One Tenant

Every `pm-server` instance serves exactly one tenant. Multi-tenancy is achieved by running multiple processes, not by routing within a single process.

```
Desktop Mode:                    SaaS Mode (per-tenant process):

~/.pm/                           /data/tenants/acme-corp/
├── config.toml                  ├── main.db (platform + pm_* tables)
├── data.db                      └── .pm/
└── logs/                            ├── config.toml → db: ../main.db
                                     └── logs/
     ↓                                    ↓
┌─────────────┐                  ┌─────────────┐
│ pm-server   │                  │ pm-server   │
│ :8080       │                  │ :8001       │
└─────────────┘                  └─────────────┘
```

---

## What We're Building

### Desktop Application (Tauri)

```
┌─────────────────────────────────────┐
│         Tauri App (thin shell)      │
│  ┌───────────────────────────────┐  │
│  │     WebView (Blazor WASM)     │  │
│  │                               │  │
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
│  │  SQLite  │  ~/.pm/data.db           │
│  └──────────┘                          │
└────────────────────────────────────────┘
```

**Key points:**
- Tauri is a thin WebView host, nothing more
- `pm-server` runs as a sidecar process (subprocess)
- Frontend connects via WebSocket to localhost
- Same `pm-server` binary works standalone for development

### SaaS Deployment (Future)

```
┌─────────────────────────────────────────────────────────────┐
│                    Platform Orchestrator                     │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  Master DB: tenants, subscriptions, plugin_access   │   │
│  └─────────────────────────────────────────────────────┘   │
│                            │                                 │
│         ┌──────────────────┼──────────────────┐             │
│         ▼                  ▼                  ▼             │
│  ┌─────────────┐   ┌─────────────┐   ┌─────────────┐       │
│  │ acme-corp   │   │ widgets-inc │   │ startup-xyz │       │
│  │ pm-server   │   │ pm-server   │   │ (no pm sub) │       │
│  │ :8001       │   │ :8002       │   │             │       │
│  └──────┬──────┘   └──────┬──────┘   └─────────────┘       │
└─────────┼──────────────────┼────────────────────────────────┘
          ▼                  ▼
   /tenants/acme/      /tenants/widgets/
   main.db             main.db
```

**Key points:**
- Platform orchestrator manages which tenants have PM plugin enabled
- Spawns/kills `pm-server` processes per tenant
- Each process connects to its tenant's database
- `pm-server` doesn't know about other tenants
- Table injection (`pm_*` tables) happens at migration time

---

## Project Structure

```
blazor-agile-board/
├── Cargo.toml                 # Workspace root
├── .sqlx/                     # SQLx offline query cache
│
├── backend/
│   ├── crates/
│   │   ├── pm-core/           # Domain models (WorkItem, Sprint, etc.)
│   │   ├── pm-db/             # SQLite repositories
│   │   ├── pm-proto/          # Protobuf messages
│   │   ├── pm-auth/           # [SIMPLIFY] JWT - only needed for SaaS
│   │   └── pm-ws/             # [SIMPLIFY] WebSocket handlers
│   └── pm-server/             # Main binary
│
├── desktop/
│   ├── src-tauri/             # Tauri app (sidecar host)
│   └── frontend/              # Static assets (placeholder for now)
│
├── frontend/                  # Blazor WASM (future)
│   ├── ProjectManagement.Core/
│   ├── ProjectManagement.Components/
│   └── ProjectManagement.Wasm/
│
└── proto/
    └── messages.proto         # Shared protobuf definitions
```

---

## What to Keep vs Simplify

### Keep As-Is

| Crate | Why |
|-------|-----|
| `pm-core` | Domain models are correct and needed |
| `pm-db/repositories/*` | CRUD operations are correct |
| `pm-db/migrations/` | Schema is correct |
| `pm-proto` | Protobuf messages are correct |

### Simplify

| Current | Problem | Simplified |
|---------|---------|------------|
| `TenantConnectionManager` | Manages multiple tenant pools | Single `SqlitePool` |
| `JwtValidator` | Extracts tenant_id from JWT | Optional auth, no tenant routing |
| `TenantContext` | Routes to tenant DB | Not needed - one DB per process |
| `ConnectionRateLimiter` | Per-tenant rate limiting | Simple global rate limit (optional) |
| `TenantBroadcaster` | Per-tenant broadcast channels | Single broadcast channel |
| `pm-ws` handlers | Take `TenantContext` | Take `SqlitePool` directly |

### Remove (Defer to SaaS Phase)

- Multi-tenant connection pooling logic
- Tenant-based request routing
- Per-tenant metrics/logging namespacing

---

## Configuration

Single config file at `~/.pm/config.toml` (or `.pm/config.toml` relative to tenant directory):

```toml
[server]
host = "127.0.0.1"    # Desktop: localhost only
# host = "0.0.0.0"    # SaaS: accept external connections
port = 8080

[database]
path = "data.db"      # Relative to config directory
# path = "../main.db" # SaaS: point to platform's tenant DB

[auth]
enabled = false       # Desktop: no auth needed
# enabled = true      # SaaS: validate JWT from platform

[logging]
level = "info"
dir = "logs"
```

**Loading order:**
1. Built-in defaults
2. Config file (`~/.pm/config.toml`)
3. Environment variables (`PM_SERVER_PORT`, etc.)
4. CLI flags (`--port 8080`)

---

## Simplified pm-server

### Current main.rs (conceptual)

```rust
// Over-engineered: multi-tenant routing
let tenant_manager = TenantConnectionManager::new(config);
let jwt_validator = JwtValidator::new(public_key);

// Every request:
// 1. Validate JWT
// 2. Extract tenant_id
// 3. Get pool from tenant_manager
// 4. Route to handler with TenantContext
```

### Simplified main.rs (target)

```rust
// Single-tenant: one pool, direct access
let config = Config::load()?;
let pool = SqlitePool::connect(&config.database.path).await?;

// Run migrations
pm_db::migrate(&pool).await?;

// Create broadcast channel for real-time updates
let (tx, _) = broadcast::channel(100);

// Build Axum app
let app = Router::new()
    .route("/ws", get(ws_handler))
    .layer(Extension(pool))
    .layer(Extension(tx));

// Start server
let addr = format!("{}:{}", config.server.host, config.server.port);
axum::serve(listener, app).await?;
```

### Simplified WebSocket Handler

```rust
// Current: TenantContext with tenant-specific pool
async fn handle_message(msg: WebSocketMessage, ctx: TenantContext) -> Result<...>

// Simplified: Direct pool access
async fn handle_message(
    msg: WebSocketMessage,
    pool: &SqlitePool,
    broadcast_tx: &broadcast::Sender<Event>,
) -> Result<WebSocketMessage> {
    match msg.payload {
        Payload::CreateWorkItem(req) => {
            let item = WorkItemRepository::create(pool, &req).await?;
            broadcast_tx.send(Event::WorkItemCreated(item.clone()))?;
            Ok(WorkItemCreated { item }.into())
        }
        // ... other handlers
    }
}
```

---

## Database

### Desktop Mode

```
~/.pm/data.db
├── pm_work_items
├── pm_sprints
├── pm_comments
├── pm_time_entries
├── pm_dependencies
├── pm_activity_log
├── pm_swim_lanes
└── pm_llm_context
```

### SaaS Mode (Table Injection)

Platform's `main.db` with `pm_*` tables injected:

```
/data/tenants/acme-corp/main.db
├── users              (platform)
├── teams              (platform)
├── subscriptions      (platform)
├── pm_work_items      (injected)
├── pm_sprints         (injected)
├── pm_comments        (injected)
└── ...
```

The `pm_*` tables can have foreign keys to platform tables (e.g., `pm_work_items.assignee_id → users.id`).

---

## Development Workflow

### Running Desktop App

```bash
# Terminal 1: Run pm-server directly (for debugging)
cargo run --bin pm-server

# Terminal 2: Run Tauri app (connects to pm-server)
cargo tauri dev
```

### Running pm-server Standalone

```bash
# Development
cargo run --bin pm-server

# With custom config
cargo run --bin pm-server -- --config /path/to/config.toml

# With CLI overrides
cargo run --bin pm-server -- --port 9000 --log-level debug
```

### Database Migrations

```bash
# Apply migrations to development database
cargo sqlx migrate run --database-url sqlite:~/.pm/data.db

# Generate offline query cache after schema changes
cargo sqlx prepare --workspace
```

---

## Refactoring Checklist

When simplifying the codebase, follow this order:

### Phase 1: Config System
- [ ] Create `pm-config` crate with `Config` struct
- [ ] Implement TOML loading with defaults
- [ ] Add CLI argument parsing (clap)
- [ ] Environment variable overrides

### Phase 2: Simplify pm-server
- [ ] Remove `TenantConnectionManager` usage
- [ ] Create single `SqlitePool` at startup
- [ ] Run migrations on startup
- [ ] Create single broadcast channel

### Phase 3: Simplify pm-ws
- [ ] Remove `TenantContext` from handlers
- [ ] Pass `SqlitePool` directly to handlers
- [ ] Simplify connection tracking (no per-tenant grouping)
- [ ] Single broadcast channel for all clients

### Phase 4: Simplify pm-auth
- [ ] Make auth optional via config
- [ ] Keep JWT validation for SaaS mode
- [ ] Remove tenant_id extraction (process already knows its tenant)
- [ ] Simple user_id extraction only

### Phase 5: Tauri Integration
- [ ] Configure pm-server as Tauri sidecar
- [ ] Auto-start on app launch
- [ ] Auto-stop on app close
- [ ] Health check / port discovery

### Phase 6: Blazor Frontend
- [ ] Create ProjectManagement.Wasm project
- [ ] WebSocket client connecting to localhost
- [ ] Basic Kanban board UI

---

## Key Differences from docs/ARCHITECTURE.md

| Aspect | Original Architecture | Simplified Architecture |
|--------|----------------------|------------------------|
| Tenant model | Single process, multiple tenants | One process per tenant |
| Connection management | `TenantConnectionManager` with pool cache | Single `SqlitePool` |
| Request routing | JWT → tenant_id → pool lookup | Direct pool access |
| Broadcast channels | Per-tenant channels | Single channel |
| Auth complexity | Required, tenant extraction | Optional, user-only |
| Deployment | Single backend instance | Multiple instances (desktop or orchestrated) |

The original architecture was designed for a complex multi-tenant SaaS from day one. The simplified architecture builds a working desktop app first, with a clear path to SaaS by running multiple instances behind an orchestrator.

---

## Future: SaaS Orchestrator

When ready for SaaS deployment, build a separate orchestrator service that:

1. Manages tenant lifecycle (create, suspend, delete)
2. Spawns `pm-server` processes per tenant
3. Assigns ports and configures reverse proxy
4. Handles subscription/billing integration
5. Monitors process health

The `pm-server` binary stays simple - the orchestrator handles the complexity.

---

**Document Version**: 1.0
**Created**: 2025-01-16
**Status**: Active development guide
