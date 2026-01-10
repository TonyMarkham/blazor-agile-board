# Implementation Plan

This document breaks down the implementation into logical, sequential sessions that fit within 100k token budgets.

## Complexity Analysis

### High Complexity (Need careful planning)
1. **WebSocket + Protobuf integration** - Bidirectional communication, message handling, reconnection logic
2. **Per-tenant connection management** - Dynamic pool creation, caching, cleanup
3. **Real-time state synchronization** - WebSocket events → local state updates
4. **Activity logging system** - Automatic tracking of all changes
5. **Dependency graph validation** - Prevent circular dependencies

### Medium Complexity
1. **SQLx migrations and schema** - Straightforward but detailed
2. **REST API CRUD operations** - Standard patterns, repetitive
3. **JWT authentication middleware** - Well-established pattern
4. **Blazor component hierarchy** - Standard Razor components
5. **Protobuf code generation** - Build-time tooling

### Low Complexity (Foundation work)
1. **Project scaffolding** - Create directory structure, Cargo.toml, .csproj files
2. **Core domain models** - Simple structs/classes
3. **Basic DTOs** - Request/response types
4. **Configuration management** - Environment variables
5. **Error types** - Standard error handling

---

## Implementation Order Strategy

### Principle: Bottom-Up with Vertical Slices

1. **Foundation first** - Core types, database, basic API
2. **Vertical slices** - Complete one feature end-to-end before moving to next
3. **Real-time last** - Get REST API working, then add WebSocket
4. **Test as we go** - Unit tests for each component

### Feature Prioritization

**Phase 1: Foundation & Work Items** (Sessions 1-3)
- Most critical feature
- Establishes patterns for everything else
- Tests database, API, and frontend integration

**Phase 2: Sprints & Comments** (Session 4)
- Builds on work items
- Simpler than work items (no hierarchy)

**Phase 3: Time Tracking & Dependencies** (Session 5)
- Independent features
- Time tracking has running timer complexity
- Dependencies need cycle detection

**Phase 4: Real-time & Polish** (Sessions 6-7)
- WebSocket integration
- Activity logging
- LLM context seeding
- End-to-end testing

---

## Session Breakdown

### Session 1: Foundation & Database (Est. 80k tokens)

**Goal**: Working database with migrations, basic Rust project structure

**Deliverables**:
- ✅ Rust workspace with all crates scaffolded
- ✅ SQLx migrations for all tables
- ✅ Per-tenant connection manager
- ✅ Core domain models (Rust)
- ✅ Repository pattern for work_items
- ✅ Basic error types
- ✅ Integration test: Create tenant DB, run migrations

**Files Created** (~30 files):
```
backend/
├── Cargo.toml (workspace)
├── crates/pm-core/ (lib + models + errors)
├── crates/pm-db/ (migrations + connection + repositories)
├── crates/pm-auth/ (jwt + middleware - stubs)
├── crates/pm-proto/ (protobuf - minimal for now)
└── pm-server/ (main.rs - minimal)
```

**Key Code**:
- `TenantConnectionManager` - Dynamic SQLite pool management
- `WorkItemRepository` - CRUD with SQLx
- All 8 migration files
- Core `WorkItem`, `Sprint`, etc. models

**Testing**:
```rust
#[tokio::test]
async fn test_tenant_connection_manager() {
    let manager = TenantConnectionManager::new("./test_data");
    let pool1 = manager.get_connection("tenant-1").await.unwrap();
    let pool2 = manager.get_connection("tenant-1").await.unwrap();
    // Should return same pool
}

#[tokio::test]
async fn test_work_item_repository() {
    let repo = WorkItemRepository::new(pool);
    let item = repo.create(work_item).await.unwrap();
    let found = repo.find_by_id(item.id).await.unwrap();
    assert_eq!(found.title, "Test Task");
}
```

**Risks**:
- SQLx compile-time verification might fail if schema doesn't match queries
- Need `sqlx-cli` installed: `cargo install sqlx-cli`

**Success Criteria**:
- `cargo test --workspace` passes
- Can create tenant database and insert work items
- Migrations run successfully

---

### Session 2: REST API for Work Items (Est. 90k tokens)

**Goal**: Complete CRUD API for work items with auth

**Deliverables**:
- ✅ JWT authentication middleware
- ✅ Tenant context extraction
- ✅ REST API routes for work items
- ✅ DTOs (Create, Update, Response)
- ✅ Request validation
- ✅ Error handling (ApiError)
- ✅ Axum server setup
- ✅ Integration tests for API endpoints

**Files Created** (~25 files):
```
backend/
├── crates/pm-auth/ (complete JWT validation)
├── crates/pm-api/
│   ├── routes/work_items.rs
│   ├── handlers/work_items.rs
│   ├── dto/work_item.rs
│   └── validators/
└── pm-server/ (complete server setup)
```

**API Endpoints**:
```
POST   /api/v1/work-items          - Create work item
GET    /api/v1/work-items/:id      - Get work item
PUT    /api/v1/work-items/:id      - Update work item
DELETE /api/v1/work-items/:id      - Delete (soft)
GET    /api/v1/projects/:id/items  - List all in project
POST   /api/v1/work-items/:id/move - Move item (parent/position)
```

**Key Code**:
```rust
// Auth middleware
async fn auth_middleware(
    State(validator): State<Arc<JwtValidator>>,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let token = extract_bearer_token(&req)?;
    let claims = validator.validate(token)?;
    req.extensions_mut().insert(claims.tenant_id);
    req.extensions_mut().insert(claims.user_id);
    Ok(next.run(req).await)
}

// Handler example
async fn create_work_item(
    State(state): State<AppState>,
    Extension(tenant_id): Extension<String>,
    Extension(user_id): Extension<String>,
    Json(payload): Json<CreateWorkItemDto>,
) -> Result<Json<WorkItemDto>, ApiError> {
    // Validate
    payload.validate()?;

    // Get connection
    let pool = state.conn_manager.get_connection(&tenant_id).await?;
    let repo = WorkItemRepository::new(pool);

    // Create
    let item = WorkItem::new(payload, user_id);
    repo.create(&item).await?;

    // TODO: Log activity, broadcast WebSocket (Session 6)

    Ok(Json(item.into()))
}
```

**Testing**:
```rust
#[tokio::test]
async fn test_create_work_item_requires_auth() {
    let app = test_app().await;
    let response = app.post("/api/v1/work-items")
        .json(&create_dto)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_create_work_item_success() {
    let response = authenticated_client()
        .post("/api/v1/work-items")
        .json(&create_dto)
        .send()
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let body: WorkItemDto = response.json().await;
    assert_eq!(body.title, "Test Task");
}
```

**Success Criteria**:
- Server starts and responds to health check
- All work item CRUD endpoints functional
- Auth middleware blocks unauthorized requests
- Tenant isolation verified (can't access other tenant's data)

---

### Session 3: Blazor Frontend - Work Items (Est. 95k tokens)

**Goal**: Working frontend that displays and creates work items

**Deliverables**:
- ✅ Blazor project structure (.sln, 4 projects)
- ✅ Core models (C#)
- ✅ HTTP API client for work items
- ✅ Basic state management
- ✅ Radzen setup
- ✅ Project dashboard page
- ✅ Work item list component
- ✅ Create work item dialog
- ✅ Work item detail view

**Files Created** (~35 files):
```
frontend/
├── ProjectManagement.sln
├── ProjectManagement.Core/ (models, interfaces, DTOs)
├── ProjectManagement.Services/ (API clients, state)
├── ProjectManagement.Components/ (Razor components)
└── ProjectManagement.Wasm/ (host)
```

**Key Components**:
```razor
<!-- Pages/ProjectDashboard.razor -->
@page "/projects/{ProjectId:guid}"
@inject WorkItemApiClient ApiClient
@inject ProjectState State

<PageTitle>Project Dashboard</PageTitle>

<RadzenStack>
    <RadzenRow>
        <RadzenColumn Size="12">
            <RadzenButton Text="New Task" Click="OpenCreateDialog" />
        </RadzenColumn>
    </RadzenRow>

    <RadzenRow>
        <RadzenColumn Size="12">
            <WorkItemList Items="@_items" OnItemClick="ShowDetails" />
        </RadzenColumn>
    </RadzenRow>
</RadzenStack>

@code {
    [Parameter] public Guid ProjectId { get; set; }
    private IEnumerable<WorkItem> _items = Array.Empty<WorkItem>();

    protected override async Task OnInitializedAsync() {
        _items = await ApiClient.GetByProjectAsync(ProjectId);
    }
}
```

**API Client**:
```csharp
public class WorkItemApiClient
{
    private readonly HttpClient _http;

    public async Task<IEnumerable<WorkItemDto>> GetByProjectAsync(Guid projectId)
    {
        return await _http.GetFromJsonAsync<IEnumerable<WorkItemDto>>(
            $"/api/v1/projects/{projectId}/items"
        ) ?? Enumerable.Empty<WorkItemDto>();
    }

    public async Task<WorkItemDto> CreateAsync(CreateWorkItemDto dto)
    {
        var response = await _http.PostAsJsonAsync("/api/v1/work-items", dto);
        response.EnsureSuccessStatusCode();
        return await response.Content.ReadFromJsonAsync<WorkItemDto>()
            ?? throw new Exception("Failed to create");
    }
}
```

**Testing**:
- Manual testing in browser
- bUnit tests for components (if time permits)

**Success Criteria**:
- Can navigate to project dashboard
- Work items display in Radzen DataGrid
- Can create new work item via dialog
- Can view work item details
- Frontend successfully calls backend API

---

### Session 4: Sprints & Comments (Est. 85k tokens)

**Goal**: Complete sprint management and commenting system

**Deliverables**:

**Backend**:
- ✅ Sprint repository & API endpoints
- ✅ Comment repository & API endpoints
- ✅ Sprint assignment validation (only stories/tasks)

**Frontend**:
- ✅ Sprint list page
- ✅ Sprint board (Kanban view with swim lanes)
- ✅ Sprint planning (drag & drop)
- ✅ Comment component for work items
- ✅ Comment list & add comment

**API Endpoints**:
```
# Sprints
POST   /api/v1/sprints
GET    /api/v1/sprints/:id
PUT    /api/v1/sprints/:id
DELETE /api/v1/sprints/:id
POST   /api/v1/sprints/:id/start
POST   /api/v1/sprints/:id/complete
GET    /api/v1/projects/:id/sprints

# Comments
POST   /api/v1/work-items/:id/comments
GET    /api/v1/work-items/:id/comments
PUT    /api/v1/comments/:id
DELETE /api/v1/comments/:id
```

**Key Component**:
```razor
<!-- Components/SprintBoard.razor -->
<RadzenRow>
    @foreach (var lane in _lanes)
    {
        <RadzenColumn Size="3">
            <RadzenCard>
                <RadzenText TextStyle="TextStyle.H6">@lane.Name</RadzenText>
                @foreach (var item in GetItemsInLane(lane.Status))
                {
                    <WorkItemCard Item="@item"
                                  OnDrop="(target) => HandleDrop(item, target)" />
                }
            </RadzenCard>
        </RadzenColumn>
    }
</RadzenRow>
```

**Success Criteria**:
- Can create and manage sprints
- Can assign tasks to sprints
- Validation prevents assigning epics to sprints
- Can view sprint board with swim lanes
- Can add/edit/delete comments on work items

---

### Session 5: Time Tracking & Dependencies (Est. 80k tokens)

**Goal**: Time tracking with running timers and dependency management

**Deliverables**:

**Backend**:
- ✅ Time entry repository & API
- ✅ Timer start/stop logic
- ✅ Running timer queries (WHERE ended_at IS NULL)
- ✅ Dependency repository & API
- ✅ Circular dependency detection

**Frontend**:
- ✅ Time tracking component
- ✅ Timer controls (start/stop)
- ✅ Time entry list
- ✅ Manual time entry form
- ✅ Dependency management UI
- ✅ Blocked task indicators

**API Endpoints**:
```
# Time Tracking
POST   /api/v1/work-items/:id/time-entries/start
POST   /api/v1/work-items/:id/time-entries/stop
GET    /api/v1/work-items/:id/time-entries
POST   /api/v1/time-entries          # Manual entry
PUT    /api/v1/time-entries/:id
DELETE /api/v1/time-entries/:id
GET    /api/v1/users/:id/active-timer

# Dependencies
POST   /api/v1/dependencies
DELETE /api/v1/dependencies/:id
GET    /api/v1/work-items/:id/dependencies
GET    /api/v1/work-items/:id/blocked-by
```

**Circular Dependency Detection**:
```rust
fn would_create_cycle(
    blocking_id: Uuid,
    blocked_id: Uuid,
    repo: &DependencyRepository
) -> Result<bool> {
    // Use recursive CTE to detect cycles
    let query = r#"
        WITH RECURSIVE dep_chain AS (
            SELECT blocked_item_id FROM pm_dependencies
            WHERE blocking_item_id = ?
            UNION ALL
            SELECT d.blocked_item_id FROM pm_dependencies d
            JOIN dep_chain dc ON d.blocking_item_id = dc.blocked_item_id
        )
        SELECT COUNT(*) FROM dep_chain WHERE blocked_item_id = ?
    "#;

    let count: i64 = sqlx::query_scalar(query)
        .bind(blocked_id)
        .bind(blocking_id)
        .fetch_one(&repo.pool)
        .await?;

    Ok(count > 0)
}
```

**Success Criteria**:
- Can start/stop timer on tasks
- Timer persists across page refreshes
- Can add manual time entries
- Can create/delete dependencies
- System prevents circular dependencies
- UI shows blocked task indicators

---

### Session 6: WebSocket & Real-time Updates (Est. 95k tokens)

**Goal**: Real-time collaboration via WebSocket + Protobuf

**Deliverables**:

**Backend**:
- ✅ Complete protobuf message handling
- ✅ WebSocket connection handler
- ✅ Per-tenant broadcast channels
- ✅ Subscription management
- ✅ Heartbeat (ping/pong)
- ✅ Broadcast on all API mutations

**Frontend**:
- ✅ WebSocket client
- ✅ Protobuf encoding/decoding
- ✅ State synchronization from WebSocket
- ✅ Reconnection logic
- ✅ Optimistic updates
- ✅ Connection status indicator

**Key Backend Code**:
```rust
// WebSocket handler
async fn handle_websocket(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Extension(tenant_id): Extension<String>,
    Extension(user_id): Extension<String>,
) -> Response {
    ws.on_upgrade(|socket| handle_socket(socket, state, tenant_id, user_id))
}

async fn handle_socket(
    socket: WebSocket,
    state: AppState,
    tenant_id: String,
    user_id: String,
) {
    let (sender, receiver) = socket.split();
    let broadcast_rx = state.broadcaster.subscribe(&tenant_id);

    let conn = WebSocketConnection {
        user_id,
        tenant_id,
        subscriptions: Arc::new(RwLock::new(HashSet::new())),
        sender,
        receiver,
    };

    conn.handle(broadcast_rx).await;
}
```

**Key Frontend Code**:
```csharp
public class ProjectStateManager
{
    private readonly ProjectManagementWebSocketClient _wsClient;

    public async Task InitializeAsync(Guid projectId)
    {
        // Load initial data
        var items = await _apiClient.GetByProjectAsync(projectId);
        _state.SetWorkItems(items);

        // Connect WebSocket
        await _wsClient.ConnectAsync();
        await _wsClient.SubscribeAsync(new[] { projectId }, Array.Empty<Guid>());

        // Start listening
        _ = Task.Run(ListenForUpdates);
    }

    private async Task ListenForUpdates()
    {
        await foreach (var message in _wsClient.Messages)
        {
            switch (message.PayloadCase)
            {
                case PayloadOneofCase.WorkItemUpdated:
                    var item = Map(message.WorkItemUpdated.WorkItem);
                    _state.UpdateWorkItem(item);
                    break;
                // ... other cases
            }
        }
    }
}
```

**Success Criteria**:
- WebSocket connects on page load
- Updates from one client appear in real-time on other clients
- Ping/pong keeps connection alive
- Client reconnects after disconnect
- Subscription filtering works (don't receive other projects' updates)

---

### Session 7: Activity Logging, LLM Context & Polish (Est. 75k tokens)

**Goal**: Complete the system with audit trail and LLM integration

**Deliverables**:

**Backend**:
- ✅ Activity log repository
- ✅ Automatic activity logging on all mutations
- ✅ Activity log API endpoints
- ✅ LLM context seed data
- ✅ LLM context API endpoints
- ✅ Swim lanes seed data

**Frontend**:
- ✅ Activity history component
- ✅ User presence indicators (via WebSocket)
- ✅ Connection status indicator
- ✅ Error boundary
- ✅ Loading states
- ✅ Toast notifications

**Documentation**:
- ✅ README with setup instructions
- ✅ API documentation
- ✅ Deployment guide
- ✅ Development guide

**Activity Logging**:
```rust
// Middleware to automatically log all changes
async fn activity_logging_middleware(
    Extension(user_id): Extension<String>,
    req: Request,
    next: Next,
) -> Response {
    let method = req.method().clone();
    let uri = req.uri().clone();

    let response = next.run(req).await;

    // Parse response and log activity
    if method == Method::POST || method == Method::PUT || method == Method::DELETE {
        let activity = ActivityLog {
            id: Uuid::new_v4(),
            entity_type: parse_entity_type(&uri),
            entity_id: parse_entity_id(&uri),
            action: map_method_to_action(method),
            user_id,
            timestamp: Utc::now().timestamp(),
            // ...
        };

        activity_repo.create(&activity).await?;
    }

    response
}
```

**LLM Context Seed**:
```sql
INSERT INTO pm_llm_context (id, context_type, category, title, content, priority) VALUES
('ctx_001', 'schema_doc', 'work_items', 'Work Item Hierarchy', '...', 100),
('ctx_002', 'query_pattern', 'dependencies', 'Find Blocked Tasks', '...', 90),
('ctx_003', 'business_rule', 'sprints', 'Sprint Assignment Rules', '...', 80),
-- ... 20+ more entries covering all tables and patterns
```

**Success Criteria**:
- All mutations logged to activity_log table
- Can view activity history on work items
- LLM context table populated with helpful documentation
- Frontend polished with loading states and error handling
- System is production-ready

---

## Token Budget Estimates

| Session | Estimated Tokens | Content Type |
|---------|------------------|--------------|
| 1 | 80k | Rust scaffolding, migrations, repositories |
| 2 | 90k | REST API, handlers, middleware, tests |
| 3 | 95k | Blazor projects, components, API clients |
| 4 | 85k | Sprints + comments (backend + frontend) |
| 5 | 80k | Time tracking + dependencies |
| 6 | 95k | WebSocket integration (most complex) |
| 7 | 75k | Activity logging, polish, docs |
| **Total** | **600k** | ~7 sessions |

---

## Risk Mitigation

### High-Risk Areas

1. **WebSocket stability** (Session 6)
   - Mitigation: Extensive testing with multiple clients
   - Fallback: Polling if WebSocket fails

2. **Per-tenant connection pooling** (Session 1)
   - Mitigation: Start simple, optimize later
   - Fallback: Create new connection per request if pooling fails

3. **Circular dependency detection** (Session 5)
   - Mitigation: Thorough testing with various graph structures
   - Fallback: Limit dependency depth to 10 levels

4. **Protobuf code generation** (Multiple sessions)
   - Mitigation: Generate code once, commit to repo
   - Fallback: Manual protobuf classes if codegen fails

---

## Success Metrics

After all sessions complete:

- ✅ Can create projects, epics, stories, tasks with hierarchy
- ✅ Can create sprints and assign work items
- ✅ Can track time with running timers
- ✅ Can manage dependencies with cycle detection
- ✅ Real-time updates work across multiple browser tabs
- ✅ Complete audit trail in activity_log
- ✅ LLM can query database and understand schema
- ✅ All CRUD operations functional via REST API
- ✅ Professional UI with Radzen components
- ✅ Authentication and tenant isolation working
- ✅ Integration tests passing
- ✅ Documentation complete

---

## Development Environment Setup

Before Session 1:

**Rust**:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
cargo install sqlx-cli --no-default-features --features sqlite
```

**.NET**:
```bash
# Install .NET 8 SDK
# Install protoc: brew install protobuf (macOS)
```

**Tools**:
```bash
# VS Code extensions
- rust-analyzer
- C# Dev Kit
- Blazor WASM Debugging
```

---

## Post-Implementation (Future Sessions)

**v1.1 Features** (3-4 more sessions):
- Labels & priorities
- Custom swim lanes
- Advanced filters
- Burndown charts
- Export functionality

**v1.2 Features** (3-4 more sessions):
- File attachments
- Custom fields
- Notifications
- @mentions

**v2.0 Features** (5+ sessions):
- Semantic search with embeddings
- AI task creation
- Git integration
- Advanced reporting
