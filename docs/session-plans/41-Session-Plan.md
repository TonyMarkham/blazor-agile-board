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
| 2 | `pm-core/src/models/project.rs` | Rust `Project` struct + `ProjectStatus` enum |
| 3 | `pm-db/src/repositories/project_repository.rs` | CRUD operations |
| 4 | `proto/messages.proto` | Project messages + Payload variants |
| 5 | `pm-ws/src/handlers/project.rs` | WebSocket handlers |
| 6 | `Core/Models/ProjectStatus.cs` | C# enum |
| 7 | `Core/Models/Project.cs` | C# record (depends on #6) |
| 8 | `Core/Models/CreateProjectRequest.cs` | DTO (depends on #6) |
| 9 | `Core/Models/UpdateProjectRequest.cs` | DTO (depends on #6, #7) |
| 10 | `Core/Interfaces/IWebSocketClient.cs` | Add events + operations (depends on #7, #8, #9) |
| 11 | `Services/WebSocket/WebSocketClient.cs` | Implement #10 (depends on #4, #5, #10) |
| 12 | `Core/ViewModels/ProjectViewModel.cs` | Wraps Project (depends on #7) |
| 13 | `Core/State/IProjectStore.cs` | Interface with async CRUD (depends on #7, #8, #9, #12) |
| 14 | `Services/State/ProjectStore.cs` | Implementation (depends on #10, #11, #13) |
| 15 | `Services/State/AppState.cs` | Add `Projects` property (depends on #13, #14) |
| 16 | `Wasm/Program.cs` | Register `IProjectStore` (depends on #14) |
| 17 | `Wasm/Layout/MainLayout.razor` | Show current project in header (depends on #15) |
| 18 | `Wasm/Pages/ProjectDetail.razor` | Set `CurrentProject` on load (depends on #15) |
| 19 | `Wasm/Pages/Home.razor` | Use Projects store, clear context (depends on #15) |
| 20 | `Components/Projects/ProjectDialog.razor` | Create/edit dialog (depends on #8, #9, #15) |
| 21 | `Wasm/Pages/Home.razor` | Wire Create button to ProjectDialog (depends on #20) |

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
pub enum ProjectStatus { Active, Archived }
```

### 3. Rust ProjectRepository

- `create`, `get_by_id`, `get_all`, `update`, `soft_delete`, `get_by_key`

### 4. Protobuf Messages

```protobuf
message Project { ... }
enum ProjectStatus { PROJECT_STATUS_UNSPECIFIED = 0; PROJECT_STATUS_ACTIVE = 1; PROJECT_STATUS_ARCHIVED = 2; }
message CreateProjectRequest { string title = 1; optional string description = 2; string key = 3; }
message UpdateProjectRequest { string project_id = 1; int32 expected_version = 2; ... }
message DeleteProjectRequest { string project_id = 1; }
message ListProjectsRequest {}
message ProjectCreated { Project project = 1; }
message ProjectUpdated { Project project = 1; }
message ProjectDeleted { string project_id = 1; }
message ProjectList { repeated Project projects = 1; }
```

### 5. Rust WebSocket Handlers

- `handle_create_project`, `handle_update_project`, `handle_delete_project`, `handle_list_projects`

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
Task DeleteProjectAsync(Guid projectId, CancellationToken ct = default);
Task<IReadOnlyList<Project>> GetProjectsAsync(CancellationToken ct = default);
```

### 11. WebSocketClient Implementation

Follow existing WorkItem pattern for protobuf serialization, request/response correlation.

### 12. ProjectViewModel

```csharp
public sealed class ProjectViewModel : IEquatable<ProjectViewModel>
{
    public ProjectViewModel(Project model, bool isPendingSync = false) { ... }
    internal Project Model { get; }
    public bool IsPendingSync { get; }
    public Guid Id => Model.Id;
    public string Title => Model.Title;
    public string? Description => Model.Description;
    public string Key => Model.Key;
    public ProjectStatus Status => Model.Status;
    // ... audit properties, equality
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
    Task DeleteAsync(Guid id, CancellationToken ct = default);
    Task RefreshAsync(CancellationToken ct = default);
    bool IsPending(Guid id);
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

- Fields: Title, Key (auto-generated from title, immutable on edit), Description
- Uses `AppState.Projects.CreateAsync()` / `UpdateAsync()`

### 21. Wire Create Button

Replace `WorkItemDialog` with `ProjectDialog` for "Create Project" button.

---

## Verification

- [ ] `pm_projects` table exists with correct schema
- [ ] Existing projects migrated from `pm_work_items`
- [ ] Backend handlers respond to Create/Update/Delete/List
- [ ] Frontend can create project via ProjectDialog
- [ ] Header shows "Agile Board / ProjectName" when in project
- [ ] Header shows only "Agile Board" on Home
- [ ] All tests pass

---

## Appendix: Full Migration SQL

```sql
-- Step 1: Create pm_projects
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

-- Step 2: Migrate existing Projects
INSERT INTO pm_projects (id, title, description, key, status, version, created_at, updated_at, created_by, updated_by, deleted_at)
SELECT id, title, description,
    UPPER(SUBSTR(REPLACE(title, ' ', ''), 1, 10)) || '_' || SUBSTR(id, 1, 4),
    'active', version, created_at, updated_at, created_by, updated_by, deleted_at
FROM pm_work_items WHERE item_type = 'project';

-- Step 3: Recreate pm_sprints with new FK
CREATE TABLE pm_sprints_new (..., FOREIGN KEY (project_id) REFERENCES pm_projects(id) ON DELETE CASCADE);
INSERT INTO pm_sprints_new SELECT * FROM pm_sprints;
DROP TABLE pm_sprints;
ALTER TABLE pm_sprints_new RENAME TO pm_sprints;

-- Step 4: Recreate pm_swim_lanes with new FK
CREATE TABLE pm_swim_lanes_new (..., FOREIGN KEY (project_id) REFERENCES pm_projects(id) ON DELETE CASCADE);
INSERT INTO pm_swim_lanes_new SELECT * FROM pm_swim_lanes;
DROP TABLE pm_swim_lanes;
ALTER TABLE pm_swim_lanes_new RENAME TO pm_swim_lanes;

-- Step 5: Recreate pm_project_members with new FK
CREATE TABLE pm_project_members_new (..., FOREIGN KEY (project_id) REFERENCES pm_projects(id) ON DELETE CASCADE);
INSERT INTO pm_project_members_new SELECT * FROM pm_project_members;
DROP TABLE pm_project_members;
ALTER TABLE pm_project_members_new RENAME TO pm_project_members;

-- Step 6: Recreate pm_work_items (remove 'project' type, new FK)
CREATE TABLE pm_work_items_new (
    ...,
    item_type TEXT NOT NULL CHECK(item_type IN ('epic', 'story', 'task')),
    FOREIGN KEY (project_id) REFERENCES pm_projects(id) ON DELETE CASCADE
);
INSERT INTO pm_work_items_new SELECT * FROM pm_work_items WHERE item_type != 'project';
DROP TABLE pm_work_items;
ALTER TABLE pm_work_items_new RENAME TO pm_work_items;

-- Step 7: Create indexes
CREATE INDEX idx_pm_projects_status ON pm_projects(status) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_projects_key ON pm_projects(key) WHERE deleted_at IS NULL;
```

---

## Appendix: ProjectStore Implementation

```csharp
public sealed class ProjectStore : IProjectStore, IDisposable
{
    private readonly IWebSocketClient _client;
    private readonly ILogger<ProjectStore> _logger;
    private readonly Dictionary<Guid, ProjectViewModel> _projects = new();
    private readonly HashSet<Guid> _pendingIds = new();
    private Guid? _currentProjectId;

    public ProjectStore(IWebSocketClient client, ILogger<ProjectStore> logger)
    {
        _client = client;
        _logger = logger;
        _client.OnProjectCreated += HandleProjectCreated;
        _client.OnProjectUpdated += HandleProjectUpdated;
        _client.OnProjectDeleted += HandleProjectDeleted;
    }

    public IReadOnlyList<ProjectViewModel> Projects => _projects.Values
        .Where(p => !p.IsDeleted).OrderBy(p => p.Title).ToList();

    public ProjectViewModel? CurrentProject => _currentProjectId.HasValue
        ? _projects.GetValueOrDefault(_currentProjectId.Value) : null;

    public bool IsLoaded { get; private set; }
    public event Action? OnCurrentProjectChanged;
    public event Action? OnProjectsChanged;

    public void SetCurrentProject(Guid projectId) { ... }
    public void ClearCurrentProject() { ... }

    public async Task<Project> CreateAsync(CreateProjectRequest request, CancellationToken ct)
    {
        // Optimistic update, call client, replace on success, rollback on failure
    }

    public async Task<Project> UpdateAsync(UpdateProjectRequest request, CancellationToken ct) { ... }
    public async Task DeleteAsync(Guid id, CancellationToken ct) { ... }
    public async Task RefreshAsync(CancellationToken ct) { ... }
    public bool IsPending(Guid id) => _pendingIds.Contains(id);

    private void HandleProjectCreated(Project p) { ... }
    private void HandleProjectUpdated(Project p, IReadOnlyList<FieldChange> c) { ... }
    private void HandleProjectDeleted(Guid id) { ... }

    public void Dispose() { /* unwire events */ }
}
```
