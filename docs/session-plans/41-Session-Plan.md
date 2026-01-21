# Session 41: Project Dialog - Dedicated UI for Project Management

**Status**: Planning (2026-01-21)
**Target**: ~35k tokens (expanded session due to schema refactor)
**Type**: Schema Refactor + Bug Fix + UX Improvement + State Management

---

## Context

**Problem Identified:** The "Create Project" button on Home.razor currently opens `WorkItemDialog`, which is designed for Epics/Stories/Tasks. This results in a confusing UX where users see irrelevant fields (Status, Priority, Story Points, Sprint) when creating a Project.

**Root Cause:** Projects are stored in the same polymorphic `pm_work_items` table as other work items, but they serve a different purpose:
- **Projects** = Containers/Portfolios (organizational constructs)
- **Epics/Stories/Tasks** = Actionable work items

**Deeper Root Cause (NEW):** The original design treated Projects as "just another work item type" stored in the polymorphic `pm_work_items` table. This is fundamentally wrong:
- Projects are organizational containers (like JIRA projects) - they define context, have settings, team members, workflows
- Work Items (Epics/Stories/Tasks) are actionable units of work that live *inside* Projects
- Projects have irrelevant fields: `status`, `priority`, `story_points`, `sprint_id`, `assignee_id`
- Self-referential hack required: `project_id = id` for Projects
- Backend handler doesn't even implement this hack correctly

**Real-World Model (JIRA, etc.):**
- Projects are first-class citizens with their own table
- Projects have unique keys (e.g., "PROJ") for issue numbering (PROJ-123)
- Work items belong to exactly one Project
- Projects don't have sprints, assignees, or story points

---

## Scope

This session has THREE parts:
1. **Part 0: Schema Foundation** - Promote Projects to first-class entities (NEW)
2. **Part A: State Management Foundation** - ProjectViewModel, ProjectStore, AppState integration
3. **Part B: UI Integration** - MainLayout header, ProjectDialog component, Home.razor updates

### Part 0: Schema Foundation (Steps 0.1-0.7) - NEW
0.1. Create `pm_projects` table with project-specific fields
0.2. Create migration to move existing Projects from `pm_work_items` to `pm_projects`
0.3. Update foreign keys in dependent tables
0.4. Create `Project` model in `pm-core` (separate from `WorkItem`)
0.5. Create `ProjectRepository` in `pm-db`
0.6. Add Project protobuf messages
0.7. Create Project WebSocket handlers (Create, Update, Delete, List)

### Part A: State Management Foundation (Steps 1-5)
1. Create `ProjectViewModel` (only project-relevant properties)
2. Add `IProjectStore` interface returning `ProjectViewModel`
3. Implement `ProjectStore` with CurrentProject and Projects list
4. Wire into `AppState` and DI container
5. Add `LoadProjectsAsync()` to fetch projects from backend

### Part B: UI Integration (Steps 6-10)
6. Update `MainLayout` header to show current project name (subscribe to state changes)
7. Update `ProjectDetail` to set CurrentProject on load; `Home.razor` clears it
8. Update `Home.razor` to load from Projects store
9. Create `ProjectDialog` component (Title + Description + Key)
10. Update `Home.razor` to use `ProjectDialog`

---

## Part 0: Database Schema Refactor (NEW)

### New Table: `pm_projects`

```sql
CREATE TABLE pm_projects (
    id TEXT PRIMARY KEY,

    -- Core fields
    title TEXT NOT NULL,
    description TEXT,
    key TEXT NOT NULL,              -- Unique key for issue numbering (e.g., "PROJ" → PROJ-123)

    -- Project lifecycle (not workflow status)
    status TEXT NOT NULL DEFAULT 'active' CHECK(status IN ('active', 'archived')),

    -- Audit
    version INTEGER NOT NULL DEFAULT 1,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    created_by TEXT NOT NULL,
    updated_by TEXT NOT NULL,
    deleted_at INTEGER,

    FOREIGN KEY (created_by) REFERENCES users(id),
    FOREIGN KEY (updated_by) REFERENCES users(id),

    UNIQUE(key)
);

CREATE INDEX idx_pm_projects_status ON pm_projects(status) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_projects_key ON pm_projects(key) WHERE deleted_at IS NULL;
```

**Key differences from `pm_work_items`:**
| Field | pm_projects | pm_work_items | Reason |
|-------|-------------|---------------|--------|
| `key` | ✅ Required | ❌ N/A | Issue numbering (PROJ-123) |
| `status` | `active`/`archived` | `backlog`/`in_progress`/etc. | Lifecycle vs workflow |
| `priority` | ❌ N/A | ✅ | Projects aren't prioritized |
| `story_points` | ❌ N/A | ✅ | Projects aren't estimated |
| `sprint_id` | ❌ N/A | ✅ | Projects contain sprints |
| `assignee_id` | ❌ N/A | ✅ | Projects have teams, not assignees |
| `parent_id` | ❌ N/A | ✅ | Projects don't nest |
| `item_type` | ❌ N/A | ✅ | No discriminator needed |

### Updated Table: `pm_work_items`

```sql
-- Remove 'project' from allowed types
item_type TEXT NOT NULL CHECK(item_type IN ('epic', 'story', 'task')),

-- Change FK to reference pm_projects
FOREIGN KEY (project_id) REFERENCES pm_projects(id) ON DELETE CASCADE,
```

### Tables Requiring FK Update

| Table | Column | Old FK | New FK |
|-------|--------|--------|--------|
| `pm_work_items` | `project_id` | `pm_work_items(id)` | `pm_projects(id)` |
| `pm_sprints` | `project_id` | `pm_work_items(id)` | `pm_projects(id)` |
| `pm_swim_lanes` | `project_id` | `pm_work_items(id)` | `pm_projects(id)` |
| `pm_project_members` | `project_id` | `pm_work_items(id)` | `pm_projects(id)` |

### Migration Strategy

SQLite doesn't support `ALTER TABLE ... ALTER COLUMN` for FK changes. We need to:

1. Create `pm_projects` table
2. Migrate data from `pm_work_items WHERE item_type = 'project'`
3. Recreate dependent tables with new FKs (SQLite limitation)
4. Delete migrated Projects from `pm_work_items`
5. Recreate `pm_work_items` without 'project' in item_type check

```sql
-- Migration: 20260121000001_create_projects_table.sql

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
    FOREIGN KEY (created_by) REFERENCES users(id),
    FOREIGN KEY (updated_by) REFERENCES users(id),
    UNIQUE(key)
);

-- Step 2: Migrate existing Projects
-- Generate key from title (uppercase, first word, max 10 chars)
INSERT INTO pm_projects (id, title, description, key, status, version, created_at, updated_at, created_by, updated_by, deleted_at)
SELECT
    id,
    title,
    description,
    UPPER(SUBSTR(REPLACE(title, ' ', ''), 1, 10)) || '_' || SUBSTR(id, 1, 4),  -- Generate unique key
    'active',
    version,
    created_at,
    updated_at,
    created_by,
    updated_by,
    deleted_at
FROM pm_work_items
WHERE item_type = 'project';

-- Step 3: Update pm_sprints FK (recreate table - SQLite limitation)
CREATE TABLE pm_sprints_new (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    name TEXT NOT NULL,
    goal TEXT,
    start_date INTEGER NOT NULL,
    end_date INTEGER NOT NULL,
    status TEXT NOT NULL DEFAULT 'planned' CHECK(status IN ('planned', 'active', 'completed', 'cancelled')),
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    created_by TEXT NOT NULL,
    updated_by TEXT NOT NULL,
    deleted_at INTEGER,
    FOREIGN KEY (project_id) REFERENCES pm_projects(id) ON DELETE CASCADE,
    FOREIGN KEY (created_by) REFERENCES users(id),
    FOREIGN KEY (updated_by) REFERENCES users(id)
);

INSERT INTO pm_sprints_new SELECT * FROM pm_sprints;
DROP TABLE pm_sprints;
ALTER TABLE pm_sprints_new RENAME TO pm_sprints;

-- Recreate indexes
CREATE INDEX idx_pm_sprints_project ON pm_sprints(project_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_sprints_status ON pm_sprints(status) WHERE deleted_at IS NULL;

-- Step 4: Update pm_swim_lanes FK (same pattern)
CREATE TABLE pm_swim_lanes_new (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    name TEXT NOT NULL,
    status_value TEXT NOT NULL,
    position INTEGER NOT NULL DEFAULT 0,
    is_default BOOLEAN NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    deleted_at INTEGER,
    FOREIGN KEY (project_id) REFERENCES pm_projects(id) ON DELETE CASCADE,
    UNIQUE(project_id, status_value)
);

INSERT INTO pm_swim_lanes_new SELECT * FROM pm_swim_lanes;
DROP TABLE pm_swim_lanes;
ALTER TABLE pm_swim_lanes_new RENAME TO pm_swim_lanes;

CREATE INDEX idx_pm_swim_lanes_project ON pm_swim_lanes(project_id) WHERE deleted_at IS NULL;

-- Step 5: Update pm_project_members FK
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

CREATE UNIQUE INDEX idx_pm_project_members_project_user ON pm_project_members(project_id, user_id);

-- Step 6: Update pm_work_items (remove 'project' type, change FK)
CREATE TABLE pm_work_items_new (
    id TEXT PRIMARY KEY,
    item_type TEXT NOT NULL CHECK(item_type IN ('epic', 'story', 'task')),
    parent_id TEXT,
    project_id TEXT NOT NULL,
    position INTEGER NOT NULL DEFAULT 0,
    title TEXT NOT NULL,
    description TEXT,
    status TEXT NOT NULL DEFAULT 'backlog',
    priority TEXT NOT NULL DEFAULT 'medium',
    assignee_id TEXT,
    sprint_id TEXT,
    story_points INTEGER,
    version INTEGER NOT NULL DEFAULT 1,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    created_by TEXT NOT NULL,
    updated_by TEXT NOT NULL,
    deleted_at INTEGER,
    FOREIGN KEY (parent_id) REFERENCES pm_work_items_new(id) ON DELETE CASCADE,
    FOREIGN KEY (project_id) REFERENCES pm_projects(id) ON DELETE CASCADE,
    FOREIGN KEY (sprint_id) REFERENCES pm_sprints(id) ON DELETE SET NULL,
    FOREIGN KEY (assignee_id) REFERENCES users(id) ON DELETE SET NULL
);

-- Copy non-project work items
INSERT INTO pm_work_items_new
SELECT * FROM pm_work_items WHERE item_type != 'project';

DROP TABLE pm_work_items;
ALTER TABLE pm_work_items_new RENAME TO pm_work_items;

-- Recreate indexes
CREATE INDEX idx_pm_work_items_parent ON pm_work_items(parent_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_work_items_project ON pm_work_items(project_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_work_items_assignee ON pm_work_items(assignee_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_work_items_type ON pm_work_items(item_type) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_work_items_status ON pm_work_items(status) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_work_items_sprint ON pm_work_items(sprint_id) WHERE deleted_at IS NULL;

-- Step 7: Create pm_projects indexes
CREATE INDEX idx_pm_projects_status ON pm_projects(status) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_projects_key ON pm_projects(key) WHERE deleted_at IS NULL;
```

### Backend Changes for Part 0

#### 0.4 Project Model (`pm-core/src/models/project.rs`)

```rust
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Project {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub key: String,                    // Unique key for issue numbering
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
```

#### 0.5 ProjectRepository (`pm-db/src/repositories/project_repository.rs`)

Standard CRUD operations:
- `create(project: &Project) -> Result<Project, DbError>`
- `get_by_id(id: Uuid) -> Result<Option<Project>, DbError>`
- `get_all() -> Result<Vec<Project>, DbError>`
- `update(project: &Project) -> Result<Project, DbError>`
- `soft_delete(id: Uuid) -> Result<(), DbError>`
- `get_by_key(key: &str) -> Result<Option<Project>, DbError>`

#### 0.6 Project Protobuf Messages

```protobuf
// project.proto
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

enum ProjectStatus {
    PROJECT_STATUS_UNSPECIFIED = 0;
    PROJECT_STATUS_ACTIVE = 1;
    PROJECT_STATUS_ARCHIVED = 2;
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
}

message ListProjectsRequest {
    // Empty for now, could add filters later
}

message ProjectCreated {
    Project project = 1;
}

message ProjectUpdated {
    Project project = 1;
}

message ProjectDeleted {
    string project_id = 1;
}

message ProjectList {
    repeated Project projects = 1;
}
```

#### 0.7 Project WebSocket Handlers

- `handle_create_project` - Create new project
- `handle_update_project` - Update existing project
- `handle_delete_project` - Soft delete project
- `handle_list_projects` - Return all projects for tenant

---

## Learning Objectives

After completing this session, you will understand:
- When to create specialized UI components vs. reusing generic ones
- How to handle polymorphic data models in the UI layer
- The difference between backend uniformity and frontend specialization
- Creating Razor dialogs with Radzen components
- Why first-class entities deserve their own tables (NEW)

---

## Prerequisites

- Session 30.5 complete (Pages + Layout)
- Understand WorkItemDialog.razor structure
- Understand AppState and ViewModelFactory patterns

### Backend Fix No Longer Required

~~**Issue:** The backend at `pm-ws/src/handlers/work_item.rs:131` doesn't handle Project creation correctly.~~

**Resolution:** With Part 0, Projects have their own table and handlers. No more self-reference hack needed.

---

## Design Decisions

### Decision 1: Separate Dialog vs. Conditional Fields

**Chosen Approach:** Separate `ProjectDialog` component

**Rationale:**
- ✅ Clear separation of concerns
- ✅ Simpler code (no complex conditionals)
- ✅ Better UX (no confusion about irrelevant fields)
- ✅ Future-proof (can add Project-specific fields later without complicating WorkItemDialog)

**Alternative Rejected:** Adding `@if (_itemType != WorkItemType.Project)` conditions in WorkItemDialog
- ❌ Complex conditionals throughout the component
- ❌ Harder to maintain
- ❌ Type dropdown would still show all options

### Decision 2: Projects as First-Class Entities (NEW)

**Approach:** Promote Projects from `pm_work_items` to their own `pm_projects` table.

**Rationale:**
- ✅ Clean data model (no irrelevant fields)
- ✅ No self-reference hack needed
- ✅ Can add project-specific fields (key, budget, team settings) without polluting work_items
- ✅ Clearer API (separate endpoints/handlers)
- ✅ Better matches industry standards (JIRA, etc.)

---

## Implementation Steps

### Part A: State Management Foundation

### Step 1: Create ProjectViewModel

**File:** `frontend/ProjectManagement.Core/ViewModels/ProjectViewModel.cs`

**Rationale:** Now that Projects have their own model, ProjectViewModel wraps `Project` (not `WorkItem`).

```csharp
using ProjectManagement.Core.Models;

namespace ProjectManagement.Core.ViewModels;

/// <summary>
/// View model for Projects. Only exposes project-relevant properties.
/// </summary>
public sealed class ProjectViewModel : IEquatable<ProjectViewModel>
{
    public ProjectViewModel(Project model, bool isPendingSync = false)
    {
        ArgumentNullException.ThrowIfNull(model);
        Model = model;
        IsPendingSync = isPendingSync;
    }

    internal Project Model { get; }
    public bool IsPendingSync { get; }

    // === Identity ===
    public Guid Id => Model.Id;
    public int Version => Model.Version;

    // === Project Properties ===
    public string Title => Model.Title;
    public string? Description => Model.Description;
    public string Key => Model.Key;
    public ProjectStatus Status => Model.Status;

    // === Audit ===
    public Guid CreatedBy => Model.CreatedBy;
    public Guid UpdatedBy => Model.UpdatedBy;
    public DateTime CreatedAt => Model.CreatedAt;
    public DateTime UpdatedAt => Model.UpdatedAt;
    public DateTime? DeletedAt => Model.DeletedAt;

    // === Computed Properties ===
    public bool IsDeleted => Model.DeletedAt.HasValue;
    public bool IsArchived => Model.Status == ProjectStatus.Archived;

    // === Equality ===
    public bool Equals(ProjectViewModel? other)
    {
        if (other is null) return false;
        if (ReferenceEquals(this, other)) return true;
        return Id == other.Id && Version == other.Version && IsPendingSync == other.IsPendingSync;
    }

    public override bool Equals(object? obj) => Equals(obj as ProjectViewModel);
    public override int GetHashCode() => HashCode.Combine(Id, Version, IsPendingSync);
}
```

### Step 2: Add IProjectStore Interface

**File:** `frontend/ProjectManagement.Core/State/IProjectStore.cs`

```csharp
using ProjectManagement.Core.ViewModels;

namespace ProjectManagement.Core.State;

public interface IProjectStore
{
    /// <summary>All loaded projects (for sidebar, home page)</summary>
    IReadOnlyList<ProjectViewModel> Projects { get; }

    /// <summary>Currently selected project (for context-aware UI)</summary>
    ProjectViewModel? CurrentProject { get; }

    /// <summary>Set the current project by ID</summary>
    void SetCurrentProject(Guid projectId);

    /// <summary>Clear the current project selection</summary>
    void ClearCurrentProject();

    /// <summary>Add or update a project in the store</summary>
    void AddOrUpdate(ProjectViewModel project);

    /// <summary>Remove a project from the store</summary>
    void Remove(Guid projectId);

    /// <summary>Check if projects have been loaded</summary>
    bool IsLoaded { get; }

    /// <summary>Event fired when current project changes</summary>
    event Action? OnCurrentProjectChanged;

    /// <summary>Event fired when projects list changes</summary>
    event Action? OnProjectsChanged;

    /// <summary>Bulk load projects (replaces existing)</summary>
    void Load(IEnumerable<ProjectViewModel> projects);
}
```

### Step 3: Implement ProjectStore

**File:** `frontend/ProjectManagement.Services/State/ProjectStore.cs`

```csharp
using ProjectManagement.Core.State;
using ProjectManagement.Core.ViewModels;

namespace ProjectManagement.Services.State;

public class ProjectStore : IProjectStore
{
    private readonly Dictionary<Guid, ProjectViewModel> _projects = new();
    private Guid? _currentProjectId;

    public IReadOnlyList<ProjectViewModel> Projects => _projects.Values.ToList();

    public ProjectViewModel? CurrentProject => _currentProjectId.HasValue
        ? _projects.GetValueOrDefault(_currentProjectId.Value)
        : null;

    public bool IsLoaded { get; private set; }

    public event Action? OnCurrentProjectChanged;
    public event Action? OnProjectsChanged;

    public void SetCurrentProject(Guid projectId)
    {
        if (_currentProjectId != projectId)
        {
            _currentProjectId = projectId;
            OnCurrentProjectChanged?.Invoke();
        }
    }

    public void ClearCurrentProject()
    {
        if (_currentProjectId.HasValue)
        {
            _currentProjectId = null;
            OnCurrentProjectChanged?.Invoke();
        }
    }

    public void AddOrUpdate(ProjectViewModel project)
    {
        _projects[project.Id] = project;
        OnProjectsChanged?.Invoke();
    }

    public void Remove(Guid projectId)
    {
        if (_projects.Remove(projectId))
        {
            if (_currentProjectId == projectId)
            {
                _currentProjectId = null;
                OnCurrentProjectChanged?.Invoke();
            }
            OnProjectsChanged?.Invoke();
        }
    }

    public void Load(IEnumerable<ProjectViewModel> projects)
    {
        _projects.Clear();
        foreach (var project in projects)
        {
            _projects[project.Id] = project;
        }
        IsLoaded = true;
        OnProjectsChanged?.Invoke();
    }
}
```

### Step 4: Update AppState

**Modify:** `frontend/ProjectManagement.Services/State/AppState.cs`

**Current constructor signature:**
```csharp
public AppState(
    IWebSocketClient client,
    IWorkItemStore workItems,
    ISprintStore sprints,
    ILogger<AppState> logger)
```

**Updated constructor:**
```csharp
public AppState(
    IWebSocketClient client,
    IWorkItemStore workItems,
    ISprintStore sprints,
    IProjectStore projects,  // NEW
    ILogger<AppState> logger)
{
    _client = client;
    _logger = logger;

    WorkItems = workItems;
    Sprints = sprints;
    Projects = projects;  // NEW

    // Existing event wiring...
    _client.OnStateChanged += state =>
    {
        OnConnectionStateChanged?.Invoke(state);
        OnStateChanged?.Invoke();
    };

    workItems.OnChanged += () => OnStateChanged?.Invoke();
    sprints.OnChanged += () => OnStateChanged?.Invoke();

    // NEW: Wire project store events
    projects.OnCurrentProjectChanged += () => OnStateChanged?.Invoke();
    projects.OnProjectsChanged += () => OnStateChanged?.Invoke();
}

// NEW property
public IProjectStore Projects { get; }
```

**Add LoadProjectsAsync method:**
```csharp
public async Task LoadProjectsAsync(CancellationToken ct = default)
{
    ThrowIfDisposed();

    // Call the new Project-specific endpoint
    var projectList = await _client.GetProjectsAsync(ct);

    var projects = projectList
        .Select(p => new ProjectViewModel(p))
        .ToList();

    Projects.Load(projects);
}
```

### Step 5: Register in DI Container

**Modify:** `frontend/ProjectManagement.Wasm/Program.cs`

**Add:**
```csharp
builder.Services.AddSingleton<IProjectStore, ProjectStore>();
```

---

### Part B: UI Integration

### Step 6: Update MainLayout Header

**Modify:** `frontend/ProjectManagement.Wasm/Layout/MainLayout.razor`

**Note:** MainLayout currently has NO `@code` block. We need to add one.

**Add at top of file:**
```razor
@using ProjectManagement.Services.State
@inject AppState AppState
@implements IDisposable
```

**Add at bottom of file (new @code block):**
```razor
@code {
    protected override void OnInitialized()
    {
        AppState.OnStateChanged += StateHasChanged;
    }

    public void Dispose()
    {
        AppState.OnStateChanged -= StateHasChanged;
    }
}
```

**Change header section:**
```razor
<RadzenStack Orientation="Orientation.Horizontal"
             AlignItems="AlignItems.Center"
             Gap="0.75rem">
    <RadzenIcon Icon="dashboard" Style="font-size: 1.5rem;" />
    <RadzenText TextStyle="TextStyle.H5" class="m-0">
        Agile Board
        @if (AppState.Projects.CurrentProject is not null)
        {
            <span class="project-context">
                <span class="separator">/</span>
                @AppState.Projects.CurrentProject.Title
            </span>
        }
    </RadzenText>
</RadzenStack>
```

**Add styling to `MainLayout.razor.css`** (scoped CSS file already exists):
```css
.project-context {
    font-weight: normal;
    color: var(--rz-text-secondary-color);
}

.project-context .separator {
    margin: 0 0.5rem;
    opacity: 0.5;
}
```

### Step 7: Set CurrentProject on Navigation

**Modify:** `frontend/ProjectManagement.Wasm/Pages/ProjectDetail.razor`

**In LoadProjectAsync():**
```csharp
private async Task LoadProjectAsync()
{
    // ... existing loading code ...

    // Set as current project (sticky - persists until Home or new project)
    AppState.Projects.SetCurrentProject(ProjectId);
}
```

**Note:** Do NOT clear CurrentProject in `Dispose()`. Project context is sticky until:
- User clicks Home (clears context)
- User loads a different project (replaces context)

**Modify:** `frontend/ProjectManagement.Wasm/Pages/Home.razor`

**In OnInitializedAsync():**
```csharp
protected override async Task OnInitializedAsync()
{
    // Clear project context when returning to Home
    AppState.Projects.ClearCurrentProject();

    await AppState.LoadProjectsAsync();
    _projects = AppState.Projects.Projects.ToList();
}
```

### Step 8: Update Home.razor (Consolidated Changes)

**Modify:** `frontend/ProjectManagement.Wasm/Pages/Home.razor`

**All changes to Home.razor in one place:**

1. **Add using directive at top:**
```razor
@using ProjectManagement.Core.ViewModels
@using ProjectManagement.Components.Projects
```

2. **Change field type:**
```csharp
// Before
private List<WorkItemViewModel> _projects = new();

// After
private List<ProjectViewModel> _projects = new();
```

3. **Update OnInitializedAsync:**
```csharp
protected override async Task OnInitializedAsync()
{
    _connectionState = AppState.ConnectionState;
    AppState.OnStateChanged += HandleStateChanged;
    AppState.OnConnectionStateChanged += HandleConnectionChanged;

    // Clear project context when returning to Home
    AppState.Projects.ClearCurrentProject();

    await LoadDataAsync();
}
```

4. **Update LoadDataAsync:**
```csharp
private async Task LoadDataAsync()
{
    _loading = true;
    StateHasChanged();

    try
    {
        await AppState.LoadProjectsAsync();
        _projects = AppState.Projects.Projects.ToList();
    }
    catch (Exception ex)
    {
        NotificationService.Notify(
            NotificationSeverity.Error,
            "Error",
            $"Failed to load projects: {ex.Message}");
    }
    finally
    {
        _loading = false;
        StateHasChanged();
    }
}
```

5. **Update ShowCreateProjectDialog:**
```csharp
private async Task ShowCreateProjectDialog()
{
    var result = await DialogService.OpenAsync<ProjectDialog>(
        "Create Project",
        parameters: null,
        new DialogOptions { Width = "450px" });

    if (result == true)
    {
        // Refresh projects list from store (already updated by dialog)
        _projects = AppState.Projects.Projects.ToList();
        StateHasChanged();
    }
}
```

6. **Update HandleStateChanged to refresh from store:**
```csharp
private void HandleStateChanged()
{
    _projects = AppState.Projects.Projects.ToList();
    InvokeAsync(StateHasChanged);
}
```

### Step 9: Create ProjectDialog Component

**File:** `frontend/ProjectManagement.Components/Projects/ProjectDialog.razor`

**Fields:**
- Title (required, max 200 chars)
- Key (required, max 10 chars, uppercase alphanumeric + underscore)
- Description (optional, max 5000 chars)

**Features:**
- Form validation (title required, key required and unique)
- Character counters (Title: 0/200, Key: 0/10, Description: 0/5000)
- Key auto-generation from title (can be overridden)
- Connection state awareness
- Dirty tracking (unsaved changes warning)
- Error handling with notifications
- Version conflict handling (for edits)
- Cancel confirmation if dirty

**Parameters:**
```csharp
[Parameter] public ProjectViewModel? Project { get; set; }
```

If `Project` is null → Create mode
If `Project` is provided → Edit mode

### Step 10: Verify Home.razor Changes Complete

All Home.razor changes are consolidated in Step 8. Verify:
- [x] Using directive for `ProjectManagement.Components.Projects` added
- [x] Field type changed to `List<ProjectViewModel>`
- [x] `ClearCurrentProject()` called in `OnInitializedAsync`
- [x] `LoadDataAsync` uses `AppState.LoadProjectsAsync()` and `AppState.Projects.Projects`
- [x] `ShowCreateProjectDialog` uses `ProjectDialog` component
- [x] `HandleStateChanged` refreshes from store

---

## Code Structure

### ProjectDialog.razor - Complete Implementation

Based on `WorkItemDialog.razor` patterns, here's the full implementation:

```razor
@using ProjectManagement.Core.Models
@using ProjectManagement.Core.ViewModels
@using ProjectManagement.Core.Exceptions
@using ProjectManagement.Components.Shared
@inject AppState AppState
@inject DialogService DialogService
@inject NotificationService NotificationService
@implements IDisposable

<RadzenStack Gap="1rem" class="p-2">
    @* Title Field *@
    <RadzenFormField Text="Title" Variant="Variant.Outlined" class="w-100">
        <ChildContent>
            <RadzenTextBox @bind-Value="@_title"
                           Change="@(args => HandleTitleChange(args))"
                           MaxLength="200"
                           Placeholder="Enter project title"
                           class="w-100" />
        </ChildContent>
        <Helper>
            <div class="d-flex justify-content-between">
                @if (_errors.TryGetValue("Title", out var titleError))
                {
                    <span class="rz-message-error">@titleError</span>
                }
                else
                {
                    <span>&nbsp;</span>
                }
                <span class="rz-text-secondary-color">@_title.Length/200</span>
            </div>
        </Helper>
    </RadzenFormField>

    @* Key Field *@
    <RadzenFormField Text="Key" Variant="Variant.Outlined" class="w-100">
        <ChildContent>
            <RadzenTextBox @bind-Value="@_key"
                           Change="@(args => HandleKeyChange(args))"
                           MaxLength="10"
                           Placeholder="e.g., PROJ"
                           Disabled="@_isEdit"
                           class="w-100"
                           Style="text-transform: uppercase;" />
        </ChildContent>
        <Helper>
            <div class="d-flex justify-content-between">
                @if (_errors.TryGetValue("Key", out var keyError))
                {
                    <span class="rz-message-error">@keyError</span>
                }
                else
                {
                    <span class="rz-text-secondary-color">Used for issue numbering (e.g., @(_key)-123)</span>
                }
                <span class="rz-text-secondary-color">@_key.Length/10</span>
            </div>
        </Helper>
    </RadzenFormField>

    @* Description Field *@
    <RadzenFormField Text="Description" Variant="Variant.Outlined" class="w-100">
        <ChildContent>
            <RadzenTextArea @bind-Value="@_description"
                            Change="@(args => HandleDescriptionChange(args))"
                            MaxLength="5000"
                            Rows="4"
                            Placeholder="Optional project description"
                            class="w-100" />
        </ChildContent>
        <Helper>
            <div class="d-flex justify-content-end">
                <span class="rz-text-secondary-color">@(_description?.Length ?? 0)/5000</span>
            </div>
        </Helper>
    </RadzenFormField>

    @* Buttons *@
    <RadzenStack Orientation="Orientation.Horizontal"
                 JustifyContent="JustifyContent.End"
                 Gap="0.5rem"
                 class="mt-2">
        <RadzenButton Text="Cancel"
                      Variant="Variant.Text"
                      Click="@HandleCancel"
                      Disabled="@_saving" />
        <LoadingButton Text="@(_isEdit ? "Save Changes" : "Create Project")"
                       LoadingText="@(_isEdit ? "Saving..." : "Creating...")"
                       IsBusy="@_saving"
                       ConnectionState="@_connectionState"
                       OnClick="@HandleSubmit" />
    </RadzenStack>
</RadzenStack>

@code {
    [Parameter]
    public ProjectViewModel? Project { get; set; }

    // State
    private bool _isEdit => Project is not null;
    private bool _saving;
    private bool _isDirty;
    private ConnectionState _connectionState = ConnectionState.Connected;
    private Dictionary<string, string> _errors = new();
    private bool _keyManuallySet;

    // Form fields
    private string _title = "";
    private string _key = "";
    private string? _description;

    // Original values for dirty tracking
    private string _originalTitle = "";
    private string _originalKey = "";
    private string? _originalDescription;

    protected override void OnInitialized()
    {
        _connectionState = AppState.ConnectionState;
        AppState.OnConnectionStateChanged += HandleConnectionChanged;

        if (_isEdit && Project is not null)
        {
            _title = Project.Title;
            _key = Project.Key;
            _description = Project.Description;
            _originalTitle = _title;
            _originalKey = _key;
            _originalDescription = _description;
            _keyManuallySet = true;  // Don't auto-generate key in edit mode
        }
    }

    private void HandleTitleChange(string value)
    {
        _title = value;
        _errors.Remove("Title");

        // Auto-generate key from title if not manually set
        if (!_keyManuallySet && !_isEdit)
        {
            _key = GenerateKeyFromTitle(value);
        }

        UpdateDirtyState();
    }

    private void HandleKeyChange(string value)
    {
        _key = value.ToUpperInvariant();
        _keyManuallySet = !string.IsNullOrWhiteSpace(value);
        _errors.Remove("Key");
        UpdateDirtyState();
    }

    private void HandleDescriptionChange(string value)
    {
        _description = value;
        UpdateDirtyState();
    }

    private string GenerateKeyFromTitle(string title)
    {
        if (string.IsNullOrWhiteSpace(title)) return "";

        // Take first word, uppercase, max 10 chars, alphanumeric only
        var words = title.Split(' ', StringSplitOptions.RemoveEmptyEntries);
        var key = words.Length > 0 ? words[0] : title;
        key = new string(key.Where(c => char.IsLetterOrDigit(c)).ToArray());
        return key.ToUpperInvariant().Substring(0, Math.Min(key.Length, 10));
    }

    private void UpdateDirtyState()
    {
        _isDirty = _title != _originalTitle ||
                   _key != _originalKey ||
                   _description != _originalDescription;
    }

    private bool Validate()
    {
        _errors.Clear();

        var trimmedTitle = _title?.Trim() ?? "";
        if (string.IsNullOrWhiteSpace(trimmedTitle))
        {
            _errors["Title"] = "Title is required";
        }
        else if (trimmedTitle.Length > 200)
        {
            _errors["Title"] = "Title must be 200 characters or less";
        }

        var trimmedKey = _key?.Trim() ?? "";
        if (string.IsNullOrWhiteSpace(trimmedKey))
        {
            _errors["Key"] = "Key is required";
        }
        else if (trimmedKey.Length > 10)
        {
            _errors["Key"] = "Key must be 10 characters or less";
        }
        else if (!System.Text.RegularExpressions.Regex.IsMatch(trimmedKey, @"^[A-Z0-9_]+$"))
        {
            _errors["Key"] = "Key must be uppercase letters, numbers, or underscore";
        }

        return _errors.Count == 0;
    }

    private async Task HandleSubmit()
    {
        if (!Validate()) return;

        _saving = true;
        StateHasChanged();

        try
        {
            if (_isEdit)
            {
                await UpdateProjectAsync();
            }
            else
            {
                await CreateProjectAsync();
            }

            NotificationService.Notify(
                NotificationSeverity.Success,
                "Success",
                _isEdit ? "Project updated" : "Project created");

            DialogService.Close(true);
        }
        catch (VersionConflictException)
        {
            await HandleVersionConflictAsync();
        }
        catch (Exception ex)
        {
            NotificationService.Notify(
                NotificationSeverity.Error,
                "Error",
                ex.Message);
        }
        finally
        {
            _saving = false;
            StateHasChanged();
        }
    }

    private async Task CreateProjectAsync()
    {
        var request = new CreateProjectRequest
        {
            Title = _title.Trim(),
            Key = _key.Trim().ToUpperInvariant(),
            Description = _description?.Trim()
        };

        var created = await AppState.Projects.CreateAsync(request);

        // Add to Projects store
        var projectVm = new ProjectViewModel(created);
        AppState.Projects.AddOrUpdate(projectVm);
    }

    private async Task UpdateProjectAsync()
    {
        var request = new UpdateProjectRequest
        {
            ProjectId = Project!.Id,
            ExpectedVersion = Project.Version,
            Title = _title.Trim(),
            Description = _description?.Trim()
        };

        var updated = await AppState.Projects.UpdateAsync(request);

        // Update in Projects store
        var projectVm = new ProjectViewModel(updated);
        AppState.Projects.AddOrUpdate(projectVm);
    }

    private async Task HandleVersionConflictAsync()
    {
        var result = await DialogService.OpenAsync<VersionConflictDialog>(
            "Version Conflict",
            new Dictionary<string, object>
            {
                { "Message", "This project was modified by another user." }
            },
            new DialogOptions { Width = "400px" });

        // Handle result: reload, overwrite, or cancel
    }

    private async Task HandleCancel()
    {
        if (_isDirty)
        {
            var confirmed = await DialogService.Confirm(
                "You have unsaved changes. Discard them?",
                "Unsaved Changes",
                new ConfirmOptions
                {
                    OkButtonText = "Discard",
                    CancelButtonText = "Keep Editing"
                });

            if (confirmed != true) return;
        }

        DialogService.Close(false);
    }

    private void HandleConnectionChanged(ConnectionState state)
    {
        _connectionState = state;
        InvokeAsync(StateHasChanged);
    }

    public void Dispose()
    {
        AppState.OnConnectionStateChanged -= HandleConnectionChanged;
    }
}
```

---

## Validation Rules

### Title Validation
- Required (not null/whitespace)
- Max length: 200 characters
- Trimmed before validation

### Key Validation (NEW)
- Required (not null/whitespace)
- Max length: 10 characters
- Uppercase alphanumeric + underscore only
- Auto-generated from title (can be overridden)
- Immutable after creation (disabled in edit mode)
- Must be unique per tenant

### Description Validation
- Optional
- Max length: 5000 characters

---

## Error Handling

### Connection State
- Monitor `AppState.ConnectionState`
- Disable Create/Save button when disconnected
- Show connection status in LoadingButton

### Version Conflicts (Edit Mode)
- Catch `VersionConflictException`
- Show `VersionConflictDialog` with options:
  - Reload (discard changes, load latest)
  - Overwrite (force save with current version)
  - Cancel (stay in dialog)

### General Errors
- Catch all exceptions
- Show error notification
- Keep dialog open
- Don't close on error

---

## Testing Checklist

### Create Project Flow
- [ ] Click "New Project" on Home page
- [ ] Dialog opens with title "Create Project"
- [ ] Only Title, Key, and Description fields visible
- [ ] Title field is empty
- [ ] Key field is empty
- [ ] Description field is empty
- [ ] Create button is disabled (title required)
- [ ] Type "Test Project" in title
- [ ] Key auto-fills with "TESTPROJE" (first 10 chars)
- [ ] Create button becomes enabled
- [ ] Character count shows "12/200"
- [ ] Click Create
- [ ] Success notification appears
- [ ] Dialog closes
- [ ] Project appears in list on Home page

### Validation
- [ ] Empty title → Create disabled
- [ ] Whitespace-only title → Error message "Title is required"
- [ ] 201 character title → Error message "Title must be 200 characters or less"
- [ ] Valid title → No errors
- [ ] Empty key → Error message "Key is required"
- [ ] Key with spaces → Error message about invalid characters
- [ ] Duplicate key → Error from backend "Key already exists"

### Dirty Tracking
- [ ] Enter title "My Project"
- [ ] Click Cancel
- [ ] Confirmation dialog appears: "You have unsaved changes. Discard them?"
- [ ] Click "Keep Editing" → Stay in dialog
- [ ] Click Cancel again → Click "Discard" → Dialog closes

### Connection State
- [ ] Disconnect backend server
- [ ] Create button shows "Disconnected" state
- [ ] Button is disabled
- [ ] Reconnect server
- [ ] Button becomes enabled again

### Edit Project Flow
- [ ] Open existing project
- [ ] Click "Edit Project" (would need to add this button)
- [ ] Dialog opens with current Title, Key, and Description
- [ ] Key field is disabled (immutable)
- [ ] Modify title
- [ ] Click Save
- [ ] Success notification
- [ ] Changes reflected in UI

### Header Context Display
- [ ] Load Home page → header shows only "Agile Board"
- [ ] Click on Project A → header shows "Agile Board / Project A"
- [ ] Click Home button → header shows only "Agile Board"
- [ ] Click on Project B → header shows "Agile Board / Project B"
- [ ] Navigate to a work item within Project B → header STILL shows "Agile Board / Project B" (sticky)
- [ ] Click Home button → header shows only "Agile Board"

---

## Files Created/Modified

### Part 0 Files Created (Backend)
| File | Lines | Purpose |
|------|-------|---------|
| `backend/crates/pm-db/migrations/20260121000001_create_projects_table.sql` | ~150 | Migration to create pm_projects and update FKs |
| `backend/crates/pm-core/src/models/project.rs` | ~30 | Project domain model |
| `backend/crates/pm-db/src/repositories/project_repository.rs` | ~150 | Project CRUD repository |
| `backend/crates/pm-proto/src/project.proto` | ~60 | Project protobuf messages |
| `backend/crates/pm-ws/src/handlers/project.rs` | ~200 | Project WebSocket handlers |

### Part A/B Files Created (Frontend)
| File | Lines | Purpose |
|------|-------|---------|
| `frontend/ProjectManagement.Core/Models/Project.cs` | ~25 | Project domain model |
| `frontend/ProjectManagement.Core/ViewModels/ProjectViewModel.cs` | ~50 | Project-specific ViewModel |
| `frontend/ProjectManagement.Core/State/IProjectStore.cs` | ~35 | Interface for project state management |
| `frontend/ProjectManagement.Services/State/ProjectStore.cs` | ~70 | Implementation of project state store |
| `frontend/ProjectManagement.Components/Projects/ProjectDialog.razor` | ~280 | Dedicated Project create/edit dialog |

### Files Modified
| File | Lines Changed | Purpose |
|------|---------------|---------|
| `backend/crates/pm-core/src/models/mod.rs` | ~2 | Export Project model |
| `backend/crates/pm-core/src/models/work_item.rs` | ~5 | Remove Project from WorkItemType enum |
| `backend/crates/pm-ws/src/main.rs` | ~10 | Register Project handlers |
| `frontend/ProjectManagement.Services/State/AppState.cs` | ~15 | Add IProjectStore, LoadProjectsAsync() |
| `frontend/ProjectManagement.Wasm/Program.cs` | ~2 | Register ProjectStore in DI |
| `frontend/ProjectManagement.Wasm/Layout/MainLayout.razor` | ~20 | Show current project in header |
| `frontend/ProjectManagement.Wasm/Pages/ProjectDetail.razor` | ~5 | Set/clear current project on navigation |
| `frontend/ProjectManagement.Wasm/Pages/Home.razor` | ~15 | Use ProjectDialog, load from Projects store |

---

## Production-Grade Checklist

- [ ] **Error Handling**: All exceptions caught and handled gracefully
- [ ] **Validation**: Title required, key required and unique, length limits enforced
- [ ] **Connection Awareness**: Button disabled when disconnected
- [ ] **Dirty Tracking**: Warns before discarding unsaved changes
- [ ] **Version Conflicts**: Handles concurrent edits
- [ ] **Loading States**: Shows "Creating..." / "Saving..." during save
- [ ] **Notifications**: Success/error feedback for all operations
- [ ] **Accessibility**: Form fields have proper labels and ARIA attributes
- [ ] **Character Counters**: Visual feedback for length limits
- [ ] **Validation Feedback**: Real-time error messages
- [ ] **Migration Safety**: Data migration preserves all existing projects

---

## Success Criteria

### Part 0: Schema Refactor
- [ ] `pm_projects` table created with correct schema
- [ ] Existing projects migrated from `pm_work_items`
- [ ] All FK updates applied successfully
- [ ] `pm_work_items` no longer has 'project' in item_type CHECK
- [ ] Backend Project model created
- [ ] Backend ProjectRepository created
- [ ] Project protobuf messages defined
- [ ] Project WebSocket handlers working

### Part A: State Management
- [ ] ProjectViewModel created with only project-relevant properties
- [ ] IProjectStore interface returns ProjectViewModel (not WorkItem)
- [ ] ProjectStore implementation complete
- [ ] AppState exposes Projects store

### Part B: UI Integration
- [ ] ProjectDialog component created with Title + Key + Description
- [ ] Home.razor uses ProjectDialog for "Create Project" button
- [ ] Projects created successfully
- [ ] All validation rules enforced
- [ ] MainLayout header shows "Agile Board / {Project Name}" when project selected
- [ ] ProjectDetail sets CurrentProject on load
- [ ] Home.razor clears CurrentProject on load

### Integration Testing
- [ ] Navigate to project → header shows project name
- [ ] Navigate away → header shows only "Agile Board"
- [ ] Create new project → appears in Projects store
- [ ] Manual testing checklist passed

---

## Future Enhancements (Out of Scope)

### Add Edit Project Button
Currently, there's no way to edit a Project after creation. Future work:
- Add "Edit" button to Project detail page
- Add "Edit" menu item to Project context menu
- Use same ProjectDialog with `Project` parameter populated

### Project-Specific Fields
Future fields that could be added to ProjectDialog:
- Start/End dates (project timeline)
- Budget tracking
- Team members
- Custom fields

### Bulk Operations
- Archive/delete multiple projects
- Export project data
- Clone project structure

---

## Notes

### Why Promote Projects to First-Class Entities?

The original design stored Projects in `pm_work_items` with `item_type = 'project'`. This was problematic:

1. **Irrelevant fields**: Projects don't have Status (workflow), Priority, Story Points, Sprint, Assignee
2. **Self-reference hack**: `project_id = id` was required but not implemented
3. **Semantic confusion**: Projects aren't "work items" from a user perspective
4. **API confusion**: Same endpoint for fundamentally different entities
5. **Industry mismatch**: Every major tool (JIRA, Azure DevOps, etc.) has Projects as first-class entities

### Backend Changes Are Required

Unlike the original plan which said "backend remains unchanged", this refactored plan requires significant backend work:
- New `pm_projects` table
- Data migration
- FK updates
- New model, repository, proto, handlers

This is the right approach - fixing the data model now prevents technical debt.

---

## Session Completion

**Target Time**: ~5-6 hours (expanded due to schema work)
**Token Budget**: ~35k tokens (larger session)

**Completion Criteria:**
- Part 0: Schema refactor complete, migration tested
- Part A: Project state management working
- Part B: ProjectDialog.razor created and tested
- Home.razor loads from Projects store
- MainLayout header shows current project context
- All manual tests passing
- No regressions in existing functionality
