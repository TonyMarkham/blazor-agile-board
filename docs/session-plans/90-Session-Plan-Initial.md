# Session 90: Project-Level Work Item IDs (JIRA-Style)

Add JIRA-style identifiers to work items: `{ProjectKey}-{number}` (e.g., "BAG-1", "BAG-2").

## Design Decisions

1. **Compute display key at runtime** - Store only `item_number` (integer). The full key "BAG-123" is computed from `project.key + work_item.item_number`. This avoids redundancy and handles potential project key changes.

2. **Atomic counter per project** - Add `next_work_item_number` to `pm_projects` table. Use `UPDATE ... RETURNING` within a transaction to atomically assign numbers.

3. **Numbers start at 1** - Simple integers (1, 2, 3...), not zero-padded.

---

## Dependency Graph

```
=== Phase 1: ALL Database & Proto Changes (do these FIRST) ===

Step 1: Create migration file (Feature A)
    └── Adds: pm_projects.next_work_item_number, pm_work_items.item_number

Step 2: Update proto/messages.proto (Features A + C combined)
    ├── Feature A: Add item_number to WorkItem, next_work_item_number to Project
    └── Feature C: Add parent_id to UpdateWorkItemRequest

Step 3: Run migration  ←── CRITICAL: must run before Rust repos compile

=== Phase 2: Rust Domain Models (no deps) ===

Step 4: pm-core Project model (Feature A)
Step 5: pm-core WorkItem model (Feature A)

=== Phase 3: Rust DB Layer (depends on: migration + models) ===

Step 6: pm-db ProjectRepository (Feature A)
Step 7: pm-db WorkItemRepository (Feature A)

=== Phase 4: Rust WS Layer (depends on: repos + proto) ===

Step 8: pm-ws response_builder (Feature A)
Step 9: pm-ws work_item handler - create (Feature A)
Step 10: pm-ws change_tracker.rs (Feature C) - track parent changes
Step 11: pm-ws work_item.rs handle_update (Feature C) - validate hierarchy
Step 12: pm-ws work_item.rs apply_updates (Feature C) - apply parent changes
Step 13: pm-ws hierarchy_validator.rs (Feature B) - update docs

=== Phase 5: C# Domain Models (no deps) ===

Step 14: C# WorkItem model (Feature A)
Step 15: C# Project model (Feature A)
Step 16: C# UpdateWorkItemRequest (Feature C) - add ParentId property

=== Phase 6: C# Converter (depends on: proto + C# models) ===

Step 17: ProtoConverter (Feature A)

=== Phase 7: C# ViewModel (depends on: C# model) ===

Step 18: WorkItemViewModel (Feature A)

=== Phase 8: Blazor UI (depends on: ViewModel + all backend) ===

Step 19: KanbanCard.razor (Feature A)
Step 20: KanbanBoard.razor (Feature A)
Step 21: WorkItemRow.razor (Feature A)
Step 22: WorkItemDetail.razor (Feature A)
Step 23: WorkItemDialog.razor (Features B + C combined)
    ├── Feature B: Make parent optional on create
    └── Feature C: Show parent selection in edit mode
```

---

## Step-by-Step Implementation

### Step 1: Create Database Migration File

**File:** `backend/crates/pm-db/migrations/20260203000001_add_work_item_numbers.sql`

This migration must handle SQLite's limitation of not allowing `ALTER TABLE ... ADD COLUMN ... NOT NULL` without a default. We'll use table recreation.

```sql
-- ============================================================
-- Migration: Add work item numbers (JIRA-style IDs)
-- Adds: pm_projects.next_work_item_number
-- Adds: pm_work_items.item_number (unique per project)
-- ============================================================

PRAGMA foreign_keys = OFF;

-- Step 1: Add next_work_item_number to pm_projects
-- SQLite allows ADD COLUMN with DEFAULT, so this is safe
ALTER TABLE pm_projects ADD COLUMN next_work_item_number INTEGER NOT NULL DEFAULT 1;

-- Step 2: Create temp table to hold work items with new column
CREATE TEMPORARY TABLE temp_work_items AS
SELECT
    wi.*,
    ROW_NUMBER() OVER (PARTITION BY wi.project_id ORDER BY wi.created_at, wi.id) as item_number
FROM pm_work_items wi;

-- Step 3: Drop existing work items table
DROP TABLE pm_work_items;

-- Step 4: Recreate pm_work_items with item_number column (NOT NULL)
CREATE TABLE pm_work_items (
    id TEXT PRIMARY KEY,
    item_type TEXT NOT NULL CHECK(item_type IN ('epic', 'story', 'task')),
    parent_id TEXT,
    project_id TEXT NOT NULL,
    position INTEGER NOT NULL DEFAULT 0,
    title TEXT NOT NULL,
    description TEXT,
    status TEXT NOT NULL DEFAULT 'backlog' CHECK(status IN ('backlog', 'todo', 'in_progress', 'review', 'done', 'blocked')),
    priority TEXT NOT NULL DEFAULT 'medium' CHECK(priority IN ('critical', 'high', 'medium', 'low')),
    story_points INTEGER,
    assignee_id TEXT,
    sprint_id TEXT,
    item_number INTEGER NOT NULL,  -- NEW: Project-scoped sequential number
    version INTEGER NOT NULL DEFAULT 1,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    created_by TEXT NOT NULL,
    updated_by TEXT NOT NULL,
    deleted_at INTEGER,
    FOREIGN KEY (project_id) REFERENCES pm_projects(id) ON DELETE CASCADE,
    FOREIGN KEY (parent_id) REFERENCES pm_work_items(id) ON DELETE SET NULL,
    FOREIGN KEY (sprint_id) REFERENCES pm_sprints(id) ON DELETE SET NULL,
    FOREIGN KEY (assignee_id) REFERENCES users(id) ON DELETE SET NULL,
    UNIQUE(project_id, item_number)  -- Enforce uniqueness within project
);

-- Step 5: Copy data back with computed item_number
INSERT INTO pm_work_items (
    id, item_type, parent_id, project_id, position, title, description,
    status, priority, story_points, assignee_id, sprint_id, item_number,
    version, created_at, updated_at, created_by, updated_by, deleted_at
)
SELECT
    id, item_type, parent_id, project_id, position, title, description,
    status, priority, story_points, assignee_id, sprint_id, item_number,
    version, created_at, updated_at, created_by, updated_by, deleted_at
FROM temp_work_items;

-- Step 6: Update project counters to max item_number + 1
UPDATE pm_projects SET next_work_item_number = COALESCE(
    (SELECT MAX(item_number) + 1 FROM pm_work_items WHERE project_id = pm_projects.id),
    1
);

-- Step 7: Recreate indexes
CREATE INDEX idx_pm_work_items_project ON pm_work_items(project_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_work_items_parent ON pm_work_items(parent_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_work_items_sprint ON pm_work_items(sprint_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_work_items_status ON pm_work_items(status) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_work_items_assignee ON pm_work_items(assignee_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_work_items_type ON pm_work_items(item_type) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_work_items_item_number ON pm_work_items(project_id, item_number) WHERE deleted_at IS NULL;

-- Cleanup
DROP TABLE temp_work_items;

PRAGMA foreign_keys = ON;
```

---

### Step 2: Update Protobuf Messages

**File:** `proto/messages.proto`

**Why early?** Proto generates code for both Rust (`pm-proto` crate) and C# (`ProjectManagement.Core.Proto`). Must be done before response_builder.rs and ProtoConverter.cs.

#### 2.1 Add `item_number` to WorkItem message:

```protobuf
message WorkItem {
  string id = 1;
  WorkItemType item_type = 2;

  // Hierarchy
  optional string parent_id = 3;
  string project_id = 4;
  int32 position = 5;

  // Core fields
  string title = 6;
  optional string description = 7;

  // Workflow
  string status = 8;

  // Assignment
  optional string assignee_id = 9;

  // Sprint
  optional string sprint_id = 10;

  // Audit
  int64 created_at = 11;
  int64 updated_at = 12;
  string created_by = 13;
  string updated_by = 14;
  optional int64 deleted_at = 15;
  string priority = 16;
  optional int32 story_points = 17;
  int32 version = 18;
  int32 item_number = 19;  // NEW: Project-scoped sequential number
}
```

#### 2.2 Add `next_work_item_number` to Project message:

```protobuf
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
  int32 next_work_item_number = 12;  // NEW: Counter for work item numbers
}
```

#### 2.3 Add `parent_id` to UpdateWorkItemRequest (Feature C):

```protobuf
message UpdateWorkItemRequest {
  string work_item_id = 1;
  int32 expected_version = 2;
  optional string title = 3;
  optional string description = 4;
  optional string status = 5;
  optional string assignee_id = 6;
  optional string sprint_id = 7;
  optional int32 position = 8;
  optional string priority = 9;
  optional int32 story_points = 10;
  optional string parent_id = 11;  // NEW (Feature C): Change/assign/clear parent
}
```

**Semantics for `parent_id`:**
- Field not set: Don't change parent
- Empty string `""`: Clear parent (make orphan)
- UUID string: Set parent to specified work item

---

### Step 3: Run Database Migration

**CRITICAL:** This must be done before compiling Rust repository code. SQLx uses compile-time query checking against the actual database schema.

```bash
# Option A: If you have a dev database configured
sqlx migrate run --database-url "sqlite:///path/to/dev.db"

# Option B: Using the project's migration setup
# (Check justfile for specific command)

# Option C: If using sqlx offline mode, regenerate query cache after migration
cargo sqlx prepare --workspace
```

**Verification:**
```bash
# Confirm new columns exist
sqlite3 /path/to/dev.db ".schema pm_projects" | grep next_work_item_number
sqlite3 /path/to/dev.db ".schema pm_work_items" | grep item_number
```

---

### Step 4: Update Rust Domain Model - Project

**File:** `backend/crates/pm-core/src/models/project.rs`

**Depends on:** Nothing

Add `next_work_item_number` field to `Project` struct.

```rust
// Add new field to Project struct (after deleted_at):
pub next_work_item_number: i32,

// Update Project::new() to initialize next_work_item_number:
impl Project {
    pub fn new(title: String, key: String, created_by: Uuid) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            title,
            description: None,
            key,
            status: ProjectStatus::Active,
            version: 1,
            created_at: now,
            updated_at: now,
            created_by,
            updated_by: created_by,
            deleted_at: None,
            next_work_item_number: 1,  // NEW
        }
    }
    // ... existing methods unchanged
}
```

---

### Step 5: Update Rust Domain Model - WorkItem

**File:** `backend/crates/pm-core/src/models/work_item.rs`

**Depends on:** Nothing

Add `item_number` field and display key helper method.

```rust
// Add new field to WorkItem struct (before version field):
pub item_number: i32,

// Update WorkItem::new() - item_number will be assigned during DB insert
impl WorkItem {
    pub fn new(
        item_type: WorkItemType,
        title: String,
        description: Option<String>,
        parent_id: Option<Uuid>,
        project_id: Uuid,
        created_by: Uuid,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            item_type,
            parent_id,
            project_id,
            position: 0,
            title,
            description,
            status: "backlog".to_string(),
            priority: "medium".to_string(),
            assignee_id: None,
            story_points: None,
            sprint_id: None,
            item_number: 0,  // NEW: Will be set during DB insert
            version: 0,
            created_at: now,
            updated_at: now,
            created_by,
            updated_by: created_by,
            deleted_at: None,
        }
    }

    /// Generate JIRA-style display key (e.g., "PROJ-123")
    pub fn display_key(&self, project_key: &str) -> String {
        format!("{}-{}", project_key, self.item_number)
    }
}
```

---

### Step 6: Update Project Repository

**File:** `backend/crates/pm-db/src/repositories/project_repository.rs`

**Depends on:** Step 3 (migration run), Step 4 (Project model)

Add `next_work_item_number` to all queries and add atomic increment method.

#### 6.1 Update `create()` method:

```rust
pub async fn create(&self, project: &Project) -> DbErrorResult<()> {
    let id = project.id.to_string();
    let status = project.status.as_str();
    let created_at = project.created_at.timestamp();
    let updated_at = project.updated_at.timestamp();
    let created_by = project.created_by.to_string();
    let updated_by = project.updated_by.to_string();
    let deleted_at = project.deleted_at.map(|dt| dt.timestamp());

    sqlx::query!(
        r#"
            INSERT INTO pm_projects (
                id, title, description, key, status, version,
                created_at, updated_at, created_by, updated_by, deleted_at,
                next_work_item_number
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        id,
        project.title,
        project.description,
        project.key,
        status,
        project.version,
        created_at,
        updated_at,
        created_by,
        updated_by,
        deleted_at,
        project.next_work_item_number,  // NEW
    )
    .execute(&self.pool)
    .await?;

    Ok(())
}
```

#### 6.2 Update all `find_*()` methods to include `next_work_item_number`:

For each SELECT query, add `next_work_item_number` to the column list:

```rust
// In find_by_id, find_by_key, find_all, find_active:
let row = sqlx::query!(
    r#"
        SELECT id, title, description, key, status, version,
               created_at, updated_at, created_by, updated_by, deleted_at,
               next_work_item_number
        FROM pm_projects
        WHERE id = ? AND deleted_at IS NULL
        "#,
    id_str
)
// ...

// And in the mapping closure, add:
next_work_item_number: r.next_work_item_number as i32,
```

#### 6.3 Add new atomic increment method:

```rust
/// Atomically get and increment the work item number for a project.
/// Returns the number to assign to the new work item.
/// Must be called within a transaction for atomicity.
pub async fn get_and_increment_work_item_number<'e, E>(
    executor: E,
    project_id: Uuid,
) -> DbErrorResult<i32>
where
    E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
{
    let project_id_str = project_id.to_string();

    // SQLite doesn't support RETURNING, so we need two queries in a transaction
    // The transaction isolation ensures atomicity

    // First, get the current value
    let current = sqlx::query_scalar!(
        r#"
            SELECT next_work_item_number as "next_work_item_number!"
            FROM pm_projects
            WHERE id = ? AND deleted_at IS NULL
            "#,
        project_id_str
    )
    .fetch_one(executor)
    .await
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => DbError::NotFound {
            message: format!("Project {} not found", project_id),
            location: ErrorLocation::from(Location::caller()),
        },
        _ => DbError::from(e),
    })?;

    let item_number = current as i32;
    let next_number = item_number + 1;

    // Then increment
    sqlx::query!(
        r#"
            UPDATE pm_projects
            SET next_work_item_number = ?
            WHERE id = ? AND deleted_at IS NULL
            "#,
        next_number,
        project_id_str
    )
    .execute(executor)
    .await?;

    Ok(item_number)
}
```

**Note:** Since SQLite doesn't support `UPDATE ... RETURNING`, we use two queries. The caller must ensure these run within a transaction for atomicity.

---

### Step 7: Update WorkItem Repository

**File:** `backend/crates/pm-db/src/repositories/work_item_repository.rs`

**Depends on:** Step 3 (migration run), Step 5 (WorkItem model)

#### 7.1 Update `create()` to include `item_number`:

```rust
pub async fn create<'e, E>(executor: E, work_item: &WorkItem) -> DbErrorResult<()>
where
    E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
{
    let id = work_item.id.to_string();
    let item_type = work_item.item_type.as_str();
    let parent_id = work_item.parent_id.map(|id| id.to_string());
    let project_id = work_item.project_id.to_string();
    let assignee_id = work_item.assignee_id.map(|id| id.to_string());
    let sprint_id = work_item.sprint_id.map(|id| id.to_string());
    let created_at = work_item.created_at.timestamp();
    let updated_at = work_item.updated_at.timestamp();
    let created_by = work_item.created_by.to_string();
    let updated_by = work_item.updated_by.to_string();
    let deleted_at = work_item.deleted_at.map(|dt| dt.timestamp());

    sqlx::query!(
        r#"
          INSERT INTO pm_work_items (
              id, item_type, parent_id, project_id, position,
              title, description, status, priority, assignee_id,
              story_points, sprint_id, item_number, version,
              created_at, updated_at, created_by, updated_by, deleted_at
          ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
          "#,
        id,
        item_type,
        parent_id,
        project_id,
        work_item.position,
        work_item.title,
        work_item.description,
        work_item.status,
        work_item.priority,
        assignee_id,
        work_item.story_points,
        sprint_id,
        work_item.item_number,  // NEW
        work_item.version,
        created_at,
        updated_at,
        created_by,
        updated_by,
        deleted_at,
    )
    .execute(executor)
    .await?;

    Ok(())
}
```

#### 7.2 Update all `find_*()` SELECT queries to include `item_number`:

```rust
// Add to SELECT columns:
SELECT
    id, item_type, parent_id, project_id, position,
    title, description, status, priority, assignee_id,
    story_points, sprint_id, item_number, version,
    created_at, updated_at, created_by, updated_by, deleted_at
FROM pm_work_items
// ...

// Add to mapping:
item_number: r.item_number as i32,
```

Apply this change to:
- `find_by_id()`
- `find_by_project()`
- `find_children()`
- `find_by_project_since()`

#### 7.3 Add new lookup method by project + item number:

```rust
/// Find a work item by project ID and item number (e.g., for "PROJ-123" lookup)
pub async fn find_by_project_and_number<'e, E>(
    executor: E,
    project_id: Uuid,
    item_number: i32,
) -> DbErrorResult<Option<WorkItem>>
where
    E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
{
    let project_id_str = project_id.to_string();

    let row = sqlx::query!(
        r#"
          SELECT
              id, item_type, parent_id, project_id, position,
              title, description, status, priority, assignee_id,
              story_points, sprint_id, item_number, version,
              created_at, updated_at, created_by, updated_by, deleted_at
          FROM pm_work_items
          WHERE project_id = ? AND item_number = ? AND deleted_at IS NULL
          "#,
        project_id_str,
        item_number
    )
    .fetch_optional(executor)
    .await?;

    row.map(|r| -> DbErrorResult<WorkItem> {
        Ok(WorkItem {
            // ... same mapping as find_by_id
            item_number: r.item_number as i32,
            // ... rest of fields
        })
    })
    .transpose()
}
```

---

### Step 8: Update Response Builder

**File:** `backend/crates/pm-ws/src/handlers/response_builder.rs`

**Depends on:** Step 2 (proto), Step 4 (Project model), Step 5 (WorkItem model)

#### 8.1 Update `work_item_to_proto()`:

```rust
/// Convert domain WorkItem to proto WorkItem
fn work_item_to_proto(item: &WorkItem) -> PmProtoWorkItem {
    PmProtoWorkItem {
        id: item.id.to_string(),
        item_type: item.item_type.clone() as i32,
        title: item.title.clone(),
        description: item.description.clone(),
        status: item.status.clone(),
        priority: item.priority.clone(),
        parent_id: item.parent_id.map(|id| id.to_string()),
        project_id: item.project_id.to_string(),
        assignee_id: item.assignee_id.map(|id| id.to_string()),
        story_points: item.story_points,
        position: item.position,
        sprint_id: item.sprint_id.map(|id| id.to_string()),
        item_number: item.item_number,  // NEW
        version: item.version,
        created_at: item.created_at.timestamp(),
        updated_at: item.updated_at.timestamp(),
        created_by: item.created_by.to_string(),
        updated_by: item.updated_by.to_string(),
        deleted_at: item.deleted_at.map(|dt| dt.timestamp()),
    }
}
```

#### 8.2 Update `project_to_proto()`:

```rust
/// Convert domain Project to proto Project
fn project_to_proto(project: &Project) -> ProtoProject {
    ProtoProject {
        id: project.id.to_string(),
        title: project.title.clone(),
        description: project.description.clone(),
        key: project.key.clone(),
        status: match project.status {
            ProjectStatus::Active => ProtoProjectStatus::Active.into(),
            ProjectStatus::Archived => ProtoProjectStatus::Archived.into(),
        },
        version: project.version,
        created_at: project.created_at.timestamp(),
        updated_at: project.updated_at.timestamp(),
        created_by: project.created_by.to_string(),
        updated_by: project.updated_by.to_string(),
        deleted_at: project.deleted_at.map(|dt| dt.timestamp()),
        next_work_item_number: project.next_work_item_number,  // NEW
    }
}
```

---

### Step 9: Update WorkItem WebSocket Handler

**File:** `backend/crates/pm-ws/src/handlers/work_item.rs`

**Depends on:** Step 5 (WorkItem model), Step 6 (ProjectRepository), Step 7 (WorkItemRepository)

Modify `handle_create()` to atomically assign item numbers within a transaction.

```rust
/// Handle CreateWorkItemRequest with full production features
pub async fn handle_create(
    req: CreateWorkItemRequest,
    ctx: HandlerContext,
) -> WsErrorResult<WebSocketMessage> {
    debug!("{} CreateWorkItem starting", ctx.log_prefix());

    // ... existing validation code (steps 1-6) ...

    // 7. Get next position
    let max_position = db_read(&ctx, "find_max_position", || async {
        WorkItemRepository::find_max_position(&ctx.pool, project_id, parent_id)
            .await
            .map_err(WsError::from)
    })
    .await?;

    // 8. Build work item (item_number will be set in transaction)
    let now = Utc::now();
    let mut work_item = WorkItem {
        id: Uuid::new_v4(),
        item_type: item_type.clone(),
        parent_id,
        project_id,
        position: max_position + 1,
        title: sanitize_string(&req.title),
        description: req.description.as_ref().map(|d| sanitize_string(d)),
        status: req.status.clone().unwrap_or_else(|| "backlog".to_string()),
        priority: req.priority.clone().unwrap_or_else(|| "medium".to_string()),
        assignee_id: None,
        story_points: None,
        sprint_id: None,
        item_number: 0,  // Will be set in transaction
        version: 1,
        created_at: now,
        updated_at: now,
        created_by: ctx.user_id,
        updated_by: ctx.user_id,
        deleted_at: None,
    };

    // 9. Execute transaction with atomic item_number assignment
    let activity = ActivityLog::created("work_item", work_item.id, ctx.user_id);
    let activity_clone = activity.clone();

    // Clone for the closure
    let work_item_id = work_item.id;
    let work_item_for_tx = work_item.clone();

    let item_number = db_write(&ctx, "create_work_item_tx", || async {
        let mut tx = ctx.pool.begin().await?;

        // Atomically get and increment the work item number
        let item_num = ProjectRepository::get_and_increment_work_item_number(&mut *tx, project_id)
            .await
            .map_err(WsError::from)?;

        // Create work item with assigned number
        let mut wi = work_item_for_tx.clone();
        wi.item_number = item_num;
        WorkItemRepository::create(&mut *tx, &wi).await?;

        // Create activity log
        ActivityLogRepository::create(&mut *tx, &activity_clone).await?;

        tx.commit().await?;
        Ok::<_, WsError>(item_num)
    })
    .await?;

    // Update our local copy with the assigned number
    work_item.item_number = item_number;

    // ... rest of the function (broadcast, response building) unchanged ...

    info!(
        "{} Created work item {} ({:?}) #{} in project {}",
        ctx.log_prefix(),
        work_item.id,
        item_type,
        item_number,
        project_id
    );

    Ok(response)
}
```

---

### Step 14: Update C# Domain Model - WorkItem

**File:** `frontend/ProjectManagement.Core/Models/WorkItem.cs`

**Depends on:** Nothing

```csharp
public sealed record WorkItem :
    IAuditable,
    IProjectScoped,
    IVersioned,
    IPositioned,
    IHierarchical<WorkItem>,
    ISprintAssignable,
    IUserAssignable,
    IStatusTracked
{
    public WorkItemType ItemType { get; init; }
    public string Title { get; init; } = string.Empty;
    public string? Description { get; init; }
    public string Priority { get; init; } = "medium";
    public int? StoryPoints { get; init; }
    public Guid Id { get; init; }
    public DateTime CreatedAt { get; init; }
    public DateTime UpdatedAt { get; init; }
    public Guid CreatedBy { get; init; }
    public Guid UpdatedBy { get; init; }
    public DateTime? DeletedAt { get; init; }
    public Guid? ParentId { get; init; }
    public int Position { get; init; }
    public Guid ProjectId { get; init; }
    public Guid? SprintId { get; init; }
    public string Status { get; init; } = "backlog";
    public Guid? AssigneeId { get; init; }
    public int Version { get; init; }

    /// <summary>
    /// Project-scoped sequential number (1, 2, 3...).
    /// Combined with project key forms the display ID (e.g., "PROJ-123").
    /// </summary>
    public int ItemNumber { get; init; }  // NEW

    /// <summary>
    /// Generate the JIRA-style display key (e.g., "PROJ-123").
    /// </summary>
    public string GetDisplayKey(string projectKey) => $"{projectKey}-{ItemNumber}";
}
```

---

### Step 15: Update C# Domain Model - Project

**File:** `frontend/ProjectManagement.Core/Models/Project.cs`

**Depends on:** Nothing

```csharp
public sealed record Project
{
    // ... existing properties ...

    /// <summary>
    /// When the project was soft-deleted (null if not deleted).
    /// </summary>
    public DateTime? DeletedAt { get; init; }

    /// <summary>
    /// Next sequential number to assign to work items.
    /// Atomically incremented when creating work items.
    /// </summary>
    public int NextWorkItemNumber { get; init; } = 1;  // NEW

    /// <summary>
    /// Check if the project is deleted.
    /// </summary>
    public bool IsDeleted => DeletedAt.HasValue;

    /// <summary>
    /// Check if the project is archived.
    /// </summary>
    public bool IsArchived => Status == ProjectStatus.Archived;
}
```

---

### Step 17: Update C# ProtoConverter

**File:** `frontend/ProjectManagement.Core/Converters/ProtoConverter.cs`

**Depends on:** Step 2 (proto), Step 10 (C# WorkItem), Step 11 (C# Project)

#### 12.1 Update `ToDomain(ProtoWorkItem)`:

```csharp
public static DomainWorkItem ToDomain(ProtoWorkItem proto)
{
    ArgumentNullException.ThrowIfNull(proto);

    return new DomainWorkItem
    {
        Id = ParseGuid(proto.Id, "WorkItem.Id"),
        ItemType = ToDomain(proto.ItemType),
        ParentId = string.IsNullOrEmpty(proto.ParentId) ? null : ParseGuid(proto.ParentId, "WorkItem.ParentId"),
        ProjectId = ParseGuid(proto.ProjectId, "WorkItem.ProjectId"),
        Position = proto.Position,
        Title = proto.Title ?? string.Empty,
        Description = string.IsNullOrEmpty(proto.Description) ? null : proto.Description,
        Status = proto.Status ?? "backlog",
        Priority = proto.Priority ?? "medium",
        AssigneeId = string.IsNullOrEmpty(proto.AssigneeId)
            ? null
            : ParseGuid(proto.AssigneeId, "WorkItem.AssigneeId"),
        StoryPoints = proto.StoryPoints == 0 ? null : proto.StoryPoints,
        SprintId = string.IsNullOrEmpty(proto.SprintId) ? null : ParseGuid(proto.SprintId, "WorkItem.SprintId"),
        ItemNumber = proto.ItemNumber,  // NEW
        Version = proto.Version,
        CreatedAt = FromUnixTimestamp(proto.CreatedAt),
        UpdatedAt = FromUnixTimestamp(proto.UpdatedAt),
        CreatedBy = ParseGuid(proto.CreatedBy, "WorkItem.CreatedBy"),
        UpdatedBy = ParseGuid(proto.UpdatedBy, "WorkItem.UpdatedBy"),
        DeletedAt = proto.DeletedAt == 0 ? null : FromUnixTimestamp(proto.DeletedAt)
    };
}
```

#### 12.2 Update `ToProto(DomainWorkItem)`:

```csharp
public static ProtoWorkItem ToProto(DomainWorkItem domain)
{
    ArgumentNullException.ThrowIfNull(domain);

    var proto = new ProtoWorkItem
    {
        Id = domain.Id.ToString(),
        ItemType = ToProto(domain.ItemType),
        ProjectId = domain.ProjectId.ToString(),
        Position = domain.Position,
        Title = domain.Title,
        Status = domain.Status,
        Priority = domain.Priority,
        ItemNumber = domain.ItemNumber,  // NEW
        Version = domain.Version,
        CreatedAt = ToUnixTimestamp(domain.CreatedAt),
        UpdatedAt = ToUnixTimestamp(domain.UpdatedAt),
        CreatedBy = domain.CreatedBy.ToString(),
        UpdatedBy = domain.UpdatedBy.ToString()
    };

    // ... rest unchanged ...
    return proto;
}
```

#### 12.3 Update `ToDomain(Proto.Project)`:

```csharp
public static Project ToDomain(Proto.Project proto)
{
    ArgumentNullException.ThrowIfNull(proto);

    return new Project
    {
        Id = ParseGuid(proto.Id, "Project.Id"),
        Title = proto.Title ?? string.Empty,
        Description = string.IsNullOrEmpty(proto.Description) ? null : proto.Description,
        Key = proto.Key ?? string.Empty,
        Status = proto.Status switch
        {
            Proto.ProjectStatus.Active => ProjectStatus.Active,
            Proto.ProjectStatus.Archived => ProjectStatus.Archived,
            _ => ProjectStatus.Active
        },
        Version = proto.Version,
        CreatedAt = FromUnixTimestamp(proto.CreatedAt),
        UpdatedAt = FromUnixTimestamp(proto.UpdatedAt),
        CreatedBy = ParseGuid(proto.CreatedBy, "Project.CreatedBy"),
        UpdatedBy = ParseGuid(proto.UpdatedBy, "Project.UpdatedBy"),
        DeletedAt = proto.DeletedAt == 0 ? null : FromUnixTimestamp(proto.DeletedAt),
        NextWorkItemNumber = proto.NextWorkItemNumber,  // NEW
    };
}
```

---

### Step 18: Update WorkItemViewModel

**File:** `frontend/ProjectManagement.Core/ViewModels/WorkItemViewModel.cs`

**Depends on:** Step 10 (C# WorkItem model)

Expose `ItemNumber` from the underlying model.

```csharp
// Add to the Identity section (after Version):

/// <summary>
/// Project-scoped sequential number (1, 2, 3...).
/// </summary>
public int ItemNumber => Model.ItemNumber;

// Add helper method for display key:

/// <summary>
/// Generate the JIRA-style display key (e.g., "PROJ-123").
/// </summary>
public string GetDisplayKey(string projectKey) => Model.GetDisplayKey(projectKey);
```

---

### Step 19: Update KanbanCard.razor

**File:** `frontend/ProjectManagement.Components/WorkItems/KanbanCard.razor`

**Depends on:** Step 13 (WorkItemViewModel)

Add project key parameter and display the identifier.

```razor
@* Add parameter in @code section: *@
[Parameter]
public string? ProjectKey { get; set; }

@* Update the Header section to show the ID: *@
@* Header: Type + Title *@
<RadzenStack Orientation="Orientation.Horizontal"
             AlignItems="AlignItems.Start"
             Gap="0.5rem">
    <WorkItemTypeIcon Type="@Item.ItemType" Size="1rem" />
    <RadzenStack Gap="0">
        @if (!string.IsNullOrEmpty(ProjectKey))
        {
            <span class="kanban-card-id">@Item.GetDisplayKey(ProjectKey)</span>
        }
        <span class="kanban-card-title">@Item.Title</span>
    </RadzenStack>
</RadzenStack>

@* Update AriaLabel to include the ID: *@
private string AriaLabel
{
    get
    {
        var id = !string.IsNullOrEmpty(ProjectKey) ? $"{Item.GetDisplayKey(ProjectKey)}: " : "";
        var label = $"{id}{Item.ItemTypeDisplayName}: {Item.Title}, Priority: {Item.PriorityDisplayName}";
        // ... rest unchanged
    }
}
```

**CSS (add to component or shared styles):**
```css
.kanban-card-id {
    font-size: 0.75rem;
    color: var(--rz-text-tertiary-color);
    font-family: var(--rz-font-family-monospace);
}
```

---

### Step 20: Update KanbanBoard.razor

**File:** `frontend/ProjectManagement.Components/WorkItems/KanbanBoard.razor`

**Depends on:** Step 14 (KanbanCard)

Pass `ProjectKey` to KanbanCard components.

```razor
@* Add field to store project key: *@
private string? _projectKey;

@* In RefreshData(), look up the project: *@
private void RefreshData()
{
    var project = AppState.Projects.GetById(ProjectId);
    _projectKey = project?.Key;

    var items = AppState.WorkItems.GetByProject(ProjectId)
        .Where(w => w.DeletedAt is null);

    _allItems = ViewModelFactory.CreateMany(items).ToList();
    ApplyFilters();
}

@* Pass to KanbanCard: *@
<KanbanCard Item="@item"
            ProjectKey="@_projectKey"
            IsConnected="@_isConnected"
            OnClick="@HandleCardClick"
            OnEdit="@HandleCardEdit" />
```

---

### Step 21: Update WorkItemRow.razor

**File:** `frontend/ProjectManagement.Components/WorkItems/WorkItemRow.razor`

**Depends on:** Step 13 (WorkItemViewModel)

Add project key parameter and display the identifier as a new cell.

```razor
@* Add parameter in @code section: *@
[Parameter]
public string? ProjectKey { get; set; }

@* Add ID Cell before Type Cell: *@
@* ID Cell *@
<div class="work-item-cell id-cell" role="cell">
    @if (!string.IsNullOrEmpty(ProjectKey))
    {
        <span class="work-item-id">@Item.GetDisplayKey(ProjectKey)</span>
    }
</div>

@* Type Cell *@
<div class="work-item-cell type-cell" role="cell">
    ...

@* Update AriaLabel to include the ID: *@
private string AriaLabel
{
    get
    {
        var parts = new List<string>();

        if (!string.IsNullOrEmpty(ProjectKey))
        {
            parts.Add(Item.GetDisplayKey(ProjectKey));
        }

        parts.Add(Item.ItemTypeDisplayName);
        parts.Add(Item.Title);
        // ... rest unchanged
    }
}
```

**CSS:**
```css
.work-item-id {
    font-family: var(--rz-font-family-monospace);
    font-size: 0.85rem;
    color: var(--rz-text-secondary-color);
}

.id-cell {
    width: 80px;
    flex-shrink: 0;
}
```

---

### Step 22: Update WorkItemDetail.razor

**File:** `frontend/ProjectManagement.Wasm/Pages/WorkItemDetail.razor`

**Depends on:** Step 13 (WorkItemViewModel), Step 16 (WorkItemRow)

Display the identifier prominently in the header and sidebar.

```razor
@* Update Header section: *@
<div class="page-header">
    <RadzenStack Orientation="Orientation.Horizontal"
                 AlignItems="AlignItems.Center"
                 Gap="0.75rem">
        <WorkItemTypeIcon Type="@_workItem.ItemType" Size="1.75rem" />
        <div>
            @if (_project is not null)
            {
                <span class="work-item-key">@_workItem.GetDisplayKey(_project.Key)</span>
            }
            <h1 class="page-title">@_workItem.Title</h1>
            <RadzenStack Orientation="Orientation.Horizontal"
                         Gap="0.5rem"
                         class="mt-1">
                <WorkItemStatusBadge Status="@_workItem.Status" />
                <PriorityBadge Priority="@_workItem.Priority" />
                @if (_workItem.StoryPoints.HasValue)
                {
                    <RadzenBadge BadgeStyle="BadgeStyle.Info"
                                 Text="@($"{_workItem.StoryPoints} pts")" />
                }
            </RadzenStack>
        </div>
    </RadzenStack>
    ...
</div>

@* Update Sidebar Details section - add ID as first item: *@
<div class="content-card">
    <h3 class="content-card-title">Details</h3>
    <RadzenStack Gap="1rem">
        @if (_project is not null)
        {
            <div>
                <RadzenText TextStyle="TextStyle.Caption" class="text-muted">ID</RadzenText>
                <RadzenText Style="font-family: var(--rz-font-family-monospace);">
                    @_workItem.GetDisplayKey(_project.Key)
                </RadzenText>
            </div>
        }
        <div>
            <RadzenText TextStyle="TextStyle.Caption" class="text-muted">Type</RadzenText>
            <RadzenText>@_workItem.ItemTypeDisplayName</RadzenText>
        </div>
        ...
    </RadzenStack>
</div>

@* Update Child Items section to pass ProjectKey: *@
@foreach (var child in _children)
{
    <WorkItemRow Item="@child"
                 ProjectKey="@_project?.Key"
                 IsConnected="@_isConnected"
                 OnSelect="@(item => NavigationManager.NavigateTo($"/workitem/{item.Id}"))"
                 OnEdit="@HandleEditChild"
                 OnDelete="@HandleDeleteChild" />
}

@* Update breadcrumb to use display key: *@
<nav class="breadcrumbs" aria-label="Breadcrumb">
    <a href="/">Home</a>
    <span class="separator">/</span>
    @if (_project is not null)
    {
        <a href="@($"/project/{_project.Id}")">@_project.Title</a>
        <span class="separator">/</span>
        <span class="current">@_workItem.GetDisplayKey(_project.Key)</span>
    }
    else
    {
        <span class="current">@_workItem.Title</span>
    }
</nav>
```

**CSS:**
```css
.work-item-key {
    font-family: var(--rz-font-family-monospace);
    font-size: 0.875rem;
    color: var(--rz-text-secondary-color);
    display: block;
    margin-bottom: 0.25rem;
}
```

---

## Files Summary (All Features)

| Step | Layer | File | Feature | Depends On |
|------|-------|------|---------|------------|
| 1 | DB | `migrations/20260203000001_add_work_item_numbers.sql` | A | - |
| 2 | Proto | `proto/messages.proto` | A, C | - |
| 3 | DB | **Run migration** | A | Step 1 |
| 4 | Rust | `pm-core/src/models/project.rs` | A | - |
| 5 | Rust | `pm-core/src/models/work_item.rs` | A | - |
| 6 | Rust | `pm-db/src/repositories/project_repository.rs` | A | Steps 3, 4 |
| 7 | Rust | `pm-db/src/repositories/work_item_repository.rs` | A | Steps 3, 5 |
| 8 | Rust | `pm-ws/src/handlers/response_builder.rs` | A | Steps 2, 4, 5 |
| 9 | Rust | `pm-ws/src/handlers/work_item.rs` (create) | A | Steps 5, 6, 7 |
| 10 | Rust | `pm-ws/src/handlers/change_tracker.rs` | C | Step 2 |
| 11 | Rust | `pm-ws/src/handlers/work_item.rs` (handle_update) | C | Steps 5, 10 |
| 12 | Rust | `pm-ws/src/handlers/work_item.rs` (apply_updates) | C | Step 11 |
| 13 | Rust | `pm-ws/src/handlers/hierarchy_validator.rs` | B | - |
| 14 | C# | `ProjectManagement.Core/Models/WorkItem.cs` | A | - |
| 15 | C# | `ProjectManagement.Core/Models/Project.cs` | A | - |
| 16 | C# | `ProjectManagement.Core/Models/UpdateWorkItemRequest.cs` | C | - |
| 17 | C# | `ProjectManagement.Core/Converters/ProtoConverter.cs` | A | Steps 2, 14, 15 |
| 18 | C# | `ProjectManagement.Core/ViewModels/WorkItemViewModel.cs` | A | Step 14 |
| 19 | Blazor | `ProjectManagement.Components/WorkItems/KanbanCard.razor` | A | Step 18 |
| 20 | Blazor | `ProjectManagement.Components/WorkItems/KanbanBoard.razor` | A | Step 19 |
| 21 | Blazor | `ProjectManagement.Components/WorkItems/WorkItemRow.razor` | A | Step 18 |
| 22 | Blazor | `ProjectManagement.Wasm/Pages/WorkItemDetail.razor` | A | Steps 18, 21 |
| 23 | Blazor | `ProjectManagement.Components/WorkItems/WorkItemDialog.razor` | B, C | Steps 12, 16, 18 |

---

## Verification Checklist

### After Step 3 (Migration)
```bash
# Verify schema updated
sqlite3 /path/to/dev.db ".schema pm_projects" | grep next_work_item_number
sqlite3 /path/to/dev.db ".schema pm_work_items" | grep item_number
```

### After Step 13 (Rust complete)
```bash
# Check Rust compiles
just check-backend

# Run clippy (fails on warnings)
just clippy-backend

# Run all backend tests
just test-backend
```

### After Step 23 (Frontend complete)
```bash
# Build frontend
just build-frontend

# Run frontend tests
just test-frontend
```

### Integration Test
1. Create a new project with key "TEST"
2. Create 3 work items (epic, story, task)
3. Verify they get sequential numbers 1, 2, 3
4. Verify display as "TEST-1", "TEST-2", "TEST-3"
5. Create work items in a different project
6. Verify numbering is independent per project

### Migration Test
1. Backup existing database
2. Run migration
3. Verify existing work items have sequential numbers by creation order
4. Verify project counters are correctly set to max + 1

### UI Verification
1. **Kanban Board**: Each card shows "PROJ-N" identifier above/before title
2. **Work Item Detail Page**:
   - Header shows "PROJ-N" above the title
   - Sidebar Details section shows "ID: PROJ-N" as first item
   - Breadcrumb shows "Home / Project Title / PROJ-N"
   - Child items list shows ID column
3. **Work Item Row**: ID column shows "PROJ-N" for each item
4. **Accessibility**: ARIA labels include the identifier for screen readers

---

## Feature B: Orphan Stories and Tasks

Allow Stories and Tasks to exist without a parent, belonging directly to a project. This makes the hierarchy optional rather than mandatory.

### Current State Analysis

| Layer | Enforces Hierarchy? | Details |
|-------|---------------------|---------|
| **Database (SQLite)** | ❌ No | `parent_id` is optional (`TEXT` without `NOT NULL`) |
| **Rust Backend** | ⚠️ Conditional | `validate_hierarchy()` only runs IF `parent_id` is provided; skipped when null |
| **Frontend Dialog** | ✅ Yes | UI requires parent selection for Story/Task; blocks submission without it |

**Key Finding:** The database and backend already support orphan items. Only the UI enforces the hierarchy.

### Implementation

**Note:** WorkItemDialog.razor changes are combined with Feature C in **Step 23**.

#### Step 23 (Feature B portion): Update WorkItemDialog.razor

**File:** `frontend/ProjectManagement.Components/WorkItems/WorkItemDialog.razor`

**Depends on:** Nothing (independent of JIRA-style IDs feature)

**Change 1:** Make parent dropdown optional by adding `AllowClear="true"`:

```razor
@* Parent Selection (for Story/Task) - Optional *@
@if (!_isEdit && (_itemType == WorkItemType.Story || _itemType == WorkItemType.Task))
{
    <RadzenFormField Text="@GetParentFieldLabel()" Style="width: 100%;">
        <RadzenDropDown @bind-Value="_selectedParentId"
                        TValue="Guid?"
                        Data="@_availableParents"
                        TextProperty="Title"
                        ValueProperty="Id"
                        AllowClear="true"
                        Placeholder="@GetParentPlaceholder()"
                        Style="width: 100%;" />
    </RadzenFormField>
}
```

**Change 2:** Remove the parent validation in `Validate()` method (lines 295-301):

```csharp
private bool Validate()
{
    _errors.Clear();

    var trimmed = _title?.Trim() ?? "";
    if (string.IsNullOrWhiteSpace(trimmed))
    {
        _errors["Title"] = "Title is required";
    }
    else if (trimmed.Length > 200)
    {
        _errors["Title"] = "Title must be 200 characters or less";
    }

    // REMOVED: Parent validation that enforced hierarchy
    // Stories and Tasks can now be orphans (belong directly to project)

    return _errors.Count == 0;
}
```

**Change 3:** Also remove the error display markup (lines 35-41):

```razor
@* Remove this block: *@
@if (_errors.TryGetValue("Parent", out var parentError))
{
    <RadzenText TextStyle="TextStyle.Caption"
                Style="color: var(--rz-danger);">
        @parentError
    </RadzenText>
}
```

**Change 4:** Update placeholder text in `GetParentPlaceholder()`:

```csharp
private string GetParentPlaceholder()
{
    return _itemType switch
    {
        WorkItemType.Story => "Select an Epic (optional)",
        WorkItemType.Task => "Select a Story (optional)",
        _ => "Select a parent (optional)"
    };
}
```

**Change 5:** Update label in `GetParentFieldLabel()`:

```csharp
private string GetParentFieldLabel()
{
    return _itemType switch
    {
        WorkItemType.Story => "Parent Epic (optional)",
        WorkItemType.Task => "Parent Story (optional)",
        _ => "Parent (optional)"
    };
}
```

---

#### Step 13: Update Rust hierarchy_validator.rs (Documentation Only)

**File:** `backend/crates/pm-ws/src/handlers/hierarchy_validator.rs`

**Why:** Clarify in code comments that orphan items are intentional, not a bug.

```rust
/// Valid parent-child relationships when a parent IS specified:
/// - Epic: no parent (parent_id should be NULL)
/// - Story: parent must be Epic
/// - Task: parent must be Story
///
/// **Important:** Stories and Tasks CAN be orphans (parent_id = NULL).
/// This function is only called when parent_id is provided.
/// Orphan items belong directly to the project via project_id.
pub async fn validate_hierarchy(
    pool: &SqlitePool,
    child_type: WorkItemType,
    parent_id: Uuid,
) -> WsErrorResult<()> {
    // ... existing implementation unchanged ...
}
```

---

### Files Summary (Orphan Feature)

| Step | Layer | File | Change |
|------|-------|------|--------|
| 13 | Rust | `hierarchy_validator.rs` | Update documentation comments |
| 23 | Blazor | `WorkItemDialog.razor` | Make parent optional, remove validation |

---

### Verification (Orphan Feature - Steps 13, 23)

1. **Create orphan Story:**
   - Open project → Create Work Item → Select "Story"
   - Leave Parent dropdown empty (shows "Select an Epic (optional)")
   - Enter title and submit → Should succeed
   - Verify Story appears in Kanban board with no parent

2. **Create orphan Task:**
   - Open project → Create Work Item → Select "Task"
   - Leave Parent dropdown empty
   - Submit → Should succeed

3. **Create Story WITH parent:**
   - Create Epic first
   - Create Story → Select the Epic as parent
   - Verify hierarchy is preserved when parent IS selected

4. **Backend validation still works:**
   - If parent is selected, wrong hierarchy should still error
   - e.g., Task with Epic parent should fail (if parent provided)

---

## Feature C: Edit Parent Assignment

Allow changing or assigning a parent to existing work items (not just at creation time).

### Current State

- `WorkItemDialog.razor` only shows parent selection when creating (`!_isEdit`)
- Once created, there's no UI to assign/change/remove a parent

### Implementation

**Note:** Proto change (`parent_id` in `UpdateWorkItemRequest`) is consolidated in **Step 2** above.

---

#### Step 16: Add ParentId to C# UpdateWorkItemRequest

**File:** `frontend/ProjectManagement.Core/Models/UpdateWorkItemRequest.cs`

```csharp
public sealed record UpdateWorkItemRequest
{
    public required Guid WorkItemId { get; init; }
    public required int ExpectedVersion { get; init; }
    public string? Title { get; init; }
    public string? Description { get; init; }
    public string? Status { get; init; }
    public string? Priority { get; init; }
    public Guid? AssigneeId { get; init; }
    public int? StoryPoints { get; init; }
    public Guid? SprintId { get; init; }
    public int? Position { get; init; }

    /// <summary>
    /// Parent work item ID. Set to Guid.Empty to clear the parent.
    /// Leave null to keep the current parent unchanged.
    /// </summary>
    public Guid? ParentId { get; init; }  // NEW

    /// <summary>
    /// Indicates whether ParentId should be updated (including clearing).
    /// Required because null ParentId could mean "no change" or "clear parent".
    /// </summary>
    public bool UpdateParent { get; init; }  // NEW
}
```

**Note:** We need `UpdateParent` flag because `null` is ambiguous (no change vs clear).

---

#### Step 10: Update Rust change_tracker.rs

**File:** `backend/crates/pm-ws/src/handlers/change_tracker.rs`

Add parent_id change tracking:

```rust
// Add after existing field tracking:

if let Some(ref new_parent_id) = request.parent_id {
    let current_parent = current.parent_id.map(|id| id.to_string()).unwrap_or_default();
    if current_parent != *new_parent_id {
        changes.push(FieldChange {
            field_name: "parent_id".to_string(),
            old_value: if current_parent.is_empty() { None } else { Some(current_parent) },
            new_value: if new_parent_id.is_empty() { None } else { Some(new_parent_id.clone()) },
        });
    }
}
```

---

#### Step 11: Update Rust work_item.rs handle_update

**File:** `backend/crates/pm-ws/src/handlers/work_item.rs`

Add hierarchy validation BEFORE apply_updates (async validation needed):

```rust
pub async fn handle_update(
    req: UpdateWorkItemRequest,
    ctx: HandlerContext,
) -> WsErrorResult<WebSocketMessage> {
    // ... existing steps 1-4 ...

    // 4b. Validate new parent if changing (NEW)
    if let Some(ref new_parent_id) = req.parent_id {
        if !new_parent_id.is_empty() {
            let parent_uuid = parse_uuid(new_parent_id, "parent_id")?;

            // Can't be your own parent
            if parent_uuid == work_item.id {
                return Err(WsError::ValidationError {
                    message: "Work item cannot be its own parent".to_string(),
                    field: Some("parent_id".to_string()),
                    location: ErrorLocation::from(Location::caller()),
                });
            }

            // Validate hierarchy rules
            db_read(&ctx, "validate_hierarchy", || async {
                validate_hierarchy(&ctx.pool, work_item.item_type.clone(), parent_uuid).await
            })
            .await?;

            // Prevent circular references (parent can't be a descendant)
            db_read(&ctx, "check_circular", || async {
                check_circular_reference(&ctx.pool, work_item.id, parent_uuid).await
            })
            .await?;
        }
    }

    // 5. Track changes
    let changes = track_changes(&work_item, &req);
    // ... rest unchanged ...
}

/// Check that new_parent is not a descendant of work_item (prevent cycles)
async fn check_circular_reference(
    pool: &SqlitePool,
    work_item_id: Uuid,
    new_parent_id: Uuid,
) -> WsErrorResult<()> {
    // Walk up the tree from new_parent to ensure we don't hit work_item_id
    let mut current = Some(new_parent_id);
    let mut depth = 0;
    const MAX_DEPTH: i32 = 100; // Prevent infinite loops

    while let Some(id) = current {
        if depth > MAX_DEPTH {
            return Err(WsError::ValidationError {
                message: "Hierarchy too deep".to_string(),
                field: Some("parent_id".to_string()),
                location: ErrorLocation::from(Location::caller()),
            });
        }

        if id == work_item_id {
            return Err(WsError::ValidationError {
                message: "Cannot create circular parent reference".to_string(),
                field: Some("parent_id".to_string()),
                location: ErrorLocation::from(Location::caller()),
            });
        }

        let item = WorkItemRepository::find_by_id(pool, id).await?;
        current = item.and_then(|i| i.parent_id);
        depth += 1;
    }

    Ok(())
}
```

---

#### Step 12: Update Rust work_item.rs apply_updates

**File:** `backend/crates/pm-ws/src/handlers/work_item.rs`

Add parent_id to apply_updates function:

```rust
fn apply_updates(work_item: &mut WorkItem, req: &UpdateWorkItemRequest) -> Result<(), WsError> {
    // ... existing field updates ...

    // Handle parent_id change (NEW)
    if let Some(ref parent_id) = req.parent_id {
        work_item.parent_id = if parent_id.is_empty() {
            None  // Clear parent (make orphan)
        } else {
            Some(parse_uuid(parent_id, "parent_id")?)
        };
    }

    Ok(())
}
```

---

#### Step 23: Update WorkItemDialog.razor for Edit Mode

**File:** `frontend/ProjectManagement.Components/WorkItems/WorkItemDialog.razor`

This step combines Feature B (optional parent on create) and Feature C (parent editing).

**Change 1:** Show parent selection in edit mode too:

```razor
@* Parent Selection (for Story/Task) - Show in both create AND edit mode *@
@if (_itemType == WorkItemType.Story || _itemType == WorkItemType.Task)
{
    <RadzenFormField Text="@GetParentFieldLabel()" Style="width: 100%;">
        <RadzenDropDown @bind-Value="_selectedParentId"
                        TValue="Guid?"
                        Data="@_availableParents"
                        TextProperty="Title"
                        ValueProperty="Id"
                        AllowClear="true"
                        Placeholder="@GetParentPlaceholder()"
                        Style="width: 100%;"
                        Change="@(_ => MarkDirty())" />
    </RadzenFormField>
}
```

**Change 2:** Load current parent and available parents in edit mode:

```csharp
protected override void OnInitialized()
{
    // ... existing code ...

    if (_isEdit && WorkItem is not null)
    {
        _itemType = WorkItem.ItemType;
        _title = _originalTitle = WorkItem.Title;
        // ... existing field assignments ...

        // NEW: Load parent for edit mode
        _selectedParentId = _originalParentId = WorkItem.ParentId;
        LoadAvailableParents();
    }
    else
    {
        _itemType = DefaultItemType;
        _selectedParentId = ParentId;
        LoadAvailableParents();
    }
    // ...
}

// Add original parent tracking
private Guid? _originalParentId;
```

**Change 3:** Filter out self and descendants from available parents:

```csharp
private void LoadAvailableParents()
{
    var validParentType = _itemType switch
    {
        WorkItemType.Story => WorkItemType.Epic,
        WorkItemType.Task => WorkItemType.Story,
        _ => (WorkItemType?)null
    };

    if (validParentType is null)
    {
        _availableParents.Clear();
        return;
    }

    var candidates = AppState.WorkItems.GetByProject(ProjectId)
        .Where(w => w.ItemType == validParentType && w.DeletedAt == null);

    // In edit mode, exclude self and descendants to prevent circular refs
    if (_isEdit && WorkItem is not null)
    {
        var descendants = GetDescendantIds(WorkItem.Id);
        candidates = candidates.Where(w => w.Id != WorkItem.Id && !descendants.Contains(w.Id));
    }

    _availableParents = candidates.ToList();
}

private HashSet<Guid> GetDescendantIds(Guid parentId)
{
    var descendants = new HashSet<Guid>();
    var queue = new Queue<Guid>();
    queue.Enqueue(parentId);

    while (queue.Count > 0)
    {
        var current = queue.Dequeue();
        var children = AppState.WorkItems.GetByProject(ProjectId)
            .Where(w => w.ParentId == current && w.DeletedAt == null);

        foreach (var child in children)
        {
            if (descendants.Add(child.Id))
            {
                queue.Enqueue(child.Id);
            }
        }
    }

    return descendants;
}
```

**Change 4:** Update dirty tracking:

```csharp
private void MarkDirty()
{
    _isDirty = _title != _originalTitle ||
               _description != _originalDescription ||
               _status != _originalStatus ||
               _priority != _originalPriority ||
               _storyPoints != _originalStoryPoints ||
               _sprintId != _originalSprintId ||
               _selectedParentId != _originalParentId;  // NEW
}
```

**Change 5:** Include parent in update request:

```csharp
private async Task UpdateWorkItemAsync()
{
    var parentChanged = _selectedParentId != _originalParentId;

    var request = new UpdateWorkItemRequest
    {
        WorkItemId = WorkItem!.Id,
        ExpectedVersion = WorkItem.Version,
        Title = _title,
        Description = _description,
        Status = _status,
        Priority = _priority,
        StoryPoints = _storyPoints,
        SprintId = _sprintId,
        ParentId = parentChanged ? (_selectedParentId ?? Guid.Empty) : null,  // NEW
        UpdateParent = parentChanged  // NEW
    };

    await AppState.WorkItems.UpdateAsync(request);
    // ...
}
```

---

### Files Summary (Edit Parent Feature)

| Step | Layer | File | Change |
|------|-------|------|--------|
| 2 | Proto | `proto/messages.proto` | Add `parent_id` field to UpdateWorkItemRequest |
| 10 | Rust | `change_tracker.rs` | Track parent_id changes |
| 11 | Rust | `work_item.rs` (handle_update) | Validate hierarchy + circular refs |
| 12 | Rust | `work_item.rs` (apply_updates) | Apply parent_id changes |
| 16 | C# | `UpdateWorkItemRequest.cs` | Add `ParentId` and `UpdateParent` properties |
| 23 | Blazor | `WorkItemDialog.razor` | Show parent selection in edit mode |

---

### Verification (Edit Parent Feature)

1. **Assign parent to orphan:**
   - Create orphan Story
   - Create Epic
   - Edit Story → Select Epic as parent → Save
   - Verify Story now appears under Epic

2. **Change parent:**
   - Create two Epics (A and B)
   - Create Story under Epic A
   - Edit Story → Change parent to Epic B → Save
   - Verify Story moved from A to B

3. **Remove parent (make orphan):**
   - Create Epic with Story under it
   - Edit Story → Clear parent dropdown → Save
   - Verify Story is now orphan

4. **Circular reference prevented:**
   - Create Epic → Story → Task hierarchy
   - Edit Epic → Try to set Story as parent
   - Should fail with validation error

5. **Self-parent prevented:**
   - Edit any work item
   - Should NOT see itself in parent dropdown
