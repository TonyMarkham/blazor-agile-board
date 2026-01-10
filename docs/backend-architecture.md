# Backend Architecture - Rust + Axum

This document outlines the structure and organization of the Rust backend.

## Project Structure

```
/backend
├── Cargo.toml                 # Workspace root
├── build.rs                   # Build script for protobuf codegen
│
├── /crates
│   ├── /pm-core               # Core domain models and business logic
│   │   ├── Cargo.toml
│   │   └── /src
│   │       ├── lib.rs
│   │       ├── /models        # Domain models (WorkItem, Sprint, etc.)
│   │       ├── /services      # Business logic services
│   │       └── /errors        # Error types
│   │
│   ├── /pm-db                 # Database layer (SQLx)
│   │   ├── Cargo.toml
│   │   └── /src
│   │       ├── lib.rs
│   │       ├── /migrations    # SQLx migration files
│   │       ├── /repositories  # Data access layer
│   │       ├── /queries       # Complex queries
│   │       └── connection.rs  # Per-tenant connection management
│   │
│   ├── /pm-api                # REST API routes and handlers
│   │   ├── Cargo.toml
│   │   └── /src
│   │       ├── lib.rs
│   │       ├── /routes        # Route definitions
│   │       ├── /handlers      # Request handlers
│   │       ├── /dto           # Data transfer objects
│   │       └── /validators    # Request validation
│   │
│   ├── /pm-ws                 # WebSocket real-time communication
│   │   ├── Cargo.toml
│   │   └── /src
│   │       ├── lib.rs
│   │       ├── connection.rs  # WebSocket connection handling
│   │       ├── broadcast.rs   # Per-tenant broadcast channels
│   │       ├── subscription.rs # Subscription management
│   │       └── handlers.rs    # WebSocket message handlers
│   │
│   ├── /pm-auth               # Authentication and authorization
│   │   ├── Cargo.toml
│   │   └── /src
│   │       ├── lib.rs
│   │       ├── jwt.rs         # JWT token validation
│   │       ├── middleware.rs  # Auth middleware
│   │       └── tenant.rs      # Tenant context extraction
│   │
│   └── /pm-proto              # Protobuf generated code
│       ├── Cargo.toml
│       ├── build.rs           # Protobuf codegen
│       └── /src
│           └── lib.rs         # Generated protobuf types
│
└── /pm-server                 # Main server binary
    ├── Cargo.toml
    └── /src
        ├── main.rs            # Entry point
        ├── config.rs          # Configuration
        └── server.rs          # Axum app setup
```

---

## Crate Descriptions

### pm-core (Domain Layer)

**Purpose**: Pure business logic, no external dependencies (DB, HTTP, etc.)

**Key Types:**
```rust
// models/work_item.rs
pub struct WorkItem {
    pub id: Uuid,
    pub item_type: WorkItemType,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub parent_id: Option<Uuid>,
    pub project_id: Uuid,
    // ...
}

pub enum WorkItemType {
    Project,
    Epic,
    Story,
    Task,
}

// services/work_item_service.rs
pub struct WorkItemService {
    // Business logic for work items
}

impl WorkItemService {
    pub fn validate_status_transition(&self, from: &str, to: &str) -> Result<()> {
        // Validate workflow rules
    }

    pub fn can_assign_to_sprint(&self, item: &WorkItem) -> Result<()> {
        // Only stories and tasks can be assigned to sprints
    }
}
```

**Dependencies:**
- uuid
- chrono
- serde
- thiserror

---

### pm-db (Data Access Layer)

**Purpose**: Database operations using SQLx with per-tenant connection management

**Key Types:**
```rust
// connection.rs
pub struct TenantConnectionManager {
    pools: Arc<RwLock<HashMap<String, SqlitePool>>>,
    base_path: PathBuf,
}

impl TenantConnectionManager {
    pub async fn get_connection(&self, tenant_id: &str) -> Result<SqlitePool> {
        // Get or create connection pool for tenant
        // Path: {base_path}/tenants/{tenant_id}/main.db
    }
}

// repositories/work_item_repository.rs
pub struct WorkItemRepository {
    pool: SqlitePool,
}

impl WorkItemRepository {
    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<WorkItem>> {
        sqlx::query_as!(
            WorkItem,
            r#"
            SELECT * FROM pm_work_items
            WHERE id = ? AND deleted_at IS NULL
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn find_by_project(&self, project_id: Uuid) -> Result<Vec<WorkItem>> {
        // ...
    }

    pub async fn create(&self, item: &WorkItem) -> Result<()> {
        // Insert + activity log
    }
}
```

**Migrations:**
```
/migrations
├── 20240101_001_create_work_items.sql
├── 20240101_002_create_sprints.sql
├── 20240101_003_create_comments.sql
├── 20240101_004_create_time_entries.sql
├── 20240101_005_create_dependencies.sql
├── 20240101_006_create_activity_log.sql
├── 20240101_007_create_swim_lanes.sql
└── 20240101_008_create_llm_context.sql
```

**Dependencies:**
- sqlx (with sqlite feature)
- uuid
- chrono

---

### pm-api (REST API Layer)

**Purpose**: HTTP REST endpoints using Axum

**Route Structure:**
```rust
// routes/mod.rs
pub fn routes() -> Router {
    Router::new()
        .nest("/projects", projects::routes())
        .nest("/work-items", work_items::routes())
        .nest("/sprints", sprints::routes())
        .nest("/comments", comments::routes())
        .nest("/time-entries", time_entries::routes())
        .nest("/dependencies", dependencies::routes())
        .nest("/llm", llm::routes())
}

// routes/work_items.rs
pub fn routes() -> Router {
    Router::new()
        .route("/", get(list_work_items).post(create_work_item))
        .route("/:id", get(get_work_item).put(update_work_item).delete(delete_work_item))
        .route("/:id/comments", get(list_comments).post(create_comment))
        .route("/:id/time-entries", get(list_time_entries))
        .route("/:id/dependencies", get(list_dependencies))
}

// handlers/work_items.rs
pub async fn create_work_item(
    State(state): State<AppState>,
    Extension(tenant_id): Extension<String>,
    Extension(user_id): Extension<String>,
    Json(payload): Json<CreateWorkItemDto>,
) -> Result<Json<WorkItemDto>, ApiError> {
    // 1. Validate input
    // 2. Get tenant connection
    // 3. Create work item via repository
    // 4. Log activity
    // 5. Broadcast WebSocket event
    // 6. Return response
}
```

**DTOs:**
```rust
// dto/work_item.rs
#[derive(Serialize, Deserialize)]
pub struct CreateWorkItemDto {
    pub item_type: WorkItemType,
    pub title: String,
    pub description: Option<String>,
    pub parent_id: Option<Uuid>,
    pub assignee_id: Option<Uuid>,
}

#[derive(Serialize, Deserialize)]
pub struct UpdateWorkItemDto {
    pub title: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
    pub assignee_id: Option<Uuid>,
    pub sprint_id: Option<Uuid>,
}

#[derive(Serialize, Deserialize)]
pub struct WorkItemDto {
    pub id: Uuid,
    pub item_type: WorkItemType,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub assignee: Option<UserDto>,
    pub sprint: Option<SprintDto>,
    pub created_at: i64,
    pub updated_at: i64,
}
```

**Dependencies:**
- axum
- serde
- validator
- tower
- tower-http

---

### pm-ws (WebSocket Layer)

**Purpose**: Real-time bidirectional communication with Protobuf

**Key Types:**
```rust
// connection.rs
pub struct WebSocketConnection {
    pub user_id: String,
    pub tenant_id: String,
    pub subscriptions: Arc<RwLock<HashSet<String>>>,
    pub sender: SplitSink<WebSocket, Message>,
    pub receiver: SplitStream<WebSocket>,
}

impl WebSocketConnection {
    pub async fn handle(self, broadcast_rx: broadcast::Receiver<WebSocketMessage>) {
        tokio::select! {
            // Receive from client
            result = self.handle_incoming() => { /* ... */ }
            // Broadcast to client
            result = self.handle_outgoing(broadcast_rx) => { /* ... */ }
        }
    }

    async fn handle_incoming(&mut self) -> Result<()> {
        while let Some(msg) = self.receiver.next().await {
            let bytes = msg?.into_data();
            let ws_msg = WebSocketMessage::decode(&bytes[..])?;

            match ws_msg.payload {
                Some(Payload::Subscribe(sub)) => {
                    self.handle_subscribe(sub).await?;
                }
                Some(Payload::Ping(ping)) => {
                    self.send_pong(ping).await?;
                }
                _ => { /* Handle other message types */ }
            }
        }
        Ok(())
    }

    async fn handle_outgoing(&mut self, mut rx: broadcast::Receiver<WebSocketMessage>) -> Result<()> {
        while let Ok(msg) = rx.recv().await {
            // Filter: only send if client subscribed to this resource
            if self.should_send(&msg) {
                let bytes = msg.encode_to_vec();
                self.sender.send(Message::Binary(bytes)).await?;
            }
        }
        Ok(())
    }
}

// broadcast.rs
pub struct TenantBroadcaster {
    channels: Arc<RwLock<HashMap<String, broadcast::Sender<WebSocketMessage>>>>,
}

impl TenantBroadcaster {
    pub fn get_or_create(&self, tenant_id: &str) -> broadcast::Sender<WebSocketMessage> {
        // Get or create broadcast channel for tenant
    }

    pub fn broadcast(&self, tenant_id: &str, msg: WebSocketMessage) -> Result<()> {
        if let Some(tx) = self.channels.read().get(tenant_id) {
            tx.send(msg)?;
        }
        Ok(())
    }
}
```

**Dependencies:**
- axum (with ws feature)
- tokio
- prost
- pm-proto

---

### pm-auth (Authentication & Authorization)

**Purpose**: JWT validation, tenant context extraction, middleware

**Key Types:**
```rust
// jwt.rs
pub struct JwtValidator {
    secret: Vec<u8>,
}

impl JwtValidator {
    pub fn validate(&self, token: &str) -> Result<Claims> {
        // Validate JWT signature and expiration
    }
}

#[derive(Deserialize)]
pub struct Claims {
    pub sub: String,        // user_id
    pub tenant_id: String,
    pub exp: i64,
}

// middleware.rs
pub async fn auth_middleware(
    State(validator): State<Arc<JwtValidator>>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // 1. Extract Authorization header
    let token = extract_token(&request)?;

    // 2. Validate JWT
    let claims = validator.validate(token)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // 3. Insert extensions for downstream handlers
    request.extensions_mut().insert(claims.tenant_id.clone());
    request.extensions_mut().insert(claims.sub.clone());

    Ok(next.run(request).await)
}

// tenant.rs
pub async fn tenant_context_middleware(
    Extension(tenant_id): Extension<String>,
    State(conn_manager): State<Arc<TenantConnectionManager>>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Get database connection for this tenant
    let pool = conn_manager.get_connection(&tenant_id).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    request.extensions_mut().insert(pool);

    Ok(next.run(request).await)
}
```

**Dependencies:**
- jsonwebtoken
- axum

---

### pm-proto (Protobuf Types)

**Purpose**: Generated Rust types from .proto files

```rust
// build.rs
fn main() {
    prost_build::Config::new()
        .out_dir("src/generated")
        .compile_protos(&["../../proto/messages.proto"], &["../../proto"])
        .unwrap();
}

// lib.rs
pub mod generated;
pub use generated::*;

// Re-export for convenience
pub use generated::web_socket_message::Payload;
pub use generated::{
    WebSocketMessage, WorkItem, Sprint, Comment,
    WorkItemCreated, WorkItemUpdated, // ...
};
```

**Dependencies:**
- prost
- prost-build

---

### pm-server (Main Binary)

**Purpose**: Application entry point, configuration, server setup

```rust
// main.rs
#[tokio::main]
async fn main() -> Result<()> {
    // Load configuration
    let config = Config::from_env()?;

    // Setup logging
    tracing_subscriber::fmt::init();

    // Initialize services
    let conn_manager = Arc::new(TenantConnectionManager::new(&config.db_path));
    let jwt_validator = Arc::new(JwtValidator::new(&config.jwt_secret));
    let broadcaster = Arc::new(TenantBroadcaster::new());

    // Build Axum app
    let app = create_app(config, conn_manager, jwt_validator, broadcaster)?;

    // Run server
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    tracing::info!("Server listening on {}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}

// server.rs
pub fn create_app(
    config: Config,
    conn_manager: Arc<TenantConnectionManager>,
    jwt_validator: Arc<JwtValidator>,
    broadcaster: Arc<TenantBroadcaster>,
) -> Result<Router> {
    let app = Router::new()
        // Health check
        .route("/health", get(|| async { "OK" }))

        // WebSocket endpoint
        .route("/ws", get(websocket_handler))

        // REST API
        .nest("/api/v1", pm_api::routes())

        // Middleware
        .layer(middleware::from_fn_with_state(
            jwt_validator.clone(),
            auth_middleware
        ))
        .layer(middleware::from_fn_with_state(
            conn_manager.clone(),
            tenant_context_middleware
        ))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())

        // Shared state
        .with_state(AppState {
            conn_manager,
            jwt_validator,
            broadcaster,
        });

    Ok(app)
}

#[derive(Clone)]
pub struct AppState {
    pub conn_manager: Arc<TenantConnectionManager>,
    pub jwt_validator: Arc<JwtValidator>,
    pub broadcaster: Arc<TenantBroadcaster>,
}
```

**Dependencies:**
- axum
- tokio
- tower
- tower-http
- tracing
- tracing-subscriber

---

## Workspace Cargo.toml

```toml
[workspace]
members = [
    "crates/pm-core",
    "crates/pm-db",
    "crates/pm-api",
    "crates/pm-ws",
    "crates/pm-auth",
    "crates/pm-proto",
    "pm-server",
]
resolver = "2"

[workspace.dependencies]
# Async runtime
tokio = { version = "1.35", features = ["full"] }

# Web framework
axum = { version = "0.7", features = ["ws", "macros"] }
tower = "0.4"
tower-http = { version = "0.5", features = ["cors", "trace"] }

# Database
sqlx = { version = "0.7", features = ["runtime-tokio-rustls", "sqlite", "uuid", "chrono"] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
prost = "0.12"
prost-build = "0.12"

# Utilities
uuid = { version = "1.6", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
thiserror = "1.0"
anyhow = "1.0"

# Auth
jsonwebtoken = "9.2"

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# Validation
validator = { version = "0.16", features = ["derive"] }
```

---

## Configuration

```rust
// config.rs
use serde::Deserialize;

#[derive(Deserialize, Clone)]
pub struct Config {
    pub server_host: String,
    pub server_port: u16,

    pub db_path: String,  // Base path for tenant databases

    pub jwt_secret: String,
    pub jwt_expiration_hours: i64,

    pub cors_origins: Vec<String>,

    pub log_level: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        envy::from_env()
            .context("Failed to load configuration from environment")
    }
}

// .env.example
SERVER_HOST=0.0.0.0
SERVER_PORT=3000
DB_PATH=/data/tenants
JWT_SECRET=your-secret-key
JWT_EXPIRATION_HOURS=24
CORS_ORIGINS=http://localhost:5000,https://app.example.com
LOG_LEVEL=info
```

---

## Testing Strategy

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_work_item_validation() {
        let service = WorkItemService::new();
        assert!(service.can_assign_to_sprint(&task).is_ok());
        assert!(service.can_assign_to_sprint(&epic).is_err());
    }
}
```

### Integration Tests
```rust
#[tokio::test]
async fn test_create_work_item_api() {
    let app = test_app().await;

    let response = app
        .post("/api/v1/work-items")
        .json(&create_dto)
        .bearer_token(&test_jwt())
        .send()
        .await;

    assert_eq!(response.status(), StatusCode::CREATED);
}
```

### WebSocket Tests
```rust
#[tokio::test]
async fn test_websocket_broadcast() {
    let (client1, client2) = connect_two_clients().await;

    client1.subscribe(vec!["project-1"]).await;
    client2.subscribe(vec!["project-1"]).await;

    // Create work item via REST API
    create_work_item("project-1").await;

    // Both clients should receive WorkItemCreated event
    assert!(client1.receive().await.is_work_item_created());
    assert!(client2.receive().await.is_work_item_created());
}
```

---

## Deployment

### Docker
```dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates
COPY --from=builder /app/target/release/pm-server /usr/local/bin/
CMD ["pm-server"]
```

### Standalone Mode
```bash
# Build
cargo build --release

# Run
./target/release/pm-server

# With env file
./target/release/pm-server --env-file .env.production
```

### Plugin Mode (Integrated into SaaS Platform)
- Backend services registered as modules in host platform
- Share database connection manager
- Share auth middleware
- WebSocket endpoint exposed under platform's router
