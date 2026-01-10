# Implementation Plan (Revised - WebSocket First)

This document breaks down the implementation into logical, sequential sessions that fit within 100k token budgets.

**Key Change**: WebSocket + Protobuf is our PRIMARY communication protocol, not REST. We'll build WebSocket early and use it for all operations.

---

## Architecture Decision: WebSocket-First

**Why WebSocket instead of REST?**
1. Real-time collaboration is core to the product
2. Single protocol for reads and writes (simpler)
3. Optimistic updates with server confirmation
4. Bidirectional = client can send commands, server broadcasts events
5. Production-grade from day one

**REST API Role**:
- Optional read-only endpoints for LLM integration
- Bulk data loading on initial page load
- Fallback if WebSocket unavailable

---

## Revised Session Breakdown

**Note**: Sessions numbered 10, 20, 30, etc. to leave room for incremental steps that will inevitably be needed along the way!

### Session 10: Foundation, Database & Protobuf (Est. 100k tokens)

**Goal**: Working database with migrations, repositories tested, and protobuf messages defined

**Deliverables**:
- ✅ Rust workspace with all crates scaffolded
- ✅ SQLx migrations for all tables (based on database-schema.md)
- ✅ Per-tenant connection manager
- ✅ Core domain models (Rust) matching database schema exactly
- ✅ Repository pattern for work_items, sprints, comments, time_entries, dependencies
- ✅ Basic error types
- ✅ Integration tests: CRUD operations work
- ✅ Protobuf messages matching database models
- ✅ Protobuf code generation setup (Rust)
- ✅ Protobuf serialization tests

**Files Created** (~30 files):
```
backend/
├── Cargo.toml (workspace)
├── crates/pm-core/ (lib + models + errors)
│   └── src/
│       ├── models/ (WorkItem, Sprint, Comment, TimeEntry, Dependency)
│       ├── errors.rs
│       └── lib.rs
├── crates/pm-db/
│   ├── migrations/ (all 8 migration SQL files)
│   ├── src/
│   │   ├── connection.rs (TenantConnectionManager)
│   │   ├── repositories/ (one per entity)
│   │   └── lib.rs
├── crates/pm-auth/ (stubs only for now)
└── pm-server/ (minimal main.rs)
```

**Key Code**:
- `TenantConnectionManager` - Dynamic SQLite pool management
- `WorkItemRepository` - Full CRUD with SQLx
- `SprintRepository`, `CommentRepository`, etc.
- All 8 migration files from docs/database-schema.md
- Core models match database columns exactly

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
async fn test_work_item_repository_crud() {
    let repo = WorkItemRepository::new(pool);
    let item = create_test_work_item();
    repo.create(&item).await.unwrap();

    let found = repo.find_by_id(item.id).await.unwrap().unwrap();
    assert_eq!(found.title, "Test Task");

    repo.delete(item.id).await.unwrap();
    assert!(repo.find_by_id(item.id).await.unwrap().is_none());
}
```

**Success Criteria**:
- `cargo build --workspace` compiles
- `cargo test --workspace` passes
- Can create tenant database and run all migrations
- Repository CRUD operations work for all entities
- Database schema matches docs/database-schema.md exactly

---

### Session 15: Protobuf Message Design (Est. 40k tokens)

**Goal**: Design protobuf messages that match the validated database schema

**Why separate session?**
- Database is now implemented and tested
- We know exactly what fields exist
- Protobuf messages can match database models 1:1
- Easier to iterate on message design

**Deliverables**:
- ✅ Review existing proto/messages.proto against actual database
- ✅ Update protobuf messages to match Rust models exactly
- ✅ Add any missing message types for CRUD operations
- ✅ Setup protobuf code generation for Rust
- ✅ Test protobuf encoding/decoding

**Files Created/Updated**:
```
proto/
└── messages.proto (update to match database)

backend/
├── build.rs (protobuf codegen)
└── crates/pm-proto/
    ├── Cargo.toml
    ├── build.rs
    └── src/
        ├── lib.rs
        └── generated/ (auto-generated)
```

**Key Work**:
- Ensure `WorkItem` protobuf message matches `pm_work_items` table
- Ensure `Sprint` protobuf message matches `pm_sprints` table
- Add request/response message types for WebSocket commands
- Add proper field numbering for backward compatibility
- Test serialization round-trips

**Success Criteria**:
- Protobuf compiles for Rust
- Can serialize/deserialize all entity types
- Field names match database columns
- All CRUD operations have corresponding message types

---

### Session 20: WebSocket Infrastructure (Est. 90k tokens)

**Goal**: Working WebSocket server with protobuf message handling

**Deliverables**:
- ✅ JWT authentication middleware
- ✅ Tenant context extraction
- ✅ WebSocket connection handler
- ✅ Protobuf message encoding/decoding
- ✅ Per-tenant broadcast channels
- ✅ Subscription management
- ✅ Heartbeat (ping/pong)
- ✅ Basic Axum server with /ws endpoint
- ✅ Connection tests

**Files Created** (~25 files):
```
backend/
├── crates/pm-auth/ (complete JWT validation)
├── crates/pm-ws/
│   ├── connection.rs      # WebSocket connection handling
│   ├── broadcast.rs       # Per-tenant broadcast channels
│   ├── subscription.rs    # Subscription filtering
│   ├── handlers.rs        # Message handler dispatch
│   └── messages/          # Individual message handlers
└── pm-server/ (complete server with WebSocket route)
```

**Key Code**:
```rust
// WebSocket upgrade handler
pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Extension(tenant_id): Extension<String>,
    Extension(user_id): Extension<String>,
) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state, tenant_id, user_id))
}

// Connection handler
async fn handle_socket(
    socket: WebSocket,
    state: AppState,
    tenant_id: String,
    user_id: String,
) {
    let (mut sender, mut receiver) = socket.split();

    // Subscribe to tenant's broadcast channel
    let mut broadcast_rx = state.broadcaster.subscribe(&tenant_id);

    // Client state
    let subscriptions = Arc::new(RwLock::new(HashSet::new()));

    // Spawn tasks for bidirectional communication
    let recv_task = tokio::spawn(handle_incoming(
        receiver,
        state.clone(),
        tenant_id.clone(),
        user_id.clone(),
        subscriptions.clone(),
    ));

    let send_task = tokio::spawn(handle_outgoing(
        sender,
        broadcast_rx,
        subscriptions.clone(),
    ));

    // Wait for either task to complete
    tokio::select! {
        _ = recv_task => {},
        _ = send_task => {},
    }
}

// Handle incoming messages from client
async fn handle_incoming(
    mut receiver: SplitStream<WebSocket>,
    state: AppState,
    tenant_id: String,
    user_id: String,
    subscriptions: Arc<RwLock<HashSet<String>>>,
) {
    while let Some(Ok(msg)) = receiver.next().await {
        if let Ok(bytes) = msg.into_data() {
            match WebSocketMessage::decode(&bytes[..]) {
                Ok(ws_msg) => {
                    if let Err(e) = handle_message(
                        ws_msg,
                        &state,
                        &tenant_id,
                        &user_id,
                        &subscriptions,
                    ).await {
                        eprintln!("Error handling message: {}", e);
                    }
                }
                Err(e) => eprintln!("Failed to decode protobuf: {}", e),
            }
        }
    }
}

// Handle outgoing broadcasts to client
async fn handle_outgoing(
    mut sender: SplitSink<WebSocket, Message>,
    mut broadcast_rx: broadcast::Receiver<WebSocketMessage>,
    subscriptions: Arc<RwLock<HashSet<String>>>,
) {
    while let Ok(msg) = broadcast_rx.recv().await {
        // Filter: only send if client subscribed
        if should_send_to_client(&msg, &subscriptions.read().await) {
            let bytes = msg.encode_to_vec();
            if let Err(e) = sender.send(Message::Binary(bytes)).await {
                eprintln!("Failed to send message: {}", e);
                break;
            }
        }
    }
}

// Message router
async fn handle_message(
    msg: WebSocketMessage,
    state: &AppState,
    tenant_id: &str,
    user_id: &str,
    subscriptions: &Arc<RwLock<HashSet<String>>>,
) -> Result<()> {
    match msg.payload {
        Some(Payload::Subscribe(sub)) => {
            handle_subscribe(sub, subscriptions).await?;
        }
        Some(Payload::Ping(ping)) => {
            send_pong(ping, state, user_id).await?;
        }
        // Work item operations handled in Session 3
        _ => {
            eprintln!("Unhandled message type");
        }
    }
    Ok(())
}
```

**Broadcast Channel**:
```rust
pub struct TenantBroadcaster {
    channels: Arc<RwLock<HashMap<String, broadcast::Sender<WebSocketMessage>>>>,
}

impl TenantBroadcaster {
    pub fn subscribe(&self, tenant_id: &str) -> broadcast::Receiver<WebSocketMessage> {
        let channels = self.channels.read().unwrap();
        if let Some(tx) = channels.get(tenant_id) {
            tx.subscribe()
        } else {
            drop(channels);
            let mut channels = self.channels.write().unwrap();
            let (tx, rx) = broadcast::channel(1000);
            channels.insert(tenant_id.to_string(), tx);
            rx
        }
    }

    pub fn broadcast(&self, tenant_id: &str, msg: WebSocketMessage) -> Result<()> {
        let channels = self.channels.read().unwrap();
        if let Some(tx) = channels.get(tenant_id) {
            tx.send(msg)?;
        }
        Ok(())
    }
}
```

**Testing**:
```rust
#[tokio::test]
async fn test_websocket_connection() {
    let app = test_app().await;
    let mut client = WebSocketClient::connect(&app, test_jwt()).await.unwrap();

    // Should receive connection success
    let msg = client.receive().await.unwrap();
    // ... assertions
}

#[tokio::test]
async fn test_subscribe_and_broadcast() {
    let (mut client1, mut client2) = connect_two_clients().await;

    // Both subscribe to same project
    client1.subscribe(vec!["project-1"]).await;
    client2.subscribe(vec!["project-1"]).await;

    // Broadcast a message
    broadcast_work_item_created("project-1", "task-1").await;

    // Both should receive
    assert!(client1.receive().await.is_ok());
    assert!(client2.receive().await.is_ok());
}
```

**Success Criteria**:
- WebSocket connection establishes with valid JWT
- Invalid JWT rejected with 401
- Ping/pong heartbeat works
- Subscribe message adds to subscriptions
- Broadcast sends to all subscribed clients
- Subscription filtering works (client 1 doesn't receive client 2's project updates)

---

### Session 30: Work Items via WebSocket (Est. 95k tokens)

**Goal**: Complete CRUD for work items using WebSocket commands

**Deliverables**:

**Backend**:
- ✅ Work item message handlers (Create, Update, Delete, Move)
- ✅ Business logic services (validation, hierarchy checks)
- ✅ Broadcast work item events to subscribed clients
- ✅ Error responses via WebSocket

**Frontend**:
- ✅ Blazor project structure (.sln, 4 projects)
- ✅ Core models (C#)
- ✅ Protobuf C# code generation
- ✅ WebSocket client
- ✅ State management
- ✅ Radzen setup
- ✅ Project dashboard page
- ✅ Work item list component
- ✅ Create work item dialog
- ✅ Work item detail view

**Backend Message Handlers**:
```rust
// handlers/work_items.rs
pub async fn handle_create_work_item(
    payload: CreateWorkItemRequest,
    state: &AppState,
    tenant_id: &str,
    user_id: &str,
) -> Result<()> {
    // 1. Validate
    validate_create_request(&payload)?;

    // 2. Get repository
    let pool = state.conn_manager.get_connection(tenant_id).await?;
    let repo = WorkItemRepository::new(pool);

    // 3. Create work item
    let work_item = WorkItem::new(
        payload.item_type,
        payload.title,
        payload.description,
        payload.parent_id,
        user_id,
    );

    repo.create(&work_item).await?;

    // 4. Broadcast event
    let event = WebSocketMessage {
        message_id: Uuid::new_v4().to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(Payload::WorkItemCreated(WorkItemCreated {
            work_item: Some(work_item.into()),
            user_id: user_id.to_string(),
        })),
    };

    state.broadcaster.broadcast(tenant_id, event)?;

    Ok(())
}

pub async fn handle_update_work_item(
    payload: UpdateWorkItemRequest,
    state: &AppState,
    tenant_id: &str,
    user_id: &str,
) -> Result<()> {
    let pool = state.conn_manager.get_connection(tenant_id).await?;
    let repo = WorkItemRepository::new(pool);

    // Get current state
    let current = repo.find_by_id(payload.work_item_id).await?
        .ok_or(Error::NotFound)?;

    // Track changes
    let mut changes = Vec::new();

    let updated = WorkItem {
        id: current.id,
        title: payload.title.unwrap_or(current.title),
        description: payload.description.or(current.description),
        status: payload.status.unwrap_or(current.status),
        // ... track each field change
        updated_at: Utc::now().timestamp(),
        updated_by: user_id.to_string(),
        ..current
    };

    repo.update(&updated).await?;

    // Broadcast
    let event = WebSocketMessage {
        message_id: Uuid::new_v4().to_string(),
        timestamp: Utc::now().timestamp(),
        payload: Some(Payload::WorkItemUpdated(WorkItemUpdated {
            work_item: Some(updated.into()),
            changes,
            user_id: user_id.to_string(),
        })),
    };

    state.broadcaster.broadcast(tenant_id, event)?;

    Ok(())
}
```

**Frontend WebSocket Client**:
```csharp
public class ProjectManagementWebSocketClient : IAsyncDisposable
{
    private ClientWebSocket _ws = new();
    private readonly string _wsUrl;
    private readonly string _jwtToken;
    private CancellationTokenSource _cts = new();

    private readonly Channel<WebSocketMessage> _outgoing = Channel.CreateUnbounded<WebSocketMessage>();
    private readonly Channel<WebSocketMessage> _incoming = Channel.CreateUnbounded<WebSocketMessage>();

    public IAsyncEnumerable<WebSocketMessage> Messages =>
        _incoming.Reader.ReadAllAsync();

    public async Task ConnectAsync()
    {
        _ws.Options.SetRequestHeader("Authorization", $"Bearer {_jwtToken}");
        await _ws.ConnectAsync(new Uri(_wsUrl), _cts.Token);

        _ = Task.Run(SendLoop);
        _ = Task.Run(ReceiveLoop);
    }

    private async Task SendLoop()
    {
        await foreach (var msg in _outgoing.Reader.ReadAllAsync(_cts.Token))
        {
            using var ms = new MemoryStream();
            msg.WriteTo(ms);
            var bytes = ms.ToArray();

            await _ws.SendAsync(
                new ArraySegment<byte>(bytes),
                WebSocketMessageType.Binary,
                true,
                _cts.Token
            );
        }
    }

    private async Task ReceiveLoop()
    {
        var buffer = new byte[8192];

        while (_ws.State == WebSocketState.Open)
        {
            var result = await _ws.ReceiveAsync(
                new ArraySegment<byte>(buffer),
                _cts.Token
            );

            if (result.MessageType == WebSocketMessageType.Close)
                break;

            var msg = WebSocketMessage.Parser.ParseFrom(buffer, 0, result.Count);
            await _incoming.Writer.WriteAsync(msg, _cts.Token);
        }
    }

    public async Task CreateWorkItemAsync(CreateWorkItemDto dto)
    {
        var msg = new WebSocketMessage
        {
            MessageId = Guid.NewGuid().ToString(),
            Timestamp = DateTimeOffset.UtcNow.ToUnixTimeSeconds(),
            CreateWorkItemRequest = new CreateWorkItemRequest
            {
                ItemType = (WorkItemType)dto.ItemType,
                Title = dto.Title,
                Description = dto.Description ?? "",
                ParentId = dto.ParentId?.ToString() ?? ""
            }
        };

        await _outgoing.Writer.WriteAsync(msg);
    }

    public async Task UpdateWorkItemAsync(Guid id, UpdateWorkItemDto dto)
    {
        var msg = new WebSocketMessage
        {
            MessageId = Guid.NewGuid().ToString(),
            Timestamp = DateTimeOffset.UtcNow.ToUnixTimeSeconds(),
            UpdateWorkItemRequest = new UpdateWorkItemRequest
            {
                WorkItemId = id.ToString(),
                Title = dto.Title,
                Status = dto.Status,
                // ...
            }
        };

        await _outgoing.Writer.WriteAsync(msg);
    }
}
```

**Frontend State Manager**:
```csharp
public class ProjectStateManager : IAsyncDisposable
{
    private readonly ProjectState _state;
    private readonly ProjectManagementWebSocketClient _wsClient;

    public async Task InitializeAsync(Guid projectId)
    {
        // TODO Session 4: Add REST endpoint for bulk initial load
        // For now, items come via WebSocket after subscribing

        await _wsClient.ConnectAsync();
        await _wsClient.SubscribeAsync(new[] { projectId }, Array.Empty<Guid>());

        _ = Task.Run(HandleMessages);
    }

    private async Task HandleMessages()
    {
        await foreach (var msg in _wsClient.Messages)
        {
            switch (msg.PayloadCase)
            {
                case PayloadOneofCase.WorkItemCreated:
                    var created = MapFromProto(msg.WorkItemCreated.WorkItem);
                    _state.AddWorkItem(created);
                    break;

                case PayloadOneofCase.WorkItemUpdated:
                    var updated = MapFromProto(msg.WorkItemUpdated.WorkItem);
                    _state.UpdateWorkItem(updated);
                    break;

                case PayloadOneofCase.WorkItemDeleted:
                    _state.RemoveWorkItem(Guid.Parse(msg.WorkItemDeleted.WorkItemId));
                    break;
            }
        }
    }
}
```

**Success Criteria**:
- Frontend connects to WebSocket successfully
- Can create work item via WebSocket command
- Created item appears in real-time on all connected clients
- Can update work item (title, status, etc.)
- Updates appear in real-time
- Can delete work item
- Deletion reflected in real-time
- Professional UI with Radzen components

---

### Session 40: Sprints & Comments via WebSocket (Est. 85k tokens)

**Goal**: Sprint management and commenting using WebSocket

**Backend**:
- ✅ Sprint message handlers (Create, Update, Delete, Start, Complete)
- ✅ Comment message handlers (Add, Update, Delete)
- ✅ Sprint assignment validation
- ✅ Broadcast sprint and comment events

**Frontend**:
- ✅ Sprint list & management
- ✅ Sprint board (Kanban)
- ✅ Sprint planning
- ✅ Comment component
- ✅ Real-time comment updates

**Success Criteria**:
- Can create and manage sprints via WebSocket
- Can assign tasks to sprints
- Sprint board displays with real-time updates
- Can add/edit/delete comments
- Comments appear in real-time for all viewers

---

### Session 50: Time Tracking & Dependencies via WebSocket (Est. 80k tokens)

**Goal**: Time tracking with timers and dependency management

**Backend**:
- ✅ Time entry handlers (Start, Stop, Create, Update, Delete)
- ✅ Running timer logic
- ✅ Dependency handlers (Create, Delete)
- ✅ Circular dependency detection
- ✅ Broadcast time and dependency events

**Frontend**:
- ✅ Timer component (start/stop)
- ✅ Time entry list
- ✅ Manual time entry
- ✅ Dependency management UI
- ✅ Blocked task indicators

**Success Criteria**:
- Can start/stop timers via WebSocket
- Timers sync across clients in real-time
- Can manage dependencies
- Circular dependency prevention works
- Blocked tasks show indicators

---

### Session 60: REST API for LLMs & Bulk Loading (Est. 70k tokens)

**Goal**: Read-only REST endpoints for LLM integration and efficient bulk loading

**Why REST Now?**
- WebSocket is great for real-time updates, not for bulk queries
- LLMs need simple HTTP endpoints
- Initial page load should fetch all data efficiently

**Backend**:
- ✅ Read-only REST endpoints
  - `GET /api/v1/projects/:id/context` - Full project data dump
  - `GET /api/v1/work-items/:id` - Single item detail
  - `GET /api/v1/activity` - Activity log queries
  - `GET /api/v1/llm/context` - Schema documentation
- ✅ Optimized bulk queries
- ✅ CORS configuration

**Frontend**:
- ✅ HTTP client for initial data load
- ✅ Use REST for page load, WebSocket for updates

**Success Criteria**:
- LLMs can query project context via REST
- Initial page load uses REST for efficiency
- WebSocket handles all mutations and real-time updates

---

### Session 70: Activity Logging, Polish & Documentation (Est. 75k tokens)

**Goal**: Complete production system

**Backend**:
- ✅ Activity log on all mutations
- ✅ LLM context seed data
- ✅ Swim lanes seed data
- ✅ Error handling improvements

**Frontend**:
- ✅ Activity history view
- ✅ User presence indicators
- ✅ Connection status
- ✅ Loading states
- ✅ Error boundaries
- ✅ Toast notifications

**Documentation**:
- ✅ README with setup
- ✅ API documentation
- ✅ Deployment guide

**Success Criteria**:
- All mutations logged
- Activity history viewable
- UI polished
- System production-ready

---

## Revised Token Budget

| Session | Tokens | Focus |
|---------|--------|-------|
| 10 | 80k | Database + Protobuf setup |
| 20 | 90k | WebSocket infrastructure |
| 30 | 95k | Work items (backend + frontend) |
| 40 | 85k | Sprints + comments |
| 50 | 80k | Time tracking + dependencies |
| 60 | 70k | REST for LLMs + bulk load |
| 70 | 75k | Activity log + polish |
| **Total** | **575k** | 7 core sessions (11-19, 21-29, etc. for incremental work) |

---

## Key Advantages of WebSocket-First

1. **Real-time from day one** - No need to retrofit
2. **Simpler architecture** - One protocol for everything
3. **Better UX** - Optimistic updates with confirmation
4. **Production-ready** - Built for collaboration
5. **REST as enhancement** - Add later for specific needs

Ready to start Session 1 when you are!
