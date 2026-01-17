# Blazor Agile Board - Architecture Guide

**Project**: Production-grade JIRA clone for multi-tenant SaaS platforms
**Tech Stack**: Blazor WebAssembly (frontend) + Rust/Axum (backend) + SQLite (per-tenant databases)
**Purpose**: This document explains HOW and WHY the system is architected

**Audience**: Future developers, Claude instances, Tony reviewing decisions

---

## Table of Contents

1. [System Overview](#system-overview)
2. [Why These Technologies?](#why-these-technologies)
3. [Multi-Tenancy Architecture](#multi-tenancy-architecture)
4. [Frontend Architecture (Blazor)](#frontend-architecture-blazor)
5. [Backend Architecture (Rust)](#backend-architecture-rust)
6. [Communication Protocol](#communication-protocol)
7. [Database Design](#database-design)
8. [Security Model](#security-model)
9. [Real-Time Collaboration](#real-time-collaboration)
10. [Plugin Architecture](#plugin-architecture)
11. [Data Flow Examples](#data-flow-examples)
12. [Deployment Model](#deployment-model)

---

## System Overview

### What We're Building

**A production-grade project management system (JIRA clone) that:**
- Runs as a plugin in multi-tenant SaaS platforms
- Supports real-time collaboration (like Google Docs for project management)
- Handles hundreds of tenants on a single server
- Provides instant updates when team members make changes
- Works offline-first with sync on reconnect
- Can be queried by LLMs for AI-powered project insights

### High-Level Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Browser (Blazor WASM)                   │
│  ┌─────────────┐  ┌─────────────┐  ┌──────────────────┐   │
│  │  Kanban UI  │  │  Sprint UI  │  │  Comments UI     │   │
│  └─────────────┘  └─────────────┘  └──────────────────┘   │
│         │                │                    │             │
│         └────────────────┴────────────────────┘             │
│                          │                                  │
│                  ┌───────▼────────┐                        │
│                  │ WebSocket      │                        │
│                  │ Client         │                        │
│                  └───────┬────────┘                        │
└──────────────────────────┼─────────────────────────────────┘
                           │ Binary Protobuf over WebSocket
                           │
┌──────────────────────────▼─────────────────────────────────┐
│                    Rust Backend (Axum)                      │
│  ┌────────────────────────────────────────────────────┐   │
│  │           WebSocket Connection Manager             │   │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────────┐    │   │
│  │  │ Conn A   │  │ Conn B   │  │ Conn C       │    │   │
│  │  │ (Tenant1)│  │ (Tenant1)│  │ (Tenant2)    │    │   │
│  │  └──────────┘  └──────────┘  └──────────────┘    │   │
│  └────────────────────────────────────────────────────┘   │
│                          │                                  │
│         ┌────────────────┼────────────────┐                │
│         │                │                │                │
│  ┌──────▼──────┐  ┌──────▼──────┐  ┌─────▼──────┐        │
│  │  Handlers   │  │  Auth       │  │  Broadcast │        │
│  │  (CRUD)     │  │  (JWT)      │  │  (Events)  │        │
│  └──────┬──────┘  └─────────────┘  └────────────┘        │
│         │                                                   │
│  ┌──────▼──────────────────────────────────────────┐     │
│  │     Tenant Connection Manager                    │     │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐      │     │
│  │  │ Pool1    │  │ Pool2    │  │ Pool3    │      │     │
│  │  │(Tenant1) │  │(Tenant2) │  │(Tenant3) │      │     │
│  │  └────┬─────┘  └────┬─────┘  └────┬─────┘      │     │
│  └───────┼─────────────┼─────────────┼────────────┘     │
└──────────┼─────────────┼─────────────┼──────────────────┘
           │             │             │
┌──────────▼──┐  ┌───────▼──────┐  ┌──▼────────────┐
│  tenant1    │  │  tenant2     │  │  tenant3      │
│  main.db    │  │  main.db     │  │  main.db      │
│  (SQLite)   │  │  (SQLite)    │  │  (SQLite)     │
└─────────────┘  └──────────────┘  └───────────────┘
```

### Request Flow Example

**User Story: Developer moves a task from "In Progress" to "Done"**

1. **Browser**: User drags card on Kanban board
2. **Blazor**: Captures event, builds protobuf message with optimistic version
3. **WebSocket**: Sends binary message to server
4. **Rust Backend**:
   - Validates JWT (who is this user?)
   - Gets tenant's database pool
   - Checks permission (can they edit this project?)
   - Starts transaction
   - Updates work item with version check (optimistic lock)
   - Logs activity to audit trail
   - Commits transaction
   - Builds response
5. **Response**: Sent back to requester with message ID correlation
6. **Broadcast**: Sent to all other team members viewing that project
7. **All Browsers**: Update UI instantly (no polling!)

---

## Why These Technologies?

### Blazor WebAssembly

**What it is**: C# code compiled to WebAssembly, runs entirely in browser

**Why chosen:**
| Requirement | Solution |
|-------------|----------|
| Type-safe UI | Compiled C#, not loosely-typed JavaScript |
| Component reusability | Razor Class Libraries (RCL) as plugins |
| Rich UI library | Radzen components (professional out-of-box) |
| Offline-first | All code runs in browser, no server round-trips |
| .NET ecosystem | Leverage existing .NET SaaS platform |

**Alternatives considered:**
- **React**: Ecosystem larger, but JavaScript type safety weaker
- **Angular**: Heavier framework, steeper learning curve
- **Vue**: Simpler, but less enterprise-ready

### Rust Backend

**What it is**: Systems programming language, compiles to native code

**Why chosen:**
| Requirement | Solution |
|-------------|----------|
| Performance | Zero-cost abstractions, no garbage collection |
| Memory safety | Ownership system prevents leaks/crashes |
| Concurrency | Tokio async runtime handles 10k+ connections |
| SQLite integration | SQLx compile-time query validation |
| Type safety | Enum exhaustiveness, Result types force error handling |

**Alternatives considered:**
- **Go**: Simpler, but less type-safe (null pointers, implicit errors)
- **Node.js**: Fast prototyping, but GC pauses under load
- **C#**: Great, but heavier runtime (CLR) vs native binary

### SQLite Per-Tenant

**What it is**: Embedded SQL database, one file per tenant

**Why chosen:**
| Requirement | Solution |
|-------------|----------|
| Tenant isolation | Physical separation (impossible to leak data) |
| LLM-friendly | AI can read entire tenant context in one file |
| Backup simplicity | Copy file = complete tenant backup |
| Resource efficiency | Only active tenants consume memory |
| No connection limits | Each tenant has own pool |

**Alternatives considered:**
- **PostgreSQL shared**: Single DB, tenant_id in every query (risk of leaks)
- **PostgreSQL per-tenant**: Better isolation, but heavy (1 DB = 1 Postgres instance)
- **DynamoDB**: Serverless, but not LLM-queryable

### Protocol Buffers + WebSocket

**What it is**: Binary serialization over full-duplex socket

**Why chosen:**
| Requirement | Solution |
|-------------|----------|
| Real-time updates | Full duplex (server can push anytime) |
| Bandwidth efficiency | Binary encoding ~3x smaller than JSON |
| Type safety | .proto file defines contract, code-generated |
| Versioning | Forward/backward compatible (field numbers) |
| Cross-language | Rust backend + Blazor frontend use same schema |

**Alternatives considered:**
- **REST + JSON**: Simple, but polling required for updates (wasteful)
- **GraphQL**: Good for queries, but not real-time (subscriptions complex)
- **SignalR**: .NET-native, but requires server-side .NET (we use Rust)

---

## Multi-Tenancy Architecture

### The Tenant Problem

**Scenario**: SaaS platform serves 1000 companies. Each company has:
- 50-500 users
- 10-100 projects
- 1000-10000 work items

**Requirements:**
1. **Complete isolation**: Company A cannot see Company B's data (ever)
2. **Resource efficiency**: Don't allocate resources for inactive tenants
3. **Independent scaling**: One tenant's load doesn't affect others
4. **Data portability**: Easy to move tenant between servers
5. **LLM accessibility**: AI can analyze tenant's entire project history

### Our Solution: Per-Tenant SQLite Files

```
/data/tenants/
├── acme-corp/
│   └── main.db          (Contains ACME's projects, work items, users)
├── widgets-inc/
│   └── main.db          (Contains Widgets Inc's data)
└── startup-xyz/
    └── main.db          (Contains Startup XYZ's data)
```

**Key Benefits:**

1. **Physical Isolation**
   ```rust
   // This is literally impossible:
   let pool_a = get_pool("acme-corp");
   let pool_b = get_pool("widgets-inc");

   // Query from pool_a CANNOT access pool_b (different files!)
   sqlx::query!("SELECT * FROM work_items WHERE tenant_id = ?", "widgets-inc")
       .fetch_all(pool_a)  // Returns empty - table doesn't even exist in this file
   ```

2. **Resource Efficiency**
   ```rust
   pub struct TenantConnectionManager {
       pools: HashMap<String, SqlitePool>,  // Only active tenants in memory
   }

   // Lazy loading:
   if tenant not in pools {
       create_pool();  // First connection
   }
   // Subsequent connections reuse pool
   ```

3. **Simple Operations**
   ```bash
   # Backup tenant
   cp /data/tenants/acme-corp/main.db /backups/acme-corp-2024-01-16.db

   # Delete tenant
   rm -rf /data/tenants/acme-corp/

   # Move tenant to another server
   scp /data/tenants/acme-corp/main.db server2:/data/tenants/
   ```

### JWT-Based Tenant Resolution

**Flow:**
```
1. User logs into SaaS platform
   ↓
2. Platform issues JWT with claims:
   {
     "tenant_id": "acme-corp",
     "user_id": "alice@acme.com",
     "roles": ["editor"]
   }
   ↓
3. Browser connects to WebSocket with JWT in header
   ↓
4. Rust backend validates JWT signature
   ↓
5. Extracts tenant_id + user_id
   ↓
6. Gets database pool for that tenant
   ↓
7. Every handler uses that pool (impossible to cross tenants)
```

**Security Properties:**
- **No tenant_id in query strings**: Would be forgeable
- **JWT signed by platform**: Backend trusts platform's authentication
- **Connection bound to tenant**: Once extracted, immutable for connection lifetime
- **Per-message validation**: Even if JWT expires mid-session, next message fails

---

## Frontend Architecture (Blazor)

### Project Structure

```
frontend/
├── ProjectManagement.Core/           # Domain models (no UI, no dependencies)
│   ├── Models/
│   │   ├── WorkItem.cs
│   │   ├── Sprint.cs
│   │   └── Comment.cs
│   └── Interfaces/
│       └── IProjectManagementService.cs
│
├── ProjectManagement.Services/       # Business logic, API clients
│   ├── WebSocketClient.cs            # Protobuf WebSocket implementation
│   ├── ProjectManagementService.cs   # Implements IProjectManagementService
│   └── StateManagement/
│       └── WorkItemStore.cs          # Local state for offline-first
│
├── ProjectManagement.Components/     # Razor Class Library (RCL) - UI components
│   ├── KanbanBoard.razor
│   ├── SprintPlanner.razor
│   ├── CommentThread.razor
│   └── wwwroot/
│       └── styles.css                # Component styles
│
└── ProjectManagement.Wasm/           # Standalone WASM host (for testing)
    ├── Program.cs
    └── Pages/
        └── Index.razor               # Host page
```

### Why This Structure?

**Core Project (Models Only):**
- No dependencies = portable
- Can be used by multiple frontends (Blazor Server, MAUI, etc.)
- Matches backend models exactly (field names, types)

**Services Project (Business Logic):**
- WebSocket client handles connection/reconnection
- State management for optimistic updates
- Background sync on reconnect
- Dependency injection ready

**Components Project (RCL = Plugin):**
- **This is the key**: Entire UI packaged as a library
- Parent platform includes as NuGet package
- Zero integration code needed
- Professional UI out-of-box (Radzen)

**WASM Project (Testing Host):**
- Standalone development/testing
- Production uses parent platform's host

### State Management Pattern

**Problem**: WebSocket is async, UI needs immediate feedback

**Solution**: Optimistic updates with rollback

```csharp
public class WorkItemStore
{
    private List<WorkItem> _items = new();

    public async Task UpdateWorkItemAsync(Guid id, string status, int expectedVersion)
    {
        // 1. Optimistic update (instant UI feedback)
        var item = _items.First(x => x.Id == id);
        var oldStatus = item.Status;
        item.Status = status;
        NotifyStateChanged();  // UI updates immediately

        try {
            // 2. Send to server
            var response = await _webSocket.UpdateWorkItemAsync(id, status, expectedVersion);

            // 3. Server confirms - update version
            item.Version = response.Version;
        }
        catch (ConflictException ex) {
            // 4. Conflict! Rollback + notify user
            item.Status = oldStatus;
            NotifyStateChanged();
            ShowConflictDialog(ex.CurrentVersion);
        }
    }
}
```

**User Experience:**
- ✅ Instant feedback (feels local)
- ✅ Network errors don't freeze UI
- ✅ Conflicts are recoverable

### WebSocket Client Implementation

```csharp
public class ProjectManagementWebSocketClient
{
    private ClientWebSocket _socket;
    private ConcurrentDictionary<string, TaskCompletionSource<WebSocketMessage>> _pendingRequests;

    // Send request, await response
    public async Task<WorkItemCreated> CreateWorkItemAsync(CreateWorkItemRequest request)
    {
        var messageId = Guid.NewGuid().ToString();
        var tcs = new TaskCompletionSource<WebSocketMessage>();
        _pendingRequests[messageId] = tcs;

        var message = new WebSocketMessage {
            MessageId = messageId,
            Payload = new Payload { CreateWorkItemRequest = request }
        };

        await SendAsync(message);

        var response = await tcs.Task;  // Waits for response with matching message_id
        return response.WorkItemCreated;
    }

    // Receive loop
    private async Task ReceiveLoopAsync()
    {
        while (true) {
            var message = await ReceiveAsync();

            if (_pendingRequests.TryRemove(message.MessageId, out var tcs)) {
                // This is a response to our request
                tcs.SetResult(message);
            } else {
                // This is a broadcast from server
                HandleBroadcast(message);
            }
        }
    }

    // Broadcast handling
    private void HandleBroadcast(WebSocketMessage message)
    {
        switch (message.Payload) {
            case WorkItemCreated created:
                _store.AddWorkItem(created.WorkItem);  // Update local state
                OnWorkItemCreated?.Invoke(created);    // Notify subscribers
                break;
            // ... other events
        }
    }
}
```

---

## Backend Architecture (Rust)

### Crate Structure

```
backend/
├── pm-core/              # Domain models (like frontend Core project)
│   ├── models/
│   │   ├── work_item.rs
│   │   ├── sprint.rs
│   │   └── activity_log.rs
│   └── error.rs
│
├── pm-db/                # Database layer
│   ├── connection/
│   │   └── tenant_connection_manager.rs
│   ├── repositories/
│   │   ├── work_item_repository.rs
│   │   ├── sprint_repository.rs
│   │   └── idempotency_repository.rs
│   └── migrations/       # SQL migration files
│
├── pm-proto/             # Protobuf generated code
│   ├── proto/messages.proto
│   └── src/generated/pm.rs
│
├── pm-auth/              # JWT validation, rate limiting
│   ├── jwt_validator.rs
│   └── rate_limiter.rs
│
├── pm-ws/                # WebSocket server
│   ├── handlers/         # Message handlers
│   │   ├── work_item.rs
│   │   ├── subscription.rs
│   │   └── query.rs
│   ├── web_socket_connection.rs
│   ├── tenant_broadcaster.rs
│   └── connection_registry.rs
│
└── pm-server/            # Main binary
    └── main.rs
```

### Why This Crate Structure?

**pm-core**: Pure business logic
- No I/O dependencies
- Can be tested without database
- Models match frontend exactly

**pm-db**: All database operations
- Repository pattern isolates SQLx details
- Migrations colocated with code
- TenantConnectionManager handles pooling

**pm-proto**: Generated protobuf code
- Build script runs `protoc` compiler
- Both backend and frontend use same `.proto` file
- Type-safe contracts

**pm-auth**: Security concerns
- JWT validation (verifies platform's signature)
- Rate limiting (per-connection, per-tenant)
- Separate crate = security audit boundary

**pm-ws**: WebSocket logic
- Connection management
- Message routing (dispatch to handlers)
- Broadcasting (fan-out to subscribers)

**pm-server**: Composition root
- Wires everything together
- Minimal code (dependency injection)

### Handler Pattern

Every handler follows this structure:

```rust
pub async fn handle_create(
    request: CreateWorkItemRequest,
    ctx: HandlerContext,
) -> Result<(WebSocketMessage, BroadcastInfo), WsError> {

    // 1. Idempotency check (cached response?)
    if let Some(cached) = check_idempotency(&ctx.pool, &ctx.message_id).await? {
        return Ok((cached, BroadcastInfo::none()));
    }

    // 2. Input validation
    validate_create_request(&request)?;

    // 3. Authorization
    check_permission(&ctx, project_id, Permission::Edit).await?;

    // 4. Business logic in transaction
    let mut tx = ctx.pool.begin().await?;

    let work_item = WorkItem::new(...);
    WorkItemRepository::create(&mut tx, &work_item).await?;

    let log = ActivityLog::created("WorkItem", work_item.id, ctx.user_id);
    ActivityLogRepository::create(&mut tx, &log).await?;

    tx.commit().await?;

    // 5. Build response
    let response = build_work_item_created_response(&ctx.message_id, &work_item, ctx.user_id);

    // 6. Store idempotency
    let response_bytes = response.encode_to_vec();
    store_idempotency(&ctx.pool, &ctx.message_id, &response_bytes).await?;

    // 7. Return response + broadcast info
    Ok((response, BroadcastInfo::new(project_id, "work_item_created")))
}
```

**Why this order?**
- **Idempotency first**: Cheapest check, avoids duplicate work
- **Validation next**: Fail fast before DB operations
- **Authorization**: Security-critical, but requires DB lookup
- **Transaction**: Atomic multi-step operation
- **Idempotency storage**: After success (don't cache failures)

### Repository Pattern (Executor Pattern)

**Problem**: Need same function to work with pool AND transaction

**Solution**: Generic over executor

```rust
pub struct WorkItemRepository;  // Zero-sized type (no fields)

impl WorkItemRepository {
    pub async fn create<'e, E>(
        executor: E,  // Can be &SqlitePool OR &mut Transaction
        work_item: &WorkItem
    ) -> DbErrorResult<()>
    where
        E: sqlx::Executor<'e, Database = sqlx::Sqlite>
    {
        sqlx::query!(
            "INSERT INTO pm_work_items (...) VALUES (...)",
            work_item.id,
            work_item.title,
            // ...
        )
        .execute(executor)  // Works with either!
        .await?;

        Ok(())
    }
}
```

**Usage:**
```rust
// Without transaction
WorkItemRepository::create(&pool, &work_item).await?;

// With transaction
let mut tx = pool.begin().await?;
WorkItemRepository::create(&mut tx, &work_item).await?;
WorkItemRepository::create(&mut tx, &another_item).await?;
tx.commit().await?;  // Both or neither
```

**Why zero-sized struct?**
- No memory overhead
- Clear namespace (`WorkItemRepository::create`)
- Can't accidentally create instances with wrong pool

---

## Communication Protocol

### Why WebSocket Over REST?

**REST limitations for real-time collaboration:**

| Problem | REST Solution | WebSocket Solution |
|---------|--------------|-------------------|
| Updates from others | Poll every 5s | Server pushes instantly |
| Bandwidth | Full JSON every poll | Only changes sent |
| Latency | 5s delay | <100ms |
| Server load | N clients × polls/sec | N clients × 1 connection |

**Example: 100 users viewing same project**
- **REST**: 100 × 12 polls/min × 10KB = 120MB/min
- **WebSocket**: 100 × 1 connection + events × 1KB = ~6MB/min (20x less)

### Protobuf Message Structure

```protobuf
message WebSocketMessage {
  string message_id = 1;        // For request/response correlation
  int64 timestamp = 2;          // For ordering/sync

  oneof payload {
    // Requests (client → server)
    CreateWorkItemRequest create_work_item_request = 10;
    UpdateWorkItemRequest update_work_item_request = 11;
    DeleteWorkItemRequest delete_work_item_request = 12;
    GetWorkItemsRequest get_work_items_request = 13;
    Subscribe subscribe = 14;

    // Responses/Events (server → client)
    WorkItemCreated work_item_created = 20;
    WorkItemUpdated work_item_updated = 21;
    WorkItemDeleted work_item_deleted = 22;
    WorkItemsList work_items_list = 23;
    Error error = 99;
  }
}
```

**Key Design Decisions:**

1. **message_id**: Client-generated UUID
   - Enables request/response correlation
   - Enables idempotency (same ID = same operation)
   - Survives network retries

2. **timestamp**: Server-generated
   - Ordering events
   - Detecting stale data
   - Conflict resolution

3. **oneof payload**: Type-safe union
   - One message contains exactly one payload
   - Compiler enforces exhaustive matching
   - Forward compatible (add new types without breaking old clients)

### Request/Response vs Broadcast

**Two message flows:**

**1. Request/Response (RPC-style):**
```
Client: CreateWorkItemRequest { message_id: "abc123", ... }
   ↓
Server: Processes request
   ↓
Server: WorkItemCreated { message_id: "abc123", ... }  ← Same ID!
   ↓
Client: Matches pending request by ID, resolves Promise
```

**2. Broadcast (Pub/Sub-style):**
```
Client A: Updates work item
   ↓
Server: Processes, saves to DB
   ↓
Server: Broadcasts to all subscribers (except Client A)
   ↓
Client B: WorkItemUpdated { message_id: "xyz789", ... }  ← New ID!
Client C: WorkItemUpdated { message_id: "xyz789", ... }  ← Same broadcast
```

**Why both?**
- **Response**: Requester needs confirmation + error handling
- **Broadcast**: Others need notification of change

### Subscription Model

**Problem**: User has access to 100 projects, but only viewing 3

**Solution**: Explicit subscription

```protobuf
message Subscribe {
  repeated string project_ids = 1;  // "I want updates for these"
}

message Unsubscribe {
  repeated string project_ids = 1;  // "Stop sending updates for these"
}
```

**Server-side filtering:**
```rust
pub struct WebSocketConnection {
    subscriptions: HashSet<Uuid>,  // Projects this connection watches
}

async fn broadcast(&self, event: WorkItemCreated) {
    if self.subscriptions.contains(&event.work_item.project_id) {
        self.send(event).await?;
    }
    // else: silently drop (not subscribed)
}
```

**Benefits:**
- Client controls bandwidth usage
- Server doesn't send irrelevant updates
- Matches user's UI state (which projects are open)

---

## Database Design

### Schema Philosophy

**Principles:**

1. **Relational for source of truth**: Work items, sprints, comments stored as rows/columns
2. **Binary for caching**: Idempotency responses stored as protobuf BLOB
3. **Soft deletes everywhere**: `deleted_at IS NULL` in every query
4. **Optimistic locking**: `version` column on mutable tables
5. **Complete audit trail**: `pm_activity_log` records every change
6. **LLM-friendly naming**: Descriptive column names, no abbreviations

### Core Tables

#### pm_work_items (Polymorphic Table)

```sql
CREATE TABLE pm_work_items (
    id TEXT PRIMARY KEY,
    item_type TEXT NOT NULL CHECK(item_type IN ('project', 'epic', 'story', 'task')),

    -- Hierarchy (self-referential)
    parent_id TEXT,
    project_id TEXT NOT NULL,  -- Denormalized for query performance
    position INTEGER NOT NULL,  -- For drag-and-drop ordering

    -- Core fields
    title TEXT NOT NULL,
    description TEXT,
    status TEXT NOT NULL DEFAULT 'backlog',
    priority TEXT NOT NULL DEFAULT 'medium',

    -- Assignment
    assignee_id TEXT,

    -- Agile
    story_points INTEGER,
    sprint_id TEXT,

    -- Concurrency control
    version INTEGER NOT NULL DEFAULT 0,

    -- Audit
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    created_by TEXT NOT NULL,
    updated_by TEXT NOT NULL,
    deleted_at INTEGER,  -- Soft delete

    FOREIGN KEY (parent_id) REFERENCES pm_work_items(id),
    FOREIGN KEY (sprint_id) REFERENCES pm_sprints(id)
);
```

**Why polymorphic (not separate tables)?**
- Project/Epic/Story/Task share 90% of fields
- Hierarchy queries span types (need JOINs anyway)
- Single version number across entire tree
- Simpler frontend code (one model)

**Why denormalize project_id?**
```sql
-- Without denormalization (slow):
SELECT * FROM pm_work_items
WHERE id IN (SELECT id FROM recursive_tree_query WHERE root = 'project123');

-- With denormalization (fast):
SELECT * FROM pm_work_items WHERE project_id = 'project123';
```

#### pm_project_members (Authorization)

```sql
CREATE TABLE pm_project_members (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    role TEXT NOT NULL CHECK(role IN ('viewer', 'editor', 'admin')),
    created_at INTEGER NOT NULL,
    UNIQUE(project_id, user_id)
);
```

**Three roles:**
- **viewer**: Read-only (can view work items, comments)
- **editor**: Can CRUD work items, add comments
- **admin**: Can manage members, delete project

#### pm_idempotency_keys (Response Cache)

```sql
CREATE TABLE pm_idempotency_keys (
    message_id TEXT PRIMARY KEY,
    operation TEXT NOT NULL,
    result_bytes BLOB NOT NULL,      -- Binary protobuf!
    created_at INTEGER NOT NULL
);
```

**Why BLOB instead of JSON?**
- Guaranteed type-safe replay
- Smaller size (~30% less)
- Faster serialization
- Forward compatible (unknown fields preserved)

**Cleanup strategy:**
```sql
DELETE FROM pm_idempotency_keys WHERE created_at < (unixepoch() - 3600);
-- Run every 15 minutes via background job
```

#### pm_activity_log (Complete Audit Trail)

```sql
CREATE TABLE pm_activity_log (
    id TEXT PRIMARY KEY,
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    action TEXT NOT NULL,  -- 'created', 'updated', 'deleted'
    field_name TEXT,       -- Which field changed (for updates)
    old_value TEXT,
    new_value TEXT,
    user_id TEXT NOT NULL,
    timestamp INTEGER NOT NULL,
    comment TEXT
);
```

**Use cases:**
- Compliance (who changed what when)
- Debugging (why is this field wrong?)
- LLM queries ("what did Alice work on last week?")
- Undo functionality (revert to previous value)

### Indexes

**Query patterns drive index design:**

```sql
-- Work items by project (most common query)
CREATE INDEX idx_pm_work_items_project ON pm_work_items(project_id, deleted_at);

-- Work items by parent (hierarchy traversal)
CREATE INDEX idx_pm_work_items_parent ON pm_work_items(parent_id, deleted_at);

-- Work items by assignee (user's dashboard)
CREATE INDEX idx_pm_work_items_assignee ON pm_work_items(assignee_id, deleted_at);

-- Project members by user (authorization check)
CREATE INDEX idx_pm_project_members_user ON pm_project_members(user_id);

-- Activity log by entity (audit trail lookup)
CREATE INDEX idx_pm_activity_log_entity ON pm_activity_log(entity_type, entity_id, timestamp DESC);
```

**Why include deleted_at in indexes?**
```sql
-- Without deleted_at in index:
SELECT * FROM pm_work_items WHERE project_id = ? AND deleted_at IS NULL;
-- Index scan → Filter deleted rows (slower)

-- With deleted_at in index:
-- Index scan already filtered (faster)
```

---

## Security Model

### Defense in Depth

**Multiple layers, each can catch attacks:**

```
Layer 1: Network          → TLS encryption, rate limiting
Layer 2: Authentication   → JWT signature validation
Layer 3: Connection       → Tenant resolution, connection limits
Layer 4: Authorization    → Project membership checks
Layer 5: Operation        → Permission-specific validation
Layer 6: Data             → Optimistic locking, soft deletes
```

### Layer 1: Network Security

**TLS Required:**
```rust
// In production config
let tls_config = RustlsConfig::from_pem_file(
    "/etc/certs/cert.pem",
    "/etc/certs/key.pem"
).await?;

axum_server::bind_rustls("0.0.0.0:443", tls_config)
    .serve(app.into_make_service())
    .await?;
```

**Rate Limiting:**
```rust
pub struct ConnectionRateLimiter {
    limiter: RateLimiter,  // 100 messages/second per connection
}

// Before processing message:
self.rate_limiter.check()?;  // Throws error if exceeded
```

**Connection Limits:**
```rust
pub struct ConnectionLimits {
    global_max: usize,     // 10,000 total connections
    per_tenant_max: usize, // 100 connections per tenant
}

// Before accepting connection:
registry.register(tenant_id)?;  // Fails if limit exceeded
```

### Layer 2: JWT Authentication

**JWT Structure:**
```json
{
  "header": {
    "alg": "RS256",
    "typ": "JWT"
  },
  "payload": {
    "sub": "alice@acme.com",
    "tenant_id": "acme-corp",
    "user_id": "550e8400-e29b-41d4-a716-446655440000",
    "roles": ["editor"],
    "exp": 1704067200
  },
  "signature": "..."  // Signed by SaaS platform's private key
}
```

**Validation:**
```rust
pub struct JwtValidator {
    decoding_key: DecodingKey,  // Platform's public key
}

impl JwtValidator {
    pub fn validate(&self, token: &str) -> Result<TenantContext> {
        let claims = decode::<Claims>(
            token,
            &self.decoding_key,
            &Validation::new(Algorithm::RS256)
        )?;

        // Verify expiration
        if claims.exp < Utc::now().timestamp() {
            return Err(AuthError::Expired);
        }

        Ok(TenantContext {
            tenant_id: claims.tenant_id,
            user_id: claims.user_id,
        })
    }
}
```

**Why RSA (not HMAC)?**
- Platform signs with private key
- Backend verifies with public key
- Backend cannot forge tokens (doesn't have private key)

### Layer 3: Tenant Isolation

**Guaranteed by separate databases:**
```rust
// Each connection bound to one tenant
pub struct WebSocketConnection {
    tenant_context: TenantContext,  // Immutable for connection lifetime
    connection_manager: Arc<TenantConnectionManager>,
}

async fn handle_message(&self, message: WebSocketMessage) {
    // Get pool for THIS connection's tenant
    let pool = self.connection_manager
        .get_pool(&self.tenant_context.tenant_id)
        .await?;

    // Handler can only access THIS tenant's DB
    let ctx = HandlerContext::new(message_id, tenant_id, user_id, pool);
    handlers::dispatch(message, ctx).await?;
}
```

**Even if code has bugs, physical isolation prevents leaks:**
```rust
// Hypothetically buggy code:
let malicious_tenant = "evil-corp";  // Attacker controls this

// This would fail:
sqlx::query!("SELECT * FROM pm_work_items")
    .fetch_all(&pool)  // pool is bound to attacker's tenant only!
    .await?;

// Cannot query other tenant (different SQLite file)
```

### Layer 4: Project-Level Authorization

**Every handler checks membership:**
```rust
pub async fn handle_create(request: CreateWorkItemRequest, ctx: HandlerContext) -> Result<...> {
    let project_id = parse_uuid(&request.project_id)?;

    // Check if user is member of this project
    check_permission(&ctx, project_id, Permission::Edit).await?;

    // ... rest of handler
}
```

**Membership check:**
```rust
pub async fn check_permission(
    ctx: &HandlerContext,
    project_id: Uuid,
    required: Permission,
) -> Result<()> {
    let member = ProjectMemberRepository::find_by_user_and_project(
        ctx.user_id,
        project_id
    ).await?;

    match member {
        None => Err(WsError::Unauthorized),
        Some(m) if !m.has_permission(required) => Err(WsError::Unauthorized),
        Some(_) => Ok(()),
    }
}
```

### Layer 5: Operation-Specific Validation

**View operations:**
```rust
check_permission(&ctx, project_id, Permission::View).await?;
```

**Edit operations:**
```rust
check_permission(&ctx, project_id, Permission::Edit).await?;
```

**Admin operations:**
```rust
check_permission(&ctx, project_id, Permission::Admin).await?;
```

**Why separate checks?**
- Principle of least privilege
- Viewers can read but not write
- Prevents accidental escalation

### Layer 6: Data Integrity

**Optimistic Locking:**
```rust
// Prevents concurrent update races
if work_item.version != request.expected_version {
    return Err(WsError::ConflictError {
        current_version: work_item.version,
    });
}
```

**Soft Deletes:**
```rust
// Prevents accidental data loss
work_item.deleted_at = Some(Utc::now());
// Can be undeleted by setting deleted_at = NULL
```

**Foreign Key Constraints:**
```sql
-- Prevents orphaned child items
FOREIGN KEY (parent_id) REFERENCES pm_work_items(id)
```

---

## Real-Time Collaboration

### The Challenge

**Scenario**: 5 developers on a team, all viewing same Kanban board

**Requirements:**
1. When Alice moves a card, Bob/Carol/Dave see it **instantly** (<100ms)
2. When Bob updates story points, everyone sees new value **immediately**
3. When Carol deletes a task, it disappears for everyone **at the same time**
4. No polling (wastes bandwidth, adds latency)
5. Conflicts are detected and resolved

### Broadcast Architecture

```rust
pub struct TenantBroadcaster {
    channels: Arc<RwLock<HashMap<String, broadcast::Sender<BroadcastMessage>>>>,
}

impl TenantBroadcaster {
    // Connection subscribes to tenant's channel
    pub async fn subscribe(&self, tenant_id: &str) -> broadcast::Receiver<BroadcastMessage> {
        let channels = self.channels.read().await;
        channels.get(tenant_id)
            .expect("Tenant channel not initialized")
            .subscribe()
    }

    // Handler broadcasts to all connections for this tenant
    pub async fn broadcast(&self, tenant_id: &str, message: BroadcastMessage) {
        let channels = self.channels.read().await;
        if let Some(tx) = channels.get(tenant_id) {
            let _ = tx.send(message);  // Ignore if no receivers
        }
    }
}
```

**How it works:**

1. **Connection established**: Client subscribes to tenant's broadcast channel
2. **Handler completes operation**: Sends event to broadcast channel
3. **All subscribers receive**: Tokio fan-out happens in O(1) time
4. **Per-connection filtering**: Each connection filters by project subscription

### Event Flow Example

**Alice moves "Bug #123" from "In Progress" to "Done":**

```
Alice's Browser
    │
    │ UpdateWorkItemRequest { id: 123, status: "done", version: 5 }
    ↓
Server (Handler)
    │
    ├─ Validate request ✓
    ├─ Check permission ✓
    ├─ Load work item from DB
    ├─ Check version (5 == 5) ✓
    ├─ Update: status = "done", version = 6
    ├─ Save to DB ✓
    ├─ Build response
    │
    ├─→ Send to Alice: WorkItemUpdated { id: 123, status: "done", version: 6 }
    │
    └─→ Broadcast to tenant: WorkItemUpdated { ... }
            │
            ├─→ Bob's connection (subscribed to project)
            │       → UI updates (card moves to "Done" column)
            │
            ├─→ Carol's connection (subscribed to project)
            │       → UI updates
            │
            ├─→ Dave's connection (NOT subscribed to project)
            │       → Filtered out, not sent
            │
            └─→ Eve's connection (different tenant)
                    → Different broadcast channel, impossible to receive
```

**Timeline:**
- T+0ms: Alice clicks "Done"
- T+5ms: Server receives message
- T+10ms: DB updated
- T+15ms: Alice gets response
- T+20ms: Bob/Carol see update
- **Total latency: 20ms**

### Subscription Filtering

**Problem**: User might have access to 50 projects, but only viewing 3

**Solution**: Client tells server which projects to send updates for

```protobuf
// Client sends on page load:
Subscribe {
  project_ids: ["proj-1", "proj-2", "proj-3"]
}
```

**Server-side filtering:**
```rust
async fn handle_broadcast(&self, event: WorkItemUpdated) {
    // Only send if client subscribed to this project
    if self.subscriptions.contains(&event.work_item.project_id) {
        self.send(event).await?;
    }
}
```

**Benefits:**
- Bandwidth: Only relevant updates sent
- UI consistency: Matches what user is viewing
- Client control: Easy to add/remove subscriptions

### Conflict Resolution

**Scenario**: Alice and Bob both edit work item #123

```
T+0: Alice fetches item (version 5)
T+1: Bob fetches item (version 5)
T+2: Alice updates title → server increments version to 6
T+3: Bob updates status → server detects conflict!
```

**Server logic:**
```rust
// Bob's request
UpdateWorkItemRequest {
    work_item_id: "123",
    expected_version: 5,  // Bob thinks it's still version 5
    status: Some("done"),
}

// Server checks
let current = repo.find_by_id("123").await?;
if current.version != request.expected_version {
    return Err(WsError::ConflictError {
        current_version: 6,  // It's actually version 6 now!
    });
}
```

**Client handling:**
```csharp
try {
    await _service.UpdateWorkItemAsync(id, changes, expectedVersion: 5);
}
catch (ConflictException ex) {
    // Fetch latest version
    var latest = await _service.GetWorkItemAsync(id);

    // Show merge dialog
    var result = await ShowConflictDialog(
        yourChanges: changes,
        theirChanges: latest,
        currentVersion: ex.CurrentVersion
    );

    // Retry with new version
    if (result.Merge) {
        await _service.UpdateWorkItemAsync(id, result.MergedChanges, ex.CurrentVersion);
    }
}
```

---

## Plugin Architecture

### The Goal

**Parent SaaS platform wants to add project management WITHOUT:**
- Rebuilding their entire UI
- Managing separate deployments
- Complex integration code
- Maintaining two authentication systems

**Our solution: Blazor RCL (Razor Class Library) as plugin**

### How It Works

**1. Parent Platform (Minimal Integration):**

```csharp
// Program.cs
builder.Services.AddProjectManagement(options => {
    options.WebSocketUrl = "wss://pm-backend.company.com";
    options.AuthTokenProvider = () => GetCurrentUserJwt();
});

// _Imports.razor
@using ProjectManagement.Components

// ProjectManagementPage.razor
<ProjectManagement.Components.KanbanBoard ProjectId="@projectId" />
```

**That's it. 5 lines of code.**

### What the Plugin Provides

**Complete UI components:**
- Kanban board (drag-and-drop)
- Sprint planner (backlog refinement)
- Comment threads (rich text)
- Time tracking (running timers)
- Dependency graph (visualization)

**Complete business logic:**
- WebSocket client (connection/reconnection)
- State management (optimistic updates)
- Offline support (IndexedDB cache)
- Conflict resolution (merge dialogs)

**Complete styling:**
- Radzen theme (professional look)
- Responsive design (mobile-friendly)
- Dark mode support
- Customizable CSS variables

### Plugin Architecture Benefits

**For Parent Platform:**
- ✅ Drop-in component (no UI work)
- ✅ Automatic updates (NuGet package)
- ✅ No backend changes needed
- ✅ Single authentication (JWT passthrough)

**For End Users:**
- ✅ Consistent UI (looks native)
- ✅ Fast (runs in browser)
- ✅ Offline capable (WebAssembly)
- ✅ Real-time (WebSocket updates)

**For Us:**
- ✅ Portable (any .NET app can use)
- ✅ Testable (standalone WASM host)
- ✅ Maintainable (one codebase)
- ✅ Monetizable (NuGet package sales)

### Table Injection Pattern

**Instead of separate plugin database:**

```
┌─────────────────────────────────────────────┐
│       Tenant's Main Database (main.db)      │
│                                             │
│  Platform Tables:                          │
│  ├─ users                                  │
│  ├─ subscriptions                          │
│  └─ audit_log                              │
│                                             │
│  Plugin Tables (injected):                 │
│  ├─ pm_work_items         ← Our plugin    │
│  ├─ pm_sprints            ← Our plugin    │
│  ├─ pm_comments           ← Our plugin    │
│  └─ pm_project_members    ← Our plugin    │
│                                             │
│  Other Plugin Tables:                      │
│  ├─ wiki_pages            ← Wiki plugin   │
│  └─ forum_posts           ← Forum plugin  │
└─────────────────────────────────────────────┘
```

**Benefits:**
- Single backup (one file = platform + all plugins)
- Cross-plugin queries (JOIN between wiki_pages and pm_work_items)
- LLM context (AI sees everything in one place)
- Foreign keys work (pm_work_items.created_by → users.id)

---

## Data Flow Examples

### Example 1: Creating a Work Item

**User Story**: Developer creates a new task "Fix login bug"

```
┌─────────────────────────────────────────────────────────────────┐
│                         Browser (Blazor)                        │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  1. User clicks "Add Task" button                             │
│     ↓                                                           │
│  2. Component captures form input                             │
│     ↓                                                           │
│  3. Generate message ID (UUID)                                │
│     messageId = "550e8400-e29b-41d4-a716-446655440000"       │
│     ↓                                                           │
│  4. Build protobuf request:                                   │
│     CreateWorkItemRequest {                                   │
│       item_type: TASK,                                        │
│       title: "Fix login bug",                                │
│       project_id: "proj-123",                                │
│       parent_id: "story-456"                                 │
│     }                                                           │
│     ↓                                                           │
│  5. Optimistic update (instant UI feedback)                  │
│     workItemStore.addPendingItem(tempItem)                   │
│     UI shows new card immediately (grayed out)               │
│     ↓                                                           │
│  6. Send WebSocket message                                    │
│     webSocket.send(protobuf.encode(message))                 │
│                                                                 │
└─────────────────────────┬───────────────────────────────────────┘
                          │ Binary protobuf over WebSocket
┌─────────────────────────▼───────────────────────────────────────┐
│                    Rust Backend (pm-ws)                         │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  7. WebSocketConnection receives binary data                  │
│     ↓                                                           │
│  8. Decode protobuf → WebSocketMessage                        │
│     ↓                                                           │
│  9. Extract tenant_id from connection (from JWT)              │
│     tenant_id = "acme-corp"                                   │
│     ↓                                                           │
│  10. Get database pool                                         │
│      pool = connection_manager.get_pool("acme-corp")          │
│      ↓                                                           │
│  11. Create handler context                                    │
│      ctx = HandlerContext {                                   │
│        message_id,                                            │
│        tenant_id,                                             │
│        user_id,                                               │
│        pool                                                   │
│      }                                                           │
│      ↓                                                           │
│  12. Dispatch to handler                                       │
│      handlers::work_item::handle_create(request, ctx)         │
│                                                                 │
└─────────────────────────┬───────────────────────────────────────┘
                          │
┌─────────────────────────▼───────────────────────────────────────┐
│                  Handler (pm-ws/handlers)                       │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  13. Check idempotency                                         │
│      if cached_response = idempotency_repo.find(message_id) { │
│        return cached_response  // Duplicate request           │
│      }                                                           │
│      ↓                                                           │
│  14. Validate input                                            │
│      if title.is_empty() {                                    │
│        return ValidationError                                 │
│      }                                                           │
│      ↓                                                           │
│  15. Check authorization                                       │
│      member = project_member_repo.find(user_id, project_id)  │
│      if !member.has_permission(Edit) {                        │
│        return Unauthorized                                    │
│      }                                                           │
│      ↓                                                           │
│  16. Validate hierarchy                                        │
│      parent = work_item_repo.find(parent_id)                 │
│      if parent.item_type != STORY {                           │
│        return ValidationError  // Task must be under Story   │
│      }                                                           │
│      ↓                                                           │
│  17. Create domain model                                       │
│      work_item = WorkItem::new(                               │
│        item_type: TASK,                                       │
│        title: "Fix login bug",                               │
│        parent_id: story-456,                                 │
│        project_id: proj-123,                                 │
│        created_by: user_id                                    │
│      )                                                           │
│                                                                 │
└─────────────────────────┬───────────────────────────────────────┘
                          │
┌─────────────────────────▼───────────────────────────────────────┐
│                 Database Transaction (SQLite)                   │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  18. Begin transaction                                         │
│      tx = pool.begin()                                         │
│      ↓                                                           │
│  19. Insert work item                                          │
│      INSERT INTO pm_work_items (                              │
│        id, item_type, title, parent_id, project_id,          │
│        position, status, version, created_at, created_by     │
│      ) VALUES (...)                                            │
│      ↓                                                           │
│  20. Insert activity log (audit trail)                        │
│      INSERT INTO pm_activity_log (                            │
│        entity_type: 'WorkItem',                               │
│        entity_id: work_item.id,                               │
│        action: 'created',                                     │
│        user_id, timestamp                                     │
│      ) VALUES (...)                                            │
│      ↓                                                           │
│  21. Commit transaction                                        │
│      tx.commit()  // Both succeed or both fail               │
│                                                                 │
└─────────────────────────┬───────────────────────────────────────┘
                          │
┌─────────────────────────▼───────────────────────────────────────┐
│                  Handler (continued)                            │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  22. Build response                                            │
│      response = WorkItemCreated {                             │
│        message_id: "550e8400...",                            │
│        work_item: {                                           │
│          id, title, status, version: 0, ...                  │
│        },                                                       │
│        user_id                                                │
│      }                                                           │
│      ↓                                                           │
│  23. Store idempotency key                                     │
│      idempotency_repo.create(                                 │
│        message_id,                                            │
│        operation: "create_work_item",                        │
│        result_bytes: response.encode()                       │
│      )                                                           │
│      ↓                                                           │
│  24. Return response + broadcast info                         │
│      Ok((response, BroadcastInfo {                            │
│        project_id: proj-123,                                 │
│        event_type: "work_item_created"                       │
│      }))                                                        │
│                                                                 │
└─────────────────────────┬───────────────────────────────────────┘
                          │
┌─────────────────────────▼───────────────────────────────────────┐
│              WebSocketConnection (continued)                    │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  25. Send response to requester (Developer)                   │
│      self.send(response)                                       │
│      ↓                                                           │
│  26. Broadcast to other subscribers                           │
│      tenant_broadcaster.broadcast(                             │
│        tenant_id: "acme-corp",                                │
│        message: response                                      │
│      )                                                           │
│      ↓                                                           │
│  27. All connections for this tenant receive event            │
│      ├─→ PM's connection (subscribed to proj-123) ✓          │
│      ├─→ QA's connection (subscribed to proj-123) ✓          │
│      └─→ Designer's connection (NOT subscribed) ✗ filtered   │
│                                                                 │
└─────────────────────────┬───────────────────────────────────────┘
                          │ Binary protobuf responses
┌─────────────────────────▼───────────────────────────────────────┐
│                    Browser (continued)                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  28. Receive response (Developer who created it)              │
│      message_id matches pending request                       │
│      ↓                                                           │
│  29. Replace optimistic item with real item                   │
│      workItemStore.confirmPendingItem(tempId, realItem)       │
│      UI updates card (no longer grayed out)                   │
│      ↓                                                           │
│  30. Other team members receive broadcast                     │
│      PM's browser: New card appears on their Kanban board     │
│      QA's browser: New task appears in their task list        │
│                                                                 │
│  TOTAL LATENCY: ~50ms from click to all UIs updated           │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

**Key Points:**
- **Optimistic UI**: User sees change instantly (step 5)
- **Idempotency**: Network retry safe (step 13)
- **Authorization**: Multi-layer security (steps 9, 15)
- **Transaction**: Atomic operation (steps 18-21)
- **Audit Trail**: Complete history (step 20)
- **Real-time**: All team members updated (steps 26-27)

---

## Deployment Model

### Production Deployment

```
                           ┌─────────────────┐
                           │   Load Balancer │
                           │    (SSL/TLS)    │
                           └────────┬────────┘
                                    │
                    ┌───────────────┼───────────────┐
                    │               │               │
           ┌────────▼────────┐ ┌───▼────────┐ ┌───▼────────┐
           │  Backend        │ │  Backend   │ │  Backend   │
           │  Instance 1     │ │  Instance  │ │  Instance  │
           │  (Rust binary)  │ │     2      │ │     3      │
           └────────┬────────┘ └───┬────────┘ └───┬────────┘
                    │               │               │
                    └───────────────┼───────────────┘
                                    │
                           ┌────────▼────────┐
                           │  Shared Storage │
                           │  /data/tenants/ │
                           │   (NFS/S3)      │
                           └─────────────────┘
```

**Scaling Considerations:**

1. **WebSocket Sticky Sessions**: Load balancer must route tenant to same instance
2. **Shared Storage**: All instances access same tenant SQLite files (NFS)
3. **Broadcast Coordination**: Use Redis pub/sub for cross-instance broadcasts

### Container Deployment

```dockerfile
# Dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release --bin pm-server

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates
COPY --from=builder /app/target/release/pm-server /usr/local/bin/
CMD ["pm-server"]
```

```yaml
# docker-compose.yml
version: '3.8'
services:
  backend:
    image: pm-backend:latest
    ports:
      - "8080:8080"
    volumes:
      - tenant-data:/data/tenants
    environment:
      - RUST_LOG=info
      - JWT_PUBLIC_KEY_PATH=/keys/public.pem
    deploy:
      replicas: 3

volumes:
  tenant-data:
```

### Resource Requirements

**Per Instance:**
- CPU: 2-4 cores (async runtime scales)
- RAM: 2-4 GB (mostly connection state)
- Disk: Negligible (tenant DBs on shared storage)
- Network: 100+ Mbps (WebSocket traffic)

**Per Tenant:**
- Database: 10-100 MB (depends on activity)
- Memory: 5-10 MB (connection pool)
- Connections: 1-100 (typically <10)

**Capacity Planning:**
- 1 instance: 1000+ concurrent connections
- 3 instances: 3000+ connections (N-1 redundancy)
- Storage: 100 tenants × 50 MB = 5 GB

---

## Summary

This JIRA clone demonstrates production-grade patterns for real-time collaborative SaaS:

**Technology Choices:**
- Blazor WASM (type-safe UI, offline-first)
- Rust backend (performance, safety)
- SQLite per-tenant (isolation, LLM-friendly)
- Protobuf + WebSocket (real-time, efficient)

**Key Patterns:**
- Optimistic locking (prevent data loss)
- Idempotency (safe retries)
- Explicit subscriptions (bandwidth efficiency)
- Plugin architecture (drop-in integration)
- Defense-in-depth security (multi-layer)

**Production Features:**
- Multi-tenancy (physical isolation)
- Real-time collaboration (<100ms latency)
- Offline support (service workers)
- Complete audit trail (compliance)
- LLM-queryable (AI integration)

**For Future Sessions:**
- Reference specific sections when implementing features
- Understand WHY before implementing HOW
- Patterns are transferable to other domains

---

**Document Version**: 1.0
**Last Updated**: 2026-01-16
**Maintainers**: Claude (Teaching Mode) + Tony
**Status**: Living document (update as architecture evolves)
