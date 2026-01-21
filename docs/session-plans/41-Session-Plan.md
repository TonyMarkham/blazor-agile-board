# Session 41: Project Dialog - Dedicated UI for Project Management

**Status**: Planning (2026-01-21)

---

## Problem

The "Create Project" button opens `WorkItemDialog`, showing irrelevant fields (Status, Priority, Story Points, Sprint). Root cause: Projects are in `pm_work_items` but are fundamentally different - they're organizational containers, not work items.

**Solution**: Promote Projects to first-class entities with dedicated table, model, and UI.

---

## Implementation Order

Each step depends only on steps above it. Implement sequentially.

| # | File | What |
|---|------|------|
| 1 | `pm-db/migrations/YYYYMMDD_create_projects_table.sql` | Create `pm_projects` table, migrate data, update FKs |
| 2 | `pm-core/src/models/project.rs` + `mod.rs` | Rust `Project` struct + `ProjectStatus` enum |
| 3 | `pm-db/src/repositories/project_repository.rs` + `mod.rs` | CRUD operations |
| 4 | `proto/messages.proto` | Project messages + Payload variants |
| 5 | `pm-ws/src/handlers/project.rs` + `mod.rs` + `dispatcher.rs` | WebSocket handlers |
| 5b | `pm-ws/src/handlers/response_builder.rs` | Add `build_project_*` functions |
| 5c | `pm-ws/src/handlers/validation.rs` | Add `validate_key` function |
| 6 | `Core/Models/ProjectStatus.cs` | C# enum |
| 7 | `Core/Models/Project.cs` | C# record |
| 8 | `Core/Models/CreateProjectRequest.cs` | DTO |
| 9 | `Core/Models/UpdateProjectRequest.cs` | DTO |
| 9b | `Core/Models/WorkItemType.cs` | Remove `Project` enum value |
| 10 | `Core/Interfaces/IWebSocketClient.cs` | Add events + operations |
| 11 | `Services/WebSocket/WebSocketClient.cs` | Implement project methods |
| 11b | `Services/WebSocket/ProtoConverter.cs` | Add Project ↔ proto converters |
| 12 | `Core/ViewModels/ProjectViewModel.cs` | Wraps Project |
| 12b | `Core/ViewModels/ViewModelFactory.cs` | Add `Create(Project)` overload |
| 13 | `Core/Interfaces/IProjectStore.cs` | Interface with async CRUD |
| 14 | `Services/State/ProjectStore.cs` | Implementation with optimistic updates |
| 15 | `Services/State/AppState.cs` | Add `Projects` property |
| 16 | `Wasm/Program.cs` | Register `IProjectStore` |
| 17 | `Wasm/Layout/MainLayout.razor` | Show current project in header |
| 18 | `Wasm/Pages/ProjectDetail.razor` | Set `CurrentProject` on load |
| 19 | `Wasm/Pages/Home.razor` | Use Projects store, clear context |
| 20 | `Components/Projects/ProjectDialog.razor` | Create/edit dialog |
| 21 | `Wasm/Pages/Home.razor` | Wire Create button to ProjectDialog |
| 22 | Backend tests | Repository + handler tests |
| 23 | Frontend tests | ViewModel + Store + Dialog tests |

---

## Step Details

### 1. Database Migration

```sql
-- Create pm_projects table
CREATE TABLE pm_projects (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    description TEXT,
    key TEXT NOT NULL UNIQUE,
    status TEXT NOT NULL DEFAULT 'active' CHECK(status IN ('active', 'archived')),
    version INTEGER NOT NULL DEFAULT 1,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    created_by TEXT NOT NULL,
    updated_by TEXT NOT NULL,
    deleted_at INTEGER
);

-- Migrate from pm_work_items WHERE item_type = 'project'
-- Recreate pm_sprints, pm_swim_lanes, pm_project_members, pm_work_items with FK to pm_projects
-- Remove 'project' from pm_work_items.item_type check
```

See full migration SQL in appendix.

### 2. Rust Project Model

Add to `pm-core/src/models/mod.rs`:
```rust
mod project;
pub use project::{Project, ProjectStatus};
```

`pm-core/src/models/project.rs`:
```rust
#[derive(Debug, Clone)]
pub struct Project {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub key: String,
    pub status: ProjectStatus,
    pub version: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Uuid,
    pub updated_by: Uuid,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectStatus {
    Active,
    Archived,
}

impl ProjectStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Archived => "archived",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "active" => Some(Self::Active),
            "archived" => Some(Self::Archived),
            _ => None,
        }
    }
}
```

### 3. Rust ProjectRepository

Add to `pm-db/src/repositories/mod.rs`:
```rust
mod project_repository;
pub use project_repository::ProjectRepository;
```

`pm-db/src/repositories/project_repository.rs`:
```rust
use pm_core::models::{Project, ProjectStatus};
use sqlx::{Executor, Sqlite};
use uuid::Uuid;
use chrono::{DateTime, Utc};

pub struct ProjectRepository;

impl ProjectRepository {
    pub async fn create<'e, E>(executor: E, project: &Project) -> Result<(), DbError>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        sqlx::query!(
            r#"
            INSERT INTO pm_projects (id, title, description, key, status, version,
                created_at, updated_at, created_by, updated_by, deleted_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            project.id, project.title, project.description, project.key,
            project.status.as_str(), project.version,
            project.created_at.timestamp(), project.updated_at.timestamp(),
            project.created_by, project.updated_by,
            project.deleted_at.map(|dt| dt.timestamp())
        )
        .execute(executor)
        .await?;
        Ok(())
    }

    pub async fn get_by_id<'e, E>(executor: E, id: Uuid) -> Result<Option<Project>, DbError>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        let row = sqlx::query!(
            r#"SELECT * FROM pm_projects WHERE id = ? AND deleted_at IS NULL"#,
            id
        )
        .fetch_optional(executor)
        .await?;
        Ok(row.map(|r| row_to_project(r)))
    }

    pub async fn get_by_key<'e, E>(executor: E, key: &str) -> Result<Option<Project>, DbError>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        let row = sqlx::query!(
            r#"SELECT * FROM pm_projects WHERE key = ? AND deleted_at IS NULL"#,
            key
        )
        .fetch_optional(executor)
        .await?;
        Ok(row.map(|r| row_to_project(r)))
    }

    pub async fn get_all<'e, E>(executor: E) -> Result<Vec<Project>, DbError>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        let rows = sqlx::query!(
            r#"SELECT * FROM pm_projects WHERE deleted_at IS NULL ORDER BY title"#
        )
        .fetch_all(executor)
        .await?;
        Ok(rows.into_iter().map(|r| row_to_project(r)).collect())
    }

    pub async fn update<'e, E>(executor: E, project: &Project) -> Result<(), DbError>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        sqlx::query!(
            r#"
            UPDATE pm_projects
            SET title = ?, description = ?, status = ?, version = ?,
                updated_at = ?, updated_by = ?
            WHERE id = ? AND deleted_at IS NULL
            "#,
            project.title, project.description, project.status.as_str(),
            project.version, project.updated_at.timestamp(), project.updated_by,
            project.id
        )
        .execute(executor)
        .await?;
        Ok(())
    }

    pub async fn soft_delete<'e, E>(executor: E, id: Uuid, deleted_by: Uuid) -> Result<(), DbError>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        let now = Utc::now().timestamp();
        sqlx::query!(
            r#"UPDATE pm_projects SET deleted_at = ?, updated_by = ? WHERE id = ?"#,
            now, deleted_by, id
        )
        .execute(executor)
        .await?;
        Ok(())
    }
}

fn row_to_project(row: /* query result type */) -> Project {
    Project {
        id: Uuid::parse_str(&row.id).unwrap(),
        title: row.title,
        description: row.description,
        key: row.key,
        status: ProjectStatus::from_str(&row.status).unwrap_or(ProjectStatus::Active),
        version: row.version,
        created_at: DateTime::from_timestamp(row.created_at, 0).unwrap().with_timezone(&Utc),
        updated_at: DateTime::from_timestamp(row.updated_at, 0).unwrap().with_timezone(&Utc),
        created_by: Uuid::parse_str(&row.created_by).unwrap(),
        updated_by: Uuid::parse_str(&row.updated_by).unwrap(),
        deleted_at: row.deleted_at.map(|ts| DateTime::from_timestamp(ts, 0).unwrap().with_timezone(&Utc)),
    }
}
```

### 4. Protobuf Messages

Add to `proto/messages.proto` (use next available field numbers in Payload oneof):

```protobuf
enum ProjectStatus {
  PROJECT_STATUS_UNSPECIFIED = 0;
  PROJECT_STATUS_ACTIVE = 1;
  PROJECT_STATUS_ARCHIVED = 2;
}

message Project {
  string id = 1;
  string title = 2;
  optional string description = 3;
  string key = 4;
  ProjectStatus status = 5;
  int32 version = 6;
  int64 created_at = 7;
  int64 updated_at = 8;
  string created_by = 9;
  string updated_by = 10;
  optional int64 deleted_at = 11;
}

message CreateProjectRequest {
  string title = 1;
  optional string description = 2;
  string key = 3;
}

message UpdateProjectRequest {
  string project_id = 1;
  int32 expected_version = 2;
  optional string title = 3;
  optional string description = 4;
  optional ProjectStatus status = 5;
}

message DeleteProjectRequest {
  string project_id = 1;
  int32 expected_version = 2;
}

message ListProjectsRequest {}

message ProjectCreated {
  Project project = 1;
  string user_id = 2;
}

message ProjectUpdated {
  Project project = 1;
  repeated FieldChange changes = 2;
  string user_id = 3;
}

message ProjectDeleted {
  string project_id = 1;
  string user_id = 2;
}

message ProjectList {
  repeated Project projects = 1;
}

// Add to WebSocketMessage.payload oneof (use next available numbers):
// CreateProjectRequest create_project_request = XX;
// UpdateProjectRequest update_project_request = XX;
// DeleteProjectRequest delete_project_request = XX;
// ListProjectsRequest list_projects_request = XX;
// ProjectCreated project_created = XX;
// ProjectUpdated project_updated = XX;
// ProjectDeleted project_deleted = XX;
// ProjectList project_list = XX;
```

### 5. Rust WebSocket Handlers

Add to `pm-ws/src/handlers/mod.rs`:
```rust
mod project;
pub use project::*;
```

Update `pm-ws/src/handlers/dispatcher.rs`:
```rust
Some(Payload::CreateProjectRequest(req)) => project::handle_create(req, ctx).await,
Some(Payload::UpdateProjectRequest(req)) => project::handle_update(req, ctx).await,
Some(Payload::DeleteProjectRequest(req)) => project::handle_delete(req, ctx).await,
Some(Payload::ListProjectsRequest(req)) => project::handle_list(req, ctx).await,
```

Update `pm-ws/src/handlers/response_builder.rs`:
```rust
pub fn build_project_created_response(message_id: &str, project: &Project, actor_id: Uuid) -> WebSocketMessage;
pub fn build_project_updated_response(message_id: &str, project: &Project, changes: &[FieldChange], actor_id: Uuid) -> WebSocketMessage;
pub fn build_project_deleted_response(message_id: &str, project_id: Uuid, actor_id: Uuid) -> WebSocketMessage;
pub fn build_project_list_response(message_id: &str, projects: &[Project]) -> WebSocketMessage;
```

`pm-ws/src/handlers/project.rs` - follow `work_item.rs` pattern:

```rust
pub async fn handle_create(req: CreateProjectRequest, ctx: HandlerContext) -> WsErrorResult<WebSocketMessage> {
    // 1. Validate input
    validate_title(&req.title, 1, 200)?;
    validate_key(&req.key)?;  // alphanumeric + underscore, 2-20 chars
    if let Some(ref desc) = req.description {
        validate_description(desc, 10000)?;
    }

    // 2. Check idempotency
    if let Some(cached) = db_read(&ctx, "check_idempotency", || check_idempotency(&ctx.pool, &ctx.message_id)).await? {
        return Ok(cached);
    }

    // 3. Check key uniqueness
    let existing = db_read(&ctx, "get_by_key", || ProjectRepository::get_by_key(&ctx.pool, &req.key)).await?;
    if existing.is_some() {
        return Err(WsError::ValidationError { message: "Project key already exists".into(), field: Some("key".into()) });
    }

    // 4. Build project
    let project = Project {
        id: Uuid::new_v4(),
        title: sanitize_string(&req.title),
        description: req.description.as_ref().map(|d| sanitize_string(d)),
        key: req.key.to_uppercase(),
        status: ProjectStatus::Active,
        version: 1,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        created_by: ctx.user_id,
        updated_by: ctx.user_id,
        deleted_at: None,
    };

    // 5. Transaction: create + activity log
    db_write(&ctx, "create_project_tx", || async {
        let mut tx = ctx.pool.begin().await?;
        ProjectRepository::create(&mut *tx, &project).await?;
        let activity = ActivityLog::created("project", project.id, ctx.user_id);
        ActivityLogRepository::create(&mut *tx, &activity).await?;
        tx.commit().await?;
        Ok::<_, WsError>(())
    }).await?;

    // 6. Build response + store idempotency
    let response = build_project_created_response(&ctx.message_id, &project, ctx.user_id);
    store_idempotency(&ctx.pool, &ctx.message_id, &response).await;

    Ok(response)
}

pub async fn handle_update(req: UpdateProjectRequest, ctx: HandlerContext) -> WsErrorResult<WebSocketMessage> {
    // 1. Parse + fetch existing
    let project_id = parse_uuid(&req.project_id, "project_id")?;
    let mut project = db_read(&ctx, "get_project", || ProjectRepository::get_by_id(&ctx.pool, project_id)).await?
        .ok_or_else(|| WsError::NotFound { entity: "Project".into(), id: project_id.to_string() })?;

    // 2. Optimistic locking
    if project.version != req.expected_version {
        return Err(WsError::ConflictError { current_version: project.version });
    }

    // 3. Track changes + apply updates
    let changes = track_project_changes(&project, &req);
    if changes.is_empty() {
        return Ok(build_project_updated_response(&ctx.message_id, &project, &[], ctx.user_id));
    }
    apply_project_updates(&mut project, &req)?;
    project.updated_at = Utc::now();
    project.updated_by = ctx.user_id;
    project.version += 1;

    // 4. Transaction
    db_write(&ctx, "update_project_tx", || async {
        let mut tx = ctx.pool.begin().await?;
        ProjectRepository::update(&mut *tx, &project).await?;
        let activity = ActivityLog::updated("project", project.id, ctx.user_id, &changes);
        ActivityLogRepository::create(&mut *tx, &activity).await?;
        tx.commit().await?;
        Ok::<_, WsError>(())
    }).await?;

    Ok(build_project_updated_response(&ctx.message_id, &project, &changes, ctx.user_id))
}

pub async fn handle_delete(req: DeleteProjectRequest, ctx: HandlerContext) -> WsErrorResult<WebSocketMessage> {
    let project_id = parse_uuid(&req.project_id, "project_id")?;
    let project = db_read(&ctx, "get_project", || ProjectRepository::get_by_id(&ctx.pool, project_id)).await?
        .ok_or_else(|| WsError::NotFound { entity: "Project".into(), id: project_id.to_string() })?;

    // Optimistic locking
    if project.version != req.expected_version {
        return Err(WsError::ConflictError { current_version: project.version });
    }

    // Check no work items exist
    let items = db_read(&ctx, "get_work_items", || WorkItemRepository::find_by_project(&ctx.pool, project_id)).await?;
    if !items.is_empty() {
        return Err(WsError::DeleteBlocked { message: format!("Project has {} work item(s)", items.len()) });
    }

    db_write(&ctx, "delete_project_tx", || async {
        let mut tx = ctx.pool.begin().await?;
        ProjectRepository::soft_delete(&mut *tx, project_id, ctx.user_id).await?;
        let activity = ActivityLog::deleted("project", project_id, ctx.user_id);
        ActivityLogRepository::create(&mut *tx, &activity).await?;
        tx.commit().await?;
        Ok::<_, WsError>(())
    }).await?;

    Ok(build_project_deleted_response(&ctx.message_id, project_id, ctx.user_id))
}

pub async fn handle_list(_req: ListProjectsRequest, ctx: HandlerContext) -> WsErrorResult<WebSocketMessage> {
    let projects = db_read(&ctx, "get_all_projects", || ProjectRepository::get_all(&ctx.pool)).await?;
    Ok(build_project_list_response(&ctx.message_id, &projects))
}
```

**Validation helpers** (add to `pm-ws/src/handlers/validation.rs`):
```rust
pub fn validate_key(key: &str) -> WsErrorResult<()> {
    if key.len() < 2 || key.len() > 20 {
        return Err(WsError::ValidationError { message: "Key must be 2-20 characters".into(), field: Some("key".into()) });
    }
    if !key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(WsError::ValidationError { message: "Key must be alphanumeric with underscores only".into(), field: Some("key".into()) });
    }
    Ok(())
}
```

### 6. C# ProjectStatus

```csharp
public enum ProjectStatus { Active = 0, Archived = 1 }
```

### 7. C# Project Model

```csharp
public sealed record Project
{
    public required Guid Id { get; init; }
    public required string Title { get; init; }
    public string? Description { get; init; }
    public required string Key { get; init; }
    public required ProjectStatus Status { get; init; }
    public required int Version { get; init; }
    public required DateTime CreatedAt { get; init; }
    public required DateTime UpdatedAt { get; init; }
    public required Guid CreatedBy { get; init; }
    public required Guid UpdatedBy { get; init; }
    public DateTime? DeletedAt { get; init; }
}
```

### 8-9. C# DTOs

```csharp
public sealed record CreateProjectRequest
{
    public required string Title { get; init; }
    public string? Description { get; init; }
    public required string Key { get; init; }
}

public sealed record UpdateProjectRequest
{
    public required Guid ProjectId { get; init; }
    public required int ExpectedVersion { get; init; }
    public string? Title { get; init; }
    public string? Description { get; init; }
    public ProjectStatus? Status { get; init; }
}
```

### 10. IWebSocketClient Extensions

```csharp
// Events
event Action<Project>? OnProjectCreated;
event Action<Project, IReadOnlyList<FieldChange>>? OnProjectUpdated;
event Action<Guid>? OnProjectDeleted;

// Operations
Task<Project> CreateProjectAsync(CreateProjectRequest request, CancellationToken ct = default);
Task<Project> UpdateProjectAsync(UpdateProjectRequest request, CancellationToken ct = default);
Task DeleteProjectAsync(Guid projectId, int expectedVersion, CancellationToken ct = default);
Task<IReadOnlyList<Project>> GetProjectsAsync(CancellationToken ct = default);
```

### 11. WebSocketClient Implementation

Follow existing WorkItem pattern for protobuf serialization, request/response correlation.

Update `Services/WebSocket/ProtoConverter.cs`:
```csharp
public static Project ToDomain(Pm.Project proto) => new()
{
    Id = Guid.Parse(proto.Id),
    Title = proto.Title,
    Description = proto.HasDescription ? proto.Description : null,
    Key = proto.Key,
    Status = (ProjectStatus)proto.Status,
    Version = proto.Version,
    CreatedAt = DateTimeOffset.FromUnixTimeSeconds(proto.CreatedAt).UtcDateTime,
    UpdatedAt = DateTimeOffset.FromUnixTimeSeconds(proto.UpdatedAt).UtcDateTime,
    CreatedBy = Guid.Parse(proto.CreatedBy),
    UpdatedBy = Guid.Parse(proto.UpdatedBy),
    DeletedAt = proto.HasDeletedAt ? DateTimeOffset.FromUnixTimeSeconds(proto.DeletedAt).UtcDateTime : null,
};

public static Pm.CreateProjectRequest ToProto(CreateProjectRequest req) => new()
{
    Title = req.Title,
    Description = req.Description ?? "",
    Key = req.Key,
};

public static Pm.UpdateProjectRequest ToProto(UpdateProjectRequest req)
{
    var proto = new Pm.UpdateProjectRequest
    {
        ProjectId = req.ProjectId.ToString(),
        ExpectedVersion = req.ExpectedVersion,
    };
    if (req.Title is not null) proto.Title = req.Title;
    if (req.Description is not null) proto.Description = req.Description;
    if (req.Status.HasValue) proto.Status = (Pm.ProjectStatus)req.Status.Value;
    return proto;
}

public static Pm.DeleteProjectRequest ToProto(Guid projectId, int expectedVersion) => new()
{
    ProjectId = projectId.ToString(),
    ExpectedVersion = expectedVersion,
};
```

Add to `WebSocketClient.cs`:
```csharp
public event Action<Project>? OnProjectCreated;
public event Action<Project, IReadOnlyList<FieldChange>>? OnProjectUpdated;
public event Action<Guid>? OnProjectDeleted;

public async Task<Project> CreateProjectAsync(CreateProjectRequest request, CancellationToken ct = default)
{
    var message = new Pm.WebSocketMessage
    {
        MessageId = Guid.NewGuid().ToString(),
        Timestamp = DateTimeOffset.UtcNow.ToUnixTimeSeconds(),
        CreateProjectRequest = ProtoConverter.ToProto(request),
    };
    var response = await SendRequestAsync(message, ct);
    if (response.PayloadCase == Pm.WebSocketMessage.PayloadOneofCase.Error)
        throw new ServerRejectedException(response.Error.Code, response.Error.Message);
    return ProtoConverter.ToDomain(response.ProjectCreated.Project);
}

public async Task<Project> UpdateProjectAsync(UpdateProjectRequest request, CancellationToken ct = default)
{
    var message = new Pm.WebSocketMessage
    {
        MessageId = Guid.NewGuid().ToString(),
        Timestamp = DateTimeOffset.UtcNow.ToUnixTimeSeconds(),
        UpdateProjectRequest = ProtoConverter.ToProto(request),
    };
    var response = await SendRequestAsync(message, ct);
    if (response.PayloadCase == Pm.WebSocketMessage.PayloadOneofCase.Error)
        throw new ServerRejectedException(response.Error.Code, response.Error.Message);
    return ProtoConverter.ToDomain(response.ProjectUpdated.Project);
}

public async Task DeleteProjectAsync(Guid projectId, int expectedVersion, CancellationToken ct = default)
{
    var message = new Pm.WebSocketMessage
    {
        MessageId = Guid.NewGuid().ToString(),
        Timestamp = DateTimeOffset.UtcNow.ToUnixTimeSeconds(),
        DeleteProjectRequest = ProtoConverter.ToProto(projectId, expectedVersion),
    };
    var response = await SendRequestAsync(message, ct);
    if (response.PayloadCase == Pm.WebSocketMessage.PayloadOneofCase.Error)
        throw new ServerRejectedException(response.Error.Code, response.Error.Message);
}

public async Task<IReadOnlyList<Project>> GetProjectsAsync(CancellationToken ct = default)
{
    var message = new Pm.WebSocketMessage
    {
        MessageId = Guid.NewGuid().ToString(),
        Timestamp = DateTimeOffset.UtcNow.ToUnixTimeSeconds(),
        ListProjectsRequest = new Pm.ListProjectsRequest(),
    };
    var response = await SendRequestAsync(message, ct);
    if (response.PayloadCase == Pm.WebSocketMessage.PayloadOneofCase.Error)
        throw new ServerRejectedException(response.Error.Code, response.Error.Message);
    return response.ProjectList.Projects.Select(ProtoConverter.ToDomain).ToList();
}

// In HandleBroadcastEvent:
case Pm.WebSocketMessage.PayloadOneofCase.ProjectCreated:
    OnProjectCreated?.Invoke(ProtoConverter.ToDomain(message.ProjectCreated.Project));
    break;
case Pm.WebSocketMessage.PayloadOneofCase.ProjectUpdated:
    var changes = message.ProjectUpdated.Changes.Select(c => new FieldChange(c.FieldName, c.OldValue, c.NewValue)).ToList();
    OnProjectUpdated?.Invoke(ProtoConverter.ToDomain(message.ProjectUpdated.Project), changes);
    break;
case Pm.WebSocketMessage.PayloadOneofCase.ProjectDeleted:
    OnProjectDeleted?.Invoke(Guid.Parse(message.ProjectDeleted.ProjectId));
    break;
```

### 12. ProjectViewModel

```csharp
public sealed class ProjectViewModel : IViewModel<Project>, IEquatable<ProjectViewModel>
{
    public ProjectViewModel(Project model, bool isPendingSync = false)
    {
        Model = model;
        IsPendingSync = isPendingSync;
    }

    public Project Model { get; }
    public bool IsPendingSync { get; }

    // Delegated properties
    public Guid Id => Model.Id;
    public string Title => Model.Title;
    public string? Description => Model.Description;
    public string Key => Model.Key;
    public ProjectStatus Status => Model.Status;
    public int Version => Model.Version;
    public DateTime CreatedAt => Model.CreatedAt;
    public DateTime UpdatedAt => Model.UpdatedAt;
    public Guid CreatedBy => Model.CreatedBy;
    public Guid UpdatedBy => Model.UpdatedBy;
    public DateTime? DeletedAt => Model.DeletedAt;

    // Display helpers
    public string StatusDisplayName => Status switch
    {
        ProjectStatus.Active => "Active",
        ProjectStatus.Archived => "Archived",
        _ => Status.ToString()
    };

    public bool IsArchived => Status == ProjectStatus.Archived;

    // Equality
    public bool Equals(ProjectViewModel? other) =>
        other is not null && Id == other.Id && Version == other.Version && IsPendingSync == other.IsPendingSync;

    public override bool Equals(object? obj) => Equals(obj as ProjectViewModel);
    public override int GetHashCode() => HashCode.Combine(Id, Version, IsPendingSync);
}
```

Update `Core/ViewModels/ViewModelFactory.cs`:
```csharp
private readonly IProjectStore _projectStore;

public ViewModelFactory(IWorkItemStore workItemStore, ISprintStore sprintStore, IProjectStore projectStore)
{
    _workItemStore = workItemStore;
    _sprintStore = sprintStore;
    _projectStore = projectStore;
}

public ProjectViewModel Create(Project project) =>
    new(project, _projectStore.IsPending(project.Id));
```

Update `Core/Models/WorkItemType.cs` - remove Project:
```csharp
public enum WorkItemType
{
    // Project = 1,  // REMOVED - Projects are now separate entity
    Epic = 2,
    Story = 3,
    Task = 4
}

// Update helper methods to remove Project references
public static class WorkItemTypeExtensions
{
    public static bool CanHaveParent(this WorkItemType type) => true;  // All can have parents now

    public static IReadOnlyList<WorkItemType> AllowedChildTypes(this WorkItemType type) => type switch
    {
        WorkItemType.Epic => [WorkItemType.Story, WorkItemType.Task],
        WorkItemType.Story => [WorkItemType.Task],
        WorkItemType.Task => [],
        _ => []
    };
}
```

### 13. IProjectStore (CRITICAL: includes async CRUD)

```csharp
public interface IProjectStore
{
    IReadOnlyList<ProjectViewModel> Projects { get; }
    ProjectViewModel? CurrentProject { get; }
    bool IsLoaded { get; }

    event Action? OnCurrentProjectChanged;
    event Action? OnProjectsChanged;

    void SetCurrentProject(Guid projectId);
    void ClearCurrentProject();

    // Async CRUD - MUST have these
    Task<Project> CreateAsync(CreateProjectRequest request, CancellationToken ct = default);
    Task<Project> UpdateAsync(UpdateProjectRequest request, CancellationToken ct = default);
    Task DeleteAsync(Guid id, int expectedVersion, CancellationToken ct = default);
    Task RefreshAsync(CancellationToken ct = default);
    bool IsPending(Guid id);

    // Helper to get project by ID (for version lookup before delete)
    ProjectViewModel? GetById(Guid id);
}
```

### 14. ProjectStore Implementation

- Inject `IWebSocketClient`
- Wire `OnProjectCreated`, `OnProjectUpdated`, `OnProjectDeleted` events
- Implement optimistic updates pattern
- See appendix for full code

### 15. AppState Integration

```csharp
// Add to constructor
public AppState(..., IProjectStore projects, ...)
{
    Projects = projects;
    projects.OnCurrentProjectChanged += () => OnStateChanged?.Invoke();
    projects.OnProjectsChanged += () => OnStateChanged?.Invoke();
}

public IProjectStore Projects { get; }

public Task LoadProjectsAsync(CancellationToken ct = default)
    => Projects.RefreshAsync(ct);
```

### 16. DI Registration

```csharp
builder.Services.AddSingleton<IProjectStore, ProjectStore>();
```

### 17. MainLayout Header

```razor
<RadzenText TextStyle="TextStyle.H5">
    Agile Board
    @if (AppState.Projects.CurrentProject is not null)
    {
        <span class="project-context"> / @AppState.Projects.CurrentProject.Title</span>
    }
</RadzenText>
```

### 18. ProjectDetail Navigation

```csharp
AppState.Projects.SetCurrentProject(ProjectId);
```

### 19. Home.razor Updates

```csharp
// OnInitializedAsync
AppState.Projects.ClearCurrentProject();
await AppState.LoadProjectsAsync();
_projects = AppState.Projects.Projects.ToList();
```

### 20. ProjectDialog

`Components/Projects/ProjectDialog.razor`:
```razor
@inject AppState AppState
@inject DialogService DialogService

<RadzenTemplateForm TItem="ProjectFormModel" Data="@_model" Submit="@OnSubmit">
    <div class="form-group">
        <RadzenLabel Text="Title" Component="Title" />
        <RadzenTextBox Name="Title" @bind-Value="_model.Title" MaxLength="200"
                       Change="@OnTitleChanged" Style="width: 100%" />
        <RadzenRequiredValidator Component="Title" Text="Title is required" />
    </div>

    <div class="form-group">
        <RadzenLabel Text="Key" Component="Key" />
        <RadzenTextBox Name="Key" @bind-Value="_model.Key" MaxLength="20"
                       Disabled="@IsEditMode" Style="width: 100%"
                       Placeholder="AUTO-GENERATED" />
        <RadzenRequiredValidator Component="Key" Text="Key is required" />
        <RadzenRegexValidator Component="Key" Pattern="^[A-Z0-9_]{2,20}$"
                              Text="Key must be 2-20 uppercase alphanumeric characters" />
    </div>

    <div class="form-group">
        <RadzenLabel Text="Description" Component="Description" />
        <RadzenTextArea Name="Description" @bind-Value="_model.Description"
                        MaxLength="10000" Rows="4" Style="width: 100%" />
    </div>

    @if (IsEditMode)
    {
        <div class="form-group">
            <RadzenLabel Text="Status" Component="Status" />
            <RadzenDropDown Name="Status" @bind-Value="_model.Status"
                            Data="@_statusOptions" TextProperty="Text" ValueProperty="Value"
                            Style="width: 100%" />
        </div>
    }

    <div class="dialog-buttons">
        <RadzenButton ButtonType="ButtonType.Submit" Text="@(IsEditMode ? "Save" : "Create")"
                      ButtonStyle="ButtonStyle.Primary" Disabled="@_isSubmitting" />
        <RadzenButton Text="Cancel" ButtonStyle="ButtonStyle.Light"
                      Click="@(() => DialogService.Close(null))" />
    </div>
</RadzenTemplateForm>

@code {
    [Parameter] public ProjectViewModel? Project { get; set; }

    private ProjectFormModel _model = new();
    private bool _isSubmitting;
    private bool IsEditMode => Project is not null;

    private readonly record struct StatusOption(string Text, ProjectStatus Value);
    private static readonly StatusOption[] _statusOptions =
    [
        new("Active", ProjectStatus.Active),
        new("Archived", ProjectStatus.Archived),
    ];

    protected override void OnInitialized()
    {
        if (Project is not null)
        {
            _model = new()
            {
                Title = Project.Title,
                Key = Project.Key,
                Description = Project.Description,
                Status = Project.Status,
            };
        }
    }

    private void OnTitleChanged(string title)
    {
        if (IsEditMode) return; // Don't auto-generate key on edit
        _model.Key = GenerateKey(title);
    }

    private static string GenerateKey(string title) =>
        new string(title
            .ToUpperInvariant()
            .Where(c => char.IsLetterOrDigit(c))
            .Take(10)
            .ToArray());

    private async Task OnSubmit()
    {
        _isSubmitting = true;
        try
        {
            if (IsEditMode)
            {
                var request = new UpdateProjectRequest
                {
                    ProjectId = Project!.Id,
                    ExpectedVersion = Project.Version,
                    Title = _model.Title != Project.Title ? _model.Title : null,
                    Description = _model.Description != Project.Description ? _model.Description : null,
                    Status = _model.Status != Project.Status ? _model.Status : null,
                };
                var updated = await AppState.Projects.UpdateAsync(request);
                DialogService.Close(updated);
            }
            else
            {
                var request = new CreateProjectRequest
                {
                    Title = _model.Title,
                    Description = _model.Description,
                    Key = _model.Key,
                };
                var created = await AppState.Projects.CreateAsync(request);
                DialogService.Close(created);
            }
        }
        catch (ServerRejectedException ex)
        {
            // Show error toast
        }
        finally
        {
            _isSubmitting = false;
        }
    }

    private sealed class ProjectFormModel
    {
        public string Title { get; set; } = "";
        public string Key { get; set; } = "";
        public string? Description { get; set; }
        public ProjectStatus Status { get; set; } = ProjectStatus.Active;
    }
}
```

### 21. Wire Create Button

Replace `WorkItemDialog` with `ProjectDialog` for "Create Project" button.

---

## Tests

### Backend Tests

**`pm-db/tests/project_repository_tests.rs`**:
```rust
#[sqlx::test]
async fn test_create_project(pool: SqlitePool) {
    let project = Project { id: Uuid::new_v4(), title: "Test".into(), key: "TEST".into(), ... };
    ProjectRepository::create(&pool, &project).await.unwrap();
    let found = ProjectRepository::get_by_id(&pool, project.id).await.unwrap();
    assert_eq!(found.unwrap().title, "Test");
}

#[sqlx::test]
async fn test_get_by_key(pool: SqlitePool) { ... }

#[sqlx::test]
async fn test_key_uniqueness(pool: SqlitePool) {
    // Should fail on duplicate key
}

#[sqlx::test]
async fn test_soft_delete(pool: SqlitePool) {
    // deleted_at set, not returned by get_all
}

#[sqlx::test]
async fn test_update_increments_version(pool: SqlitePool) { ... }
```

**`pm-ws/tests/project_handler_tests.rs`**:
```rust
#[tokio::test]
async fn test_create_project_success() { ... }

#[tokio::test]
async fn test_create_project_duplicate_key_fails() { ... }

#[tokio::test]
async fn test_update_project_version_conflict() { ... }

#[tokio::test]
async fn test_delete_project_with_work_items_blocked() { ... }

#[tokio::test]
async fn test_list_projects_excludes_deleted() { ... }

#[tokio::test]
async fn test_key_validation_alphanumeric_only() { ... }

#[tokio::test]
async fn test_title_validation_length() { ... }
```

### Frontend Tests

**`ProjectManagement.Core.Tests/ViewModels/ProjectViewModelTests.cs`**:
```csharp
[Fact]
public void Properties_DelegateToModel() { ... }

[Fact]
public void Equality_BasedOnIdVersionPending() { ... }

[Fact]
public void StatusDisplayName_ReturnsReadableString() { ... }
```

**`ProjectManagement.Services.Tests/State/ProjectStoreTests.cs`**:
```csharp
[Fact]
public async Task CreateAsync_AddsToStore_AndCallsClient() { ... }

[Fact]
public async Task CreateAsync_RollsBackOnFailure() { ... }

[Fact]
public async Task HandleProjectCreated_SkipsPendingItems() { ... }

[Fact]
public void Projects_ExcludesDeleted() { ... }

[Fact]
public void CurrentProject_ReturnsNullWhenNotSet() { ... }

[Fact]
public void SetCurrentProject_FiresEvent() { ... }
```

**`ProjectManagement.Components.Tests/Projects/ProjectDialogTests.cs`**:
```csharp
[Fact]
public void Create_ShowsEmptyForm() { ... }

[Fact]
public void Edit_PopulatesFromProject() { ... }

[Fact]
public void KeyField_DisabledOnEdit() { ... }

[Fact]
public void Submit_CallsStoreCreate() { ... }

[Fact]
public void Validation_RequiresTitleAndKey() { ... }
```

---

## Verification

**Backend**:
- [ ] `cargo build --workspace` succeeds
- [ ] `cargo test --workspace` passes (including new project tests)
- [ ] `pm_projects` table exists with correct schema
- [ ] Existing projects migrated from `pm_work_items`
- [ ] Backend handlers respond to Create/Update/Delete/List

**Frontend**:
- [ ] `dotnet build frontend/ProjectManagement.sln` succeeds
- [ ] `dotnet test` passes (including new project tests)
- [ ] Home page loads projects from ProjectStore
- [ ] "Create Project" opens ProjectDialog (not WorkItemDialog)
- [ ] Header shows "Agile Board / ProjectName" when in project context
- [ ] Header shows only "Agile Board" on Home page

**End-to-End**:
- [ ] Create project via UI → appears in list
- [ ] Edit project title → updates in header
- [ ] Archive project → removed from active list
- [ ] Delete project with work items → blocked with error message

---

## Appendix: Full Migration SQL

```sql
-- ============================================================
-- Migration: Create pm_projects table and migrate data
-- ============================================================

-- Step 1: Create pm_projects table
CREATE TABLE pm_projects (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    description TEXT,
    key TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'active' CHECK(status IN ('active', 'archived')),
    version INTEGER NOT NULL DEFAULT 1,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    created_by TEXT NOT NULL,
    updated_by TEXT NOT NULL,
    deleted_at INTEGER,
    UNIQUE(key)
);

-- Step 2: Migrate existing projects from pm_work_items
-- Generate key from title (uppercase, no spaces, max 10 chars) + first 4 chars of ID for uniqueness
INSERT INTO pm_projects (id, title, description, key, status, version, created_at, updated_at, created_by, updated_by, deleted_at)
SELECT
    id,
    title,
    description,
    UPPER(SUBSTR(REPLACE(REPLACE(title, ' ', ''), '-', ''), 1, 10)) || '_' || UPPER(SUBSTR(id, 1, 4)),
    'active',
    COALESCE(version, 1),
    created_at,
    updated_at,
    created_by,
    updated_by,
    deleted_at
FROM pm_work_items
WHERE item_type = 'project';

-- Step 3: Recreate pm_sprints with FK to pm_projects
CREATE TABLE pm_sprints_new (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    name TEXT NOT NULL,
    goal TEXT,
    start_date INTEGER,
    end_date INTEGER,
    status TEXT NOT NULL DEFAULT 'planning' CHECK(status IN ('planning', 'active', 'completed', 'cancelled')),
    velocity INTEGER,
    version INTEGER NOT NULL DEFAULT 1,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    created_by TEXT NOT NULL,
    updated_by TEXT NOT NULL,
    deleted_at INTEGER,
    FOREIGN KEY (project_id) REFERENCES pm_projects(id) ON DELETE CASCADE
);
INSERT INTO pm_sprints_new SELECT * FROM pm_sprints;
DROP TABLE pm_sprints;
ALTER TABLE pm_sprints_new RENAME TO pm_sprints;
CREATE INDEX idx_pm_sprints_project ON pm_sprints(project_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_sprints_status ON pm_sprints(status) WHERE deleted_at IS NULL;

-- Step 4: Recreate pm_swim_lanes with FK to pm_projects
CREATE TABLE pm_swim_lanes_new (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    name TEXT NOT NULL,
    position INTEGER NOT NULL DEFAULT 0,
    wip_limit INTEGER,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    deleted_at INTEGER,
    FOREIGN KEY (project_id) REFERENCES pm_projects(id) ON DELETE CASCADE
);
INSERT INTO pm_swim_lanes_new SELECT * FROM pm_swim_lanes;
DROP TABLE pm_swim_lanes;
ALTER TABLE pm_swim_lanes_new RENAME TO pm_swim_lanes;
CREATE INDEX idx_pm_swim_lanes_project ON pm_swim_lanes(project_id) WHERE deleted_at IS NULL;

-- Step 5: Recreate pm_project_members with FK to pm_projects
CREATE TABLE pm_project_members_new (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    role TEXT NOT NULL CHECK(role IN ('viewer', 'editor', 'admin')),
    created_at INTEGER NOT NULL,
    FOREIGN KEY (project_id) REFERENCES pm_projects(id) ON DELETE CASCADE,
    UNIQUE(project_id, user_id)
);
INSERT INTO pm_project_members_new SELECT * FROM pm_project_members;
DROP TABLE pm_project_members;
ALTER TABLE pm_project_members_new RENAME TO pm_project_members;
CREATE INDEX idx_pm_project_members_project ON pm_project_members(project_id);
CREATE INDEX idx_pm_project_members_user ON pm_project_members(user_id);

-- Step 6: Recreate pm_work_items (remove 'project' type, FK to pm_projects)
CREATE TABLE pm_work_items_new (
    id TEXT PRIMARY KEY,
    item_type TEXT NOT NULL CHECK(item_type IN ('epic', 'story', 'task')),
    parent_id TEXT,
    project_id TEXT NOT NULL,
    position INTEGER NOT NULL DEFAULT 0,
    title TEXT NOT NULL,
    description TEXT,
    status TEXT NOT NULL DEFAULT 'backlog' CHECK(status IN ('backlog', 'todo', 'in_progress', 'review', 'done')),
    priority TEXT NOT NULL DEFAULT 'medium' CHECK(priority IN ('critical', 'high', 'medium', 'low')),
    story_points INTEGER,
    assignee_id TEXT,
    sprint_id TEXT,
    version INTEGER NOT NULL DEFAULT 1,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    created_by TEXT NOT NULL,
    updated_by TEXT NOT NULL,
    deleted_at INTEGER,
    FOREIGN KEY (project_id) REFERENCES pm_projects(id) ON DELETE CASCADE,
    FOREIGN KEY (parent_id) REFERENCES pm_work_items_new(id) ON DELETE SET NULL,
    FOREIGN KEY (sprint_id) REFERENCES pm_sprints(id) ON DELETE SET NULL
);
INSERT INTO pm_work_items_new
SELECT * FROM pm_work_items WHERE item_type != 'project';
DROP TABLE pm_work_items;
ALTER TABLE pm_work_items_new RENAME TO pm_work_items;
CREATE INDEX idx_pm_work_items_project ON pm_work_items(project_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_work_items_parent ON pm_work_items(parent_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_work_items_sprint ON pm_work_items(sprint_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_work_items_status ON pm_work_items(status) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_work_items_assignee ON pm_work_items(assignee_id) WHERE deleted_at IS NULL;

-- Step 7: Create indexes on pm_projects
CREATE INDEX idx_pm_projects_status ON pm_projects(status) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_projects_key ON pm_projects(key) WHERE deleted_at IS NULL;
```

**Note**: SQLite does not support transactional DDL fully. If migration fails partway, manual cleanup may be required. Consider backing up `.pm/data.db` before running.

---

## Appendix: ProjectStore Implementation

```csharp
public sealed class ProjectStore : IProjectStore, IDisposable
{
    private readonly IWebSocketClient _client;
    private readonly ILogger<ProjectStore> _logger;
    private readonly ConcurrentDictionary<Guid, Project> _projects = new();
    private readonly ConcurrentDictionary<Guid, OptimisticUpdate<Project>> _pendingUpdates = new();
    private Guid? _currentProjectId;
    private bool _disposed;

    public ProjectStore(IWebSocketClient client, ILogger<ProjectStore> logger)
    {
        _client = client;
        _logger = logger;
        _client.OnProjectCreated += HandleProjectCreated;
        _client.OnProjectUpdated += HandleProjectUpdated;
        _client.OnProjectDeleted += HandleProjectDeleted;
    }

    public IReadOnlyList<ProjectViewModel> Projects => _projects.Values
        .Where(p => p.DeletedAt is null)
        .OrderBy(p => p.Title)
        .Select(p => new ProjectViewModel(p, _pendingUpdates.ContainsKey(p.Id)))
        .ToList();

    public ProjectViewModel? CurrentProject => _currentProjectId.HasValue
        && _projects.TryGetValue(_currentProjectId.Value, out var p)
        ? new ProjectViewModel(p, _pendingUpdates.ContainsKey(p.Id))
        : null;

    public bool IsLoaded { get; private set; }
    public event Action? OnCurrentProjectChanged;
    public event Action? OnProjectsChanged;

    public ProjectViewModel? GetById(Guid id) =>
        _projects.TryGetValue(id, out var p) ? new ProjectViewModel(p, _pendingUpdates.ContainsKey(id)) : null;

    public void SetCurrentProject(Guid projectId)
    {
        if (_currentProjectId == projectId) return;
        _currentProjectId = projectId;
        NotifyCurrentProjectChanged();
    }

    public void ClearCurrentProject()
    {
        if (_currentProjectId is null) return;
        _currentProjectId = null;
        NotifyCurrentProjectChanged();
    }

    public async Task<Project> CreateAsync(CreateProjectRequest request, CancellationToken ct)
    {
        // Create optimistic project
        var tempId = Guid.NewGuid();
        var optimistic = new Project
        {
            Id = tempId,
            Title = request.Title,
            Description = request.Description,
            Key = request.Key.ToUpperInvariant(),
            Status = ProjectStatus.Active,
            Version = 1,
            CreatedAt = DateTime.UtcNow,
            UpdatedAt = DateTime.UtcNow,
            CreatedBy = Guid.Empty, // Unknown until server responds
            UpdatedBy = Guid.Empty,
        };

        // Add optimistically
        _projects[tempId] = optimistic;
        _pendingUpdates[tempId] = new OptimisticUpdate<Project>(tempId, null, optimistic);
        NotifyProjectsChanged();

        try
        {
            var created = await _client.CreateProjectAsync(request, ct);

            // Replace temp with real
            _projects.TryRemove(tempId, out _);
            _pendingUpdates.TryRemove(tempId, out _);
            _projects[created.Id] = created;
            NotifyProjectsChanged();

            return created;
        }
        catch
        {
            // Rollback
            _projects.TryRemove(tempId, out _);
            _pendingUpdates.TryRemove(tempId, out _);
            NotifyProjectsChanged();
            throw;
        }
    }

    public async Task<Project> UpdateAsync(UpdateProjectRequest request, CancellationToken ct)
    {
        if (!_projects.TryGetValue(request.ProjectId, out var current))
            throw new InvalidOperationException($"Project {request.ProjectId} not found");

        // Create optimistic version
        var optimistic = current with
        {
            Title = request.Title ?? current.Title,
            Description = request.Description ?? current.Description,
            Status = request.Status ?? current.Status,
            Version = current.Version + 1,
            UpdatedAt = DateTime.UtcNow,
        };

        _projects[request.ProjectId] = optimistic;
        _pendingUpdates[request.ProjectId] = new OptimisticUpdate<Project>(request.ProjectId, current, optimistic);
        NotifyProjectsChanged();

        try
        {
            var updated = await _client.UpdateProjectAsync(request, ct);
            _projects[updated.Id] = updated;
            _pendingUpdates.TryRemove(request.ProjectId, out _);
            NotifyProjectsChanged();
            return updated;
        }
        catch
        {
            // Rollback
            _projects[request.ProjectId] = current;
            _pendingUpdates.TryRemove(request.ProjectId, out _);
            NotifyProjectsChanged();
            throw;
        }
    }

    public async Task DeleteAsync(Guid id, int expectedVersion, CancellationToken ct)
    {
        if (!_projects.TryGetValue(id, out var current))
            throw new InvalidOperationException($"Project {id} not found");

        // Optimistic delete (mark as deleted)
        var optimistic = current with { DeletedAt = DateTime.UtcNow };
        _projects[id] = optimistic;
        _pendingUpdates[id] = new OptimisticUpdate<Project>(id, current, optimistic);
        NotifyProjectsChanged();

        try
        {
            await _client.DeleteProjectAsync(id, expectedVersion, ct);
            _projects.TryRemove(id, out _);
            _pendingUpdates.TryRemove(id, out _);

            // Clear current if deleted
            if (_currentProjectId == id)
            {
                _currentProjectId = null;
                NotifyCurrentProjectChanged();
            }
            NotifyProjectsChanged();
        }
        catch
        {
            // Rollback
            _projects[id] = current;
            _pendingUpdates.TryRemove(id, out _);
            NotifyProjectsChanged();
            throw;
        }
    }

    public async Task RefreshAsync(CancellationToken ct)
    {
        var projects = await _client.GetProjectsAsync(ct);
        _projects.Clear();
        foreach (var p in projects)
            _projects[p.Id] = p;
        IsLoaded = true;
        NotifyProjectsChanged();
    }

    public bool IsPending(Guid id) => _pendingUpdates.ContainsKey(id);

    private void HandleProjectCreated(Project p)
    {
        if (_pendingUpdates.ContainsKey(p.Id)) return; // Skip if we created it
        _projects[p.Id] = p;
        NotifyProjectsChanged();
    }

    private void HandleProjectUpdated(Project p, IReadOnlyList<FieldChange> _)
    {
        if (_pendingUpdates.ContainsKey(p.Id)) return; // Skip if we updated it
        _projects[p.Id] = p;
        NotifyProjectsChanged();
    }

    private void HandleProjectDeleted(Guid id)
    {
        if (_pendingUpdates.ContainsKey(id)) return; // Skip if we deleted it
        _projects.TryRemove(id, out _);
        if (_currentProjectId == id)
        {
            _currentProjectId = null;
            NotifyCurrentProjectChanged();
        }
        NotifyProjectsChanged();
    }

    private void NotifyProjectsChanged()
    {
        try { OnProjectsChanged?.Invoke(); }
        catch (Exception ex) { _logger.LogError(ex, "Error in OnProjectsChanged handler"); }
    }

    private void NotifyCurrentProjectChanged()
    {
        try { OnCurrentProjectChanged?.Invoke(); }
        catch (Exception ex) { _logger.LogError(ex, "Error in OnCurrentProjectChanged handler"); }
    }

    public void Dispose()
    {
        if (_disposed) return;
        _disposed = true;
        _client.OnProjectCreated -= HandleProjectCreated;
        _client.OnProjectUpdated -= HandleProjectUpdated;
        _client.OnProjectDeleted -= HandleProjectDeleted;
    }
}
```
