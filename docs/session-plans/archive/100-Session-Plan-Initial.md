# Session 100: LLM-Callable CLI for Work Item CRUD

## Summary

Add a REST API layer to pm-server and a CLI binary (pm-cli) that communicates via HTTP. When the CLI creates/updates/deletes work items, the server broadcasts changes via WebSocket so the Blazor app sees real-time updates.

## Architecture

```
+----------+     HTTP/JSON     +---------------------------+
|  pm-cli  | ----------------> |        pm-server          |
+----------+                   |  +-------+  +----------+  |
                               |  | REST  |  | WebSocket|  |
                               |  | API   |  | Handler  |  |
                               |  +---+---+  +----+-----+  |
                               |      |           |        |
                               |      +-----+-----+        |
                               |            v              |
                               |    +--------------+       |
                               |    |  Broadcast   |-----> Blazor clients
                               |    +--------------+       |
                               +---------------------------+
```

---

## Step 1: Add API Config to pm-config

**File:** `backend/crates/pm-config/src/api_config.rs` (NEW)

```rust
use serde::Deserialize;
use uuid::Uuid;

pub const DEFAULT_API_ENABLED: bool = true;
pub const DEFAULT_LLM_USER_ID: &str = "00000000-0000-0000-0000-000000000001";
pub const DEFAULT_LLM_USER_NAME: &str = "Claude Assistant";

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ApiConfig {
    pub enabled: bool,
    pub llm_user_id: String,
    pub llm_user_name: String,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            enabled: DEFAULT_API_ENABLED,
            llm_user_id: DEFAULT_LLM_USER_ID.to_string(),
            llm_user_name: DEFAULT_LLM_USER_NAME.to_string(),
        }
    }
}

impl ApiConfig {
    pub fn llm_user_uuid(&self) -> Uuid {
        Uuid::parse_str(&self.llm_user_id).unwrap_or_else(|_| {
            Uuid::parse_str(DEFAULT_LLM_USER_ID).unwrap()
        })
    }
}
```

**File:** `backend/crates/pm-config/src/lib.rs` (MODIFY)

```rust
// Add to imports
mod api_config;
pub use api_config::ApiConfig;

// Add to Config struct
pub struct Config {
    // ... existing fields ...
    #[serde(default)]
    pub api: ApiConfig,
}
```

---

## Step 2: Create API Error Types

**File:** `backend/pm-server/src/api/error.rs` (NEW)

```rust
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ApiErrorResponse {
    pub error: ApiErrorBody,
}

#[derive(Debug, Serialize)]
pub struct ApiErrorBody {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
}

#[derive(Debug)]
pub enum ApiError {
    NotFound(String),
    Validation { message: String, field: Option<String> },
    Conflict { message: String, current_version: i32 },
    Internal(String),
    BadRequest(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, body) = match self {
            ApiError::NotFound(msg) => (
                StatusCode::NOT_FOUND,
                ApiErrorBody { code: "NOT_FOUND".into(), message: msg, field: None },
            ),
            ApiError::Validation { message, field } => (
                StatusCode::BAD_REQUEST,
                ApiErrorBody { code: "VALIDATION_ERROR".into(), message, field },
            ),
            ApiError::Conflict { message, current_version } => (
                StatusCode::CONFLICT,
                ApiErrorBody {
                    code: "CONFLICT".into(),
                    message: format!("{} (current version: {})", message, current_version),
                    field: None,
                },
            ),
            ApiError::Internal(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                ApiErrorBody { code: "INTERNAL_ERROR".into(), message: msg, field: None },
            ),
            ApiError::BadRequest(msg) => (
                StatusCode::BAD_REQUEST,
                ApiErrorBody { code: "BAD_REQUEST".into(), message: msg, field: None },
            ),
        };

        (status, Json(ApiErrorResponse { error: body })).into_response()
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(e: sqlx::Error) -> Self {
        ApiError::Internal(format!("Database error: {}", e))
    }
}

impl From<uuid::Error> for ApiError {
    fn from(e: uuid::Error) -> Self {
        ApiError::Validation {
            message: format!("Invalid UUID: {}", e),
            field: None,
        }
    }
}
```

---

## Step 3: Create User ID Extractor

**File:** `backend/pm-server/src/api/extractors.rs` (NEW)

```rust
use axum::{
    extract::FromRequestParts,
    http::{HeaderMap, request::Parts},
};
use uuid::Uuid;

use crate::api::error::ApiError;
use pm_ws::AppState;

/// Extracts user ID from X-User-Id header or falls back to LLM user
pub struct UserId(pub Uuid);

#[axum::async_trait]
impl FromRequestParts<AppState> for UserId {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let headers = &parts.headers;

        // Try X-User-Id header first
        if let Some(header_value) = headers.get("X-User-Id") {
            if let Ok(user_id_str) = header_value.to_str() {
                if let Ok(uuid) = Uuid::parse_str(user_id_str) {
                    return Ok(UserId(uuid));
                }
            }
        }

        // Fall back to configured LLM user ID
        let llm_user_id = Uuid::parse_str(&state.config.llm_user_id)
            .unwrap_or_else(|_| {
                Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap()
            });

        Ok(UserId(llm_user_id))
    }
}
```

---

## Step 4: Create Work Item API Handlers

**File:** `backend/pm-server/src/api/work_items.rs` (NEW)

```rust
use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use pm_core::{ActivityLog, WorkItem, WorkItemType};
use pm_db::{ActivityLogRepository, ProjectRepository, WorkItemRepository};
use pm_ws::{
    AppState, MessageValidator, build_activity_log_created_event,
    validate_hierarchy, validate_status, validate_priority, sanitize_string,
};
use prost::Message as ProstMessage;
use axum::extract::ws::Message;

use crate::api::{error::ApiError, extractors::UserId};

// === Request/Response Types ===

#[derive(Debug, Deserialize)]
pub struct CreateWorkItemRequest {
    pub project_id: String,
    pub item_type: String,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub parent_id: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub priority: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateWorkItemRequest {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub priority: Option<String>,
    #[serde(default)]
    pub assignee_id: Option<String>,
    #[serde(default)]
    pub sprint_id: Option<String>,
    #[serde(default)]
    pub story_points: Option<i32>,
    #[serde(default)]
    pub parent_id: Option<String>,
    #[serde(default)]
    pub update_parent: bool,
    pub expected_version: i32,
}

#[derive(Debug, Serialize)]
pub struct WorkItemResponse {
    pub work_item: WorkItemDto,
}

#[derive(Debug, Serialize)]
pub struct WorkItemListResponse {
    pub work_items: Vec<WorkItemDto>,
}

#[derive(Debug, Serialize)]
pub struct DeleteResponse {
    pub deleted_id: String,
}

#[derive(Debug, Serialize)]
pub struct WorkItemDto {
    pub id: String,
    pub display_key: String,
    pub item_type: String,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub priority: String,
    pub parent_id: Option<String>,
    pub project_id: String,
    pub assignee_id: Option<String>,
    pub sprint_id: Option<String>,
    pub story_points: Option<i32>,
    pub item_number: i32,
    pub position: i32,
    pub version: i32,
    pub created_at: i64,
    pub updated_at: i64,
    pub created_by: String,
    pub updated_by: String,
}

impl From<WorkItem> for WorkItemDto {
    fn from(w: WorkItem) -> Self {
        Self {
            id: w.id.to_string(),
            display_key: format!("PROJ-{}", w.item_number), // TODO: Get real project key
            item_type: w.item_type.as_str().to_string(),
            title: w.title,
            description: w.description,
            status: w.status,
            priority: w.priority,
            parent_id: w.parent_id.map(|id| id.to_string()),
            project_id: w.project_id.to_string(),
            assignee_id: w.assignee_id.map(|id| id.to_string()),
            sprint_id: w.sprint_id.map(|id| id.to_string()),
            story_points: w.story_points,
            item_number: w.item_number,
            position: w.position,
            version: w.version,
            created_at: w.created_at.timestamp(),
            updated_at: w.updated_at.timestamp(),
            created_by: w.created_by.to_string(),
            updated_by: w.updated_by.to_string(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ListWorkItemsQuery {
    #[serde(rename = "type")]
    pub item_type: Option<String>,
    pub status: Option<String>,
    pub sprint_id: Option<String>,
}

// === Handlers ===

pub async fn get_work_item(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<WorkItemResponse>, ApiError> {
    let work_item_id = Uuid::parse_str(&id)?;

    let work_item = WorkItemRepository::find_by_id(&state.pool, work_item_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Work item {} not found", id)))?;

    Ok(Json(WorkItemResponse {
        work_item: work_item.into(),
    }))
}

pub async fn list_work_items(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    Query(query): Query<ListWorkItemsQuery>,
) -> Result<Json<WorkItemListResponse>, ApiError> {
    let project_uuid = Uuid::parse_str(&project_id)?;

    let work_items = WorkItemRepository::find_by_project(&state.pool, project_uuid)
        .await?;

    // Apply filters
    let filtered: Vec<WorkItemDto> = work_items
        .into_iter()
        .filter(|w| {
            query.item_type.as_ref().map_or(true, |t| w.item_type.as_str() == t)
                && query.status.as_ref().map_or(true, |s| &w.status == s)
                && query.sprint_id.as_ref().map_or(true, |sid| {
                    w.sprint_id.map_or(false, |ws| ws.to_string() == *sid)
                })
        })
        .map(WorkItemDto::from)
        .collect();

    Ok(Json(WorkItemListResponse { work_items: filtered }))
}

pub async fn create_work_item(
    State(state): State<AppState>,
    UserId(user_id): UserId,
    Json(req): Json<CreateWorkItemRequest>,
) -> Result<Json<WorkItemResponse>, ApiError> {
    // 1. Parse and validate item_type
    let item_type = WorkItemType::from_str(&req.item_type)
        .map_err(|_| ApiError::Validation {
            message: format!("Invalid item_type: {}. Valid: epic, story, task", req.item_type),
            field: Some("item_type".into()),
        })?;

    // 2. Validate input
    MessageValidator::validate_work_item_create(
        &req.title,
        req.description.as_deref(),
        item_type.as_str(),
    ).map_err(|e| ApiError::Validation {
        message: e.to_string(),
        field: None,
    })?;

    // 3. Parse IDs
    let project_id = Uuid::parse_str(&req.project_id)?;
    let parent_id = req.parent_id
        .as_ref()
        .filter(|s| !s.is_empty())
        .map(|s| Uuid::parse_str(s))
        .transpose()?;

    // 4. Validate hierarchy
    if let Some(pid) = parent_id {
        validate_hierarchy(&state.pool, item_type.clone(), pid)
            .await
            .map_err(|e| ApiError::Validation {
                message: e.to_string(),
                field: Some("parent_id".into()),
            })?;
    }

    // 5. Get next position
    let max_position = WorkItemRepository::find_max_position(&state.pool, project_id, parent_id)
        .await?;

    // 6. Build work item
    let now = Utc::now();
    let mut work_item = WorkItem {
        id: Uuid::new_v4(),
        item_type,
        parent_id,
        project_id,
        position: max_position + 1,
        title: sanitize_string(&req.title),
        description: req.description.as_ref().map(|d| sanitize_string(d)),
        status: req.status.unwrap_or_else(|| "backlog".to_string()),
        priority: req.priority.unwrap_or_else(|| "medium".to_string()),
        assignee_id: None,
        story_points: None,
        sprint_id: None,
        item_number: 0,
        version: 1,
        created_at: now,
        updated_at: now,
        created_by: user_id,
        updated_by: user_id,
        deleted_at: None,
    };

    // 7. Execute transaction
    let activity = ActivityLog::created("work_item", work_item.id, user_id);
    let activity_clone = activity.clone();
    let work_item_clone = work_item.clone();

    let mut tx = state.pool.begin().await?;

    // Get and increment item number
    let item_number = ProjectRepository::get_and_increment_work_item_number(&mut tx, project_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

    work_item.item_number = item_number;
    let mut wi = work_item_clone;
    wi.item_number = item_number;

    WorkItemRepository::create(&mut *tx, &wi).await?;
    ActivityLogRepository::create(&mut *tx, &activity_clone).await?;
    tx.commit().await?;

    // 8. Broadcast to WebSocket clients
    let event = build_activity_log_created_event(&activity);
    let bytes = event.encode_to_vec();
    let message = Message::Binary(bytes.into());
    let _ = state.registry.broadcast_activity_log_created(
        &work_item.project_id.to_string(),
        Some(&work_item.id.to_string()),
        None,
        message,
    ).await;

    work_item.item_number = item_number;
    Ok(Json(WorkItemResponse {
        work_item: work_item.into(),
    }))
}

pub async fn update_work_item(
    State(state): State<AppState>,
    UserId(user_id): UserId,
    Path(id): Path<String>,
    Json(req): Json<UpdateWorkItemRequest>,
) -> Result<Json<WorkItemResponse>, ApiError> {
    let work_item_id = Uuid::parse_str(&id)?;

    // 1. Fetch existing
    let mut work_item = WorkItemRepository::find_by_id(&state.pool, work_item_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Work item {} not found", id)))?;

    // 2. Check version (optimistic locking)
    if work_item.version != req.expected_version {
        return Err(ApiError::Conflict {
            message: "Version mismatch".into(),
            current_version: work_item.version,
        });
    }

    // 3. Apply updates with validation
    if let Some(ref title) = req.title {
        MessageValidator::validate_string(title, "title", 1, 200)
            .map_err(|e| ApiError::Validation { message: e.to_string(), field: Some("title".into()) })?;
        work_item.title = sanitize_string(title);
    }
    if let Some(ref desc) = req.description {
        work_item.description = Some(sanitize_string(desc));
    }
    if let Some(ref status) = req.status {
        validate_status(status)
            .map_err(|e| ApiError::Validation { message: e.to_string(), field: Some("status".into()) })?;
        work_item.status = status.clone();
    }
    if let Some(ref priority) = req.priority {
        validate_priority(priority)
            .map_err(|e| ApiError::Validation { message: e.to_string(), field: Some("priority".into()) })?;
        work_item.priority = priority.clone();
    }
    if let Some(ref assignee_id) = req.assignee_id {
        work_item.assignee_id = if assignee_id.is_empty() { None } else { Some(Uuid::parse_str(assignee_id)?) };
    }
    if let Some(ref sprint_id) = req.sprint_id {
        work_item.sprint_id = if sprint_id.is_empty() { None } else { Some(Uuid::parse_str(sprint_id)?) };
    }
    if let Some(sp) = req.story_points {
        work_item.story_points = Some(sp);
    }
    if req.update_parent {
        work_item.parent_id = req.parent_id
            .as_ref()
            .filter(|s| !s.is_empty())
            .map(|s| Uuid::parse_str(s))
            .transpose()?;
    }

    // 4. Update metadata
    let now = Utc::now();
    work_item.updated_at = now;
    work_item.updated_by = user_id;
    work_item.version += 1;

    // 5. Execute transaction
    let activity = ActivityLog::updated("work_item", work_item.id, user_id, &[]);
    let work_item_clone = work_item.clone();
    let activity_clone = activity.clone();

    let mut tx = state.pool.begin().await?;
    WorkItemRepository::update(&mut *tx, &work_item_clone).await?;
    ActivityLogRepository::create(&mut *tx, &activity_clone).await?;
    tx.commit().await?;

    // 6. Broadcast
    let event = build_activity_log_created_event(&activity);
    let bytes = event.encode_to_vec();
    let message = Message::Binary(bytes.into());
    let _ = state.registry.broadcast_activity_log_created(
        &work_item.project_id.to_string(),
        Some(&work_item.id.to_string()),
        None,
        message,
    ).await;

    Ok(Json(WorkItemResponse {
        work_item: work_item.into(),
    }))
}

pub async fn delete_work_item(
    State(state): State<AppState>,
    UserId(user_id): UserId,
    Path(id): Path<String>,
) -> Result<Json<DeleteResponse>, ApiError> {
    let work_item_id = Uuid::parse_str(&id)?;

    // 1. Fetch existing
    let work_item = WorkItemRepository::find_by_id(&state.pool, work_item_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Work item {} not found", id)))?;

    // 2. Check for children
    let children = WorkItemRepository::find_children(&state.pool, work_item_id).await?;
    if !children.is_empty() {
        return Err(ApiError::Validation {
            message: format!("Cannot delete: has {} child item(s)", children.len()),
            field: None,
        });
    }

    // 3. Execute transaction
    let activity = ActivityLog::deleted("work_item", work_item_id, user_id);
    let activity_clone = activity.clone();

    let mut tx = state.pool.begin().await?;
    WorkItemRepository::soft_delete(&mut *tx, work_item_id, user_id).await?;
    ActivityLogRepository::create(&mut *tx, &activity_clone).await?;
    tx.commit().await?;

    // 4. Broadcast
    let event = build_activity_log_created_event(&activity);
    let bytes = event.encode_to_vec();
    let message = Message::Binary(bytes.into());
    let _ = state.registry.broadcast_activity_log_created(
        &work_item.project_id.to_string(),
        Some(&work_item_id.to_string()),
        None,
        message,
    ).await;

    Ok(Json(DeleteResponse {
        deleted_id: work_item_id.to_string(),
    }))
}
```

---

## Step 5: Create Project API Handlers

**File:** `backend/pm-server/src/api/projects.rs` (NEW)

```rust
use axum::{extract::{Path, State}, Json};
use serde::Serialize;
use uuid::Uuid;

use pm_core::Project;
use pm_db::ProjectRepository;
use pm_ws::AppState;

use crate::api::error::ApiError;

#[derive(Debug, Serialize)]
pub struct ProjectResponse {
    pub project: ProjectDto,
}

#[derive(Debug, Serialize)]
pub struct ProjectListResponse {
    pub projects: Vec<ProjectDto>,
}

#[derive(Debug, Serialize)]
pub struct ProjectDto {
    pub id: String,
    pub key: String,
    pub title: String,
    pub description: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl From<Project> for ProjectDto {
    fn from(p: Project) -> Self {
        Self {
            id: p.id.to_string(),
            key: p.key,
            title: p.title,
            description: p.description,
            created_at: p.created_at.timestamp(),
            updated_at: p.updated_at.timestamp(),
        }
    }
}

pub async fn list_projects(
    State(state): State<AppState>,
) -> Result<Json<ProjectListResponse>, ApiError> {
    let projects = ProjectRepository::find_all(&state.pool).await?;

    Ok(Json(ProjectListResponse {
        projects: projects.into_iter().map(ProjectDto::from).collect(),
    }))
}

pub async fn get_project(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ProjectResponse>, ApiError> {
    let project_id = Uuid::parse_str(&id)?;

    let project = ProjectRepository::find_by_id(&state.pool, project_id)
        .await?
        .ok_or_else(|| ApiError::NotFound(format!("Project {} not found", id)))?;

    Ok(Json(ProjectResponse {
        project: project.into(),
    }))
}
```

---

## Step 6: Create API Module and Routes

**File:** `backend/pm-server/src/api/mod.rs` (NEW)

```rust
pub mod error;
pub mod extractors;
pub mod projects;
pub mod work_items;

pub use error::ApiError;
pub use extractors::UserId;
```

**File:** `backend/pm-server/src/routes.rs` (MODIFY)

```rust
use crate::{admin, api, health};

use pm_ws::AppState;

use axum::{
    Router,
    routing::{delete, get, post, put},
};
use tower_http::cors::{Any, CorsLayer};

pub fn build_router(state: AppState) -> Router {
    Router::new()
        // WebSocket endpoint
        .route("/ws", get(pm_ws::handler))
        // Health check endpoints
        .route("/health", get(health::health))
        .route("/live", get(health::liveness))
        .route("/ready", get(health::readiness))
        // Admin endpoints
        .route("/admin/checkpoint", post(admin::checkpoint_handler))
        .route("/admin/shutdown", post(admin::shutdown_handler))
        // REST API v1
        .route("/api/v1/projects", get(api::projects::list_projects))
        .route("/api/v1/projects/:id", get(api::projects::get_project))
        .route("/api/v1/projects/:id/work-items", get(api::work_items::list_work_items))
        .route("/api/v1/work-items", post(api::work_items::create_work_item))
        .route("/api/v1/work-items/:id", get(api::work_items::get_work_item))
        .route("/api/v1/work-items/:id", put(api::work_items::update_work_item))
        .route("/api/v1/work-items/:id", delete(api::work_items::delete_work_item))
        // Add shared state
        .with_state(state)
        // CORS middleware
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
}
```

**File:** `backend/pm-server/src/main.rs` (MODIFY)

Add `mod api;` to the module declarations.

---

## Step 7: Update Cargo.toml

**File:** `Cargo.toml` (root, MODIFY)

```toml
[workspace.dependencies]
# Add these new dependencies
clap = { version = "4.5", features = ["derive"] }
reqwest = { version = "0.12", features = ["json", "rustls-tls"], default-features = false }

[workspace]
members = [
    # ... existing members ...
    "backend/crates/pm-cli",
]
```

---

## Step 8: Create pm-cli Crate

**File:** `backend/crates/pm-cli/Cargo.toml` (NEW)

```toml
[package]
name = "pm-cli"
version = "0.1.0"
edition = "2024"

[[bin]]
name = "pm"
path = "src/main.rs"

[dependencies]
clap = { workspace = true }
reqwest = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
tokio = { workspace = true }
uuid = { workspace = true }
thiserror = { workspace = true }
```

**File:** `backend/crates/pm-cli/src/main.rs` (NEW)

```rust
use clap::{Parser, Subcommand};

mod client;
mod commands;

use client::PmClient;

#[derive(Parser)]
#[command(name = "pm")]
#[command(about = "Blazor Agile Board CLI for LLM integration")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Server URL
    #[arg(long, default_value = "http://127.0.0.1:8000", global = true)]
    server: String,

    /// User ID (optional, uses LLM user by default)
    #[arg(long, global = true)]
    user_id: Option<String>,

    /// Pretty-print JSON output
    #[arg(long, global = true)]
    pretty: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Project operations
    Project {
        #[command(subcommand)]
        action: ProjectCommands,
    },
    /// Work item operations
    WorkItem {
        #[command(subcommand)]
        action: WorkItemCommands,
    },
}

#[derive(Subcommand)]
enum ProjectCommands {
    /// List all projects
    List,
    /// Get a project by ID
    Get { id: String },
}

#[derive(Subcommand)]
enum WorkItemCommands {
    /// Create a new work item
    Create {
        #[arg(long)]
        project_id: String,
        #[arg(long, value_parser = ["epic", "story", "task"])]
        r#type: String,
        #[arg(long)]
        title: String,
        #[arg(long)]
        description: Option<String>,
        #[arg(long)]
        parent_id: Option<String>,
        #[arg(long)]
        status: Option<String>,
        #[arg(long)]
        priority: Option<String>,
    },
    /// Get a work item by ID
    Get { id: String },
    /// List work items in a project
    List {
        project_id: String,
        #[arg(long, value_parser = ["epic", "story", "task"])]
        r#type: Option<String>,
        #[arg(long)]
        status: Option<String>,
    },
    /// Update a work item
    Update {
        id: String,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        description: Option<String>,
        #[arg(long)]
        status: Option<String>,
        #[arg(long)]
        priority: Option<String>,
        #[arg(long)]
        assignee_id: Option<String>,
        #[arg(long)]
        sprint_id: Option<String>,
        #[arg(long)]
        story_points: Option<i32>,
        #[arg(long)]
        version: i32,
    },
    /// Delete a work item
    Delete { id: String },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let client = PmClient::new(&cli.server, cli.user_id.as_deref());

    let result = match cli.command {
        Commands::Project { action } => match action {
            ProjectCommands::List => client.list_projects().await,
            ProjectCommands::Get { id } => client.get_project(&id).await,
        },
        Commands::WorkItem { action } => match action {
            WorkItemCommands::Create {
                project_id, r#type, title, description, parent_id, status, priority,
            } => {
                client.create_work_item(
                    &project_id, &r#type, &title, description.as_deref(),
                    parent_id.as_deref(), status.as_deref(), priority.as_deref(),
                ).await
            }
            WorkItemCommands::Get { id } => client.get_work_item(&id).await,
            WorkItemCommands::List { project_id, r#type, status } => {
                client.list_work_items(&project_id, r#type.as_deref(), status.as_deref()).await
            }
            WorkItemCommands::Update {
                id, title, description, status, priority,
                assignee_id, sprint_id, story_points, version,
            } => {
                client.update_work_item(
                    &id, title.as_deref(), description.as_deref(), status.as_deref(),
                    priority.as_deref(), assignee_id.as_deref(), sprint_id.as_deref(),
                    story_points, version,
                ).await
            }
            WorkItemCommands::Delete { id } => client.delete_work_item(&id).await,
        },
    }?;

    // Print result
    if cli.pretty {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!("{}", serde_json::to_string(&result)?);
    }

    Ok(())
}
```

**File:** `backend/crates/pm-cli/src/client.rs` (NEW)

```rust
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub struct PmClient {
    base_url: String,
    user_id: Option<String>,
    client: Client,
}

impl PmClient {
    pub fn new(base_url: &str, user_id: Option<&str>) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            user_id: user_id.map(String::from),
            client: Client::new(),
        }
    }

    fn request(&self, method: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
        let url = format!("{}{}", self.base_url, path);
        let mut req = self.client.request(method, &url);
        if let Some(ref user_id) = self.user_id {
            req = req.header("X-User-Id", user_id);
        }
        req
    }

    pub async fn list_projects(&self) -> Result<Value, Box<dyn std::error::Error>> {
        let resp = self.request(reqwest::Method::GET, "/api/v1/projects")
            .send().await?
            .json().await?;
        Ok(resp)
    }

    pub async fn get_project(&self, id: &str) -> Result<Value, Box<dyn std::error::Error>> {
        let resp = self.request(reqwest::Method::GET, &format!("/api/v1/projects/{}", id))
            .send().await?
            .json().await?;
        Ok(resp)
    }

    pub async fn get_work_item(&self, id: &str) -> Result<Value, Box<dyn std::error::Error>> {
        let resp = self.request(reqwest::Method::GET, &format!("/api/v1/work-items/{}", id))
            .send().await?
            .json().await?;
        Ok(resp)
    }

    pub async fn list_work_items(
        &self,
        project_id: &str,
        item_type: Option<&str>,
        status: Option<&str>,
    ) -> Result<Value, Box<dyn std::error::Error>> {
        let mut url = format!("/api/v1/projects/{}/work-items", project_id);
        let mut params = vec![];
        if let Some(t) = item_type { params.push(format!("type={}", t)); }
        if let Some(s) = status { params.push(format!("status={}", s)); }
        if !params.is_empty() { url.push_str(&format!("?{}", params.join("&"))); }

        let resp = self.request(reqwest::Method::GET, &url)
            .send().await?
            .json().await?;
        Ok(resp)
    }

    pub async fn create_work_item(
        &self,
        project_id: &str,
        item_type: &str,
        title: &str,
        description: Option<&str>,
        parent_id: Option<&str>,
        status: Option<&str>,
        priority: Option<&str>,
    ) -> Result<Value, Box<dyn std::error::Error>> {
        #[derive(Serialize)]
        struct Request<'a> {
            project_id: &'a str,
            item_type: &'a str,
            title: &'a str,
            #[serde(skip_serializing_if = "Option::is_none")]
            description: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            parent_id: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            status: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            priority: Option<&'a str>,
        }

        let body = Request { project_id, item_type, title, description, parent_id, status, priority };
        let resp = self.request(reqwest::Method::POST, "/api/v1/work-items")
            .json(&body)
            .send().await?
            .json().await?;
        Ok(resp)
    }

    pub async fn update_work_item(
        &self,
        id: &str,
        title: Option<&str>,
        description: Option<&str>,
        status: Option<&str>,
        priority: Option<&str>,
        assignee_id: Option<&str>,
        sprint_id: Option<&str>,
        story_points: Option<i32>,
        expected_version: i32,
    ) -> Result<Value, Box<dyn std::error::Error>> {
        #[derive(Serialize)]
        struct Request<'a> {
            #[serde(skip_serializing_if = "Option::is_none")]
            title: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            description: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            status: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            priority: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            assignee_id: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            sprint_id: Option<&'a str>,
            #[serde(skip_serializing_if = "Option::is_none")]
            story_points: Option<i32>,
            expected_version: i32,
        }

        let body = Request {
            title, description, status, priority, assignee_id, sprint_id, story_points, expected_version,
        };
        let resp = self.request(reqwest::Method::PUT, &format!("/api/v1/work-items/{}", id))
            .json(&body)
            .send().await?
            .json().await?;
        Ok(resp)
    }

    pub async fn delete_work_item(&self, id: &str) -> Result<Value, Box<dyn std::error::Error>> {
        let resp = self.request(reqwest::Method::DELETE, &format!("/api/v1/work-items/{}", id))
            .send().await?
            .json().await?;
        Ok(resp)
    }
}
```

---

## Step 9: Add Justfile Commands

**File:** `justfile` (MODIFY - add to Rust Backend section)

```just
# CLI package
rust_cli := "pm-cli"

# Build CLI
build-rs-cli:
    cargo build -p {{rust_cli}}

# Build CLI (release)
build-rs-cli-release:
    cargo build -p {{rust_cli}} --release

# Run CLI with args
run-cli *ARGS:
    cargo run -p {{rust_cli}} -- {{ARGS}}

# Test CLI
test-rs-cli:
    cargo test -p {{rust_cli}}

# Check CLI
check-rs-cli:
    cargo check -p {{rust_cli}} {{cargo_all_targets}}

# Clippy CLI
clippy-rs-cli:
    cargo clippy -p {{rust_cli}} {{cargo_all_targets}} {{cargo_all_features}} -- -D warnings
```

---

## Step 10: Ensure LLM User Exists in Database

**File:** `backend/pm-server/src/main.rs` (MODIFY - add to startup)

```rust
// After database pool is created, ensure LLM user exists
async fn ensure_llm_user(pool: &SqlitePool, config: &Config) {
    let llm_user_id = &config.api.llm_user_id;
    let llm_user_name = &config.api.llm_user_name;

    let _ = sqlx::query(
        "INSERT OR IGNORE INTO users (id, email, display_name) VALUES (?, ?, ?)"
    )
    .bind(llm_user_id)
    .bind(format!("{}@system.local", llm_user_id))
    .bind(llm_user_name)
    .execute(pool)
    .await;
}
```

---

## Verification Steps

1. **Build everything:**
   ```bash
   just restore
   just check-backend
   just build-rs-cli
   ```

2. **Start the server:**
   ```bash
   just build-rs-server
   cargo run -p pm-server
   ```

3. **Start Blazor app (separate terminal):**
   ```bash
   just dev
   ```

4. **Test CLI commands:**
   ```bash
   # List projects
   pm project list --pretty

   # Create a work item
   pm work-item create \
     --project-id <project-uuid> \
     --type story \
     --title "Test from CLI" \
     --description "Created by Claude" \
     --pretty

   # Verify it appears in Blazor UI in real-time!

   # List work items
   pm work-item list <project-uuid> --pretty

   # Update a work item
   pm work-item update <work-item-uuid> \
     --status in_progress \
     --version 1 \
     --pretty

   # Delete a work item
   pm work-item delete <work-item-uuid> --pretty
   ```

5. **Run tests:**
   ```bash
   just test-backend
   just test-rs-cli
   ```

---

## Files Summary

| File | Action | Description |
|------|--------|-------------|
| `backend/crates/pm-config/src/api_config.rs` | NEW | API configuration with LLM user settings |
| `backend/crates/pm-config/src/lib.rs` | MODIFY | Export ApiConfig |
| `backend/pm-server/src/api/mod.rs` | NEW | API module exports |
| `backend/pm-server/src/api/error.rs` | NEW | API error types |
| `backend/pm-server/src/api/extractors.rs` | NEW | User ID extraction |
| `backend/pm-server/src/api/work_items.rs` | NEW | Work item CRUD handlers |
| `backend/pm-server/src/api/projects.rs` | NEW | Project handlers |
| `backend/pm-server/src/routes.rs` | MODIFY | Add API routes |
| `backend/pm-server/src/main.rs` | MODIFY | Add api module, ensure LLM user |
| `backend/crates/pm-cli/Cargo.toml` | NEW | CLI crate manifest |
| `backend/crates/pm-cli/src/main.rs` | NEW | CLI entry point |
| `backend/crates/pm-cli/src/client.rs` | NEW | HTTP client |
| `Cargo.toml` (root) | MODIFY | Add clap, reqwest, pm-cli member |
| `justfile` | MODIFY | Add CLI build commands |

**Total: ~14 files (10 new, 4 modified)**
