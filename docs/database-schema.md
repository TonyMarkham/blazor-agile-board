# Database Schema Design

All tables are prefixed with `pm_` to avoid collisions with the host SaaS platform tables.

## Core Principles

1. **UUID primary keys**: All tables use TEXT UUIDs for primary keys (enables distributed/offline creation)
2. **Audit everything**: All tables have created_at, updated_at, created_by, updated_by
3. **Soft deletes**: deleted_at column for soft deletion (important for audit trail)
4. **Foreign keys enabled**: Use `PRAGMA foreign_keys = ON`
5. **Indexes**: Create indexes on foreign keys and frequently queried columns
6. **LLM-friendly**: Descriptive column names, normalized but not over-normalized, dedicated context table

---

## Work Item Types

We use a single polymorphic table for Projects, Epics, Stories, and Tasks to simplify hierarchy.

### pm_work_items

The core table for all hierarchical work (Projects, Epics, Stories, Tasks).

```sql
CREATE TABLE pm_work_items (
    id TEXT PRIMARY KEY,  -- UUID for distributed/offline creation
    item_type TEXT NOT NULL CHECK(item_type IN ('project', 'epic', 'story', 'task')),

    -- Hierarchy
    parent_id TEXT,  -- Self-referential: epic → project, story → epic, task → story
    project_id TEXT NOT NULL,  -- Denormalized for query performance
    position INTEGER NOT NULL DEFAULT 0,  -- For ordering within parent

    -- Core fields
    title TEXT NOT NULL,
    description TEXT,  -- Markdown format

    -- Workflow
    status TEXT NOT NULL DEFAULT 'backlog',  -- backlog, in_progress, in_review, done, custom_*

    -- Assignment
    assignee_id TEXT,  -- Foreign key to platform's users table

    -- Sprint
    sprint_id TEXT,  -- NULL if in backlog, set when assigned to sprint

    -- Metadata
    created_at INTEGER NOT NULL,  -- Unix timestamp
    updated_at INTEGER NOT NULL,
    created_by TEXT NOT NULL,  -- Foreign key to platform's users table
    updated_by TEXT NOT NULL,
    deleted_at INTEGER,  -- Soft delete

    FOREIGN KEY (parent_id) REFERENCES pm_work_items(id) ON DELETE CASCADE,
    FOREIGN KEY (project_id) REFERENCES pm_work_items(id) ON DELETE CASCADE,
    FOREIGN KEY (sprint_id) REFERENCES pm_sprints(id) ON DELETE SET NULL,
    FOREIGN KEY (assignee_id) REFERENCES users(id) ON DELETE SET NULL,
    FOREIGN KEY (created_by) REFERENCES users(id),
    FOREIGN KEY (updated_by) REFERENCES users(id)
);

CREATE INDEX idx_pm_work_items_parent ON pm_work_items(parent_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_work_items_project ON pm_work_items(project_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_work_items_sprint ON pm_work_items(sprint_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_work_items_assignee ON pm_work_items(assignee_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_work_items_type ON pm_work_items(item_type) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_work_items_status ON pm_work_items(status) WHERE deleted_at IS NULL;
```

**Design Notes:**
- Single table for all work item types simplifies hierarchy queries
- `item_type` discriminator column for polymorphic behavior
- `project_id` denormalized for fast project-scoped queries
- `position` for drag-and-drop ordering within parent
- `status` is TEXT to support custom swim lanes (v1.1 feature prep)

---

## Sprints

### pm_sprints

Time-boxed iterations for planning and execution.

```sql
CREATE TABLE pm_sprints (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,

    name TEXT NOT NULL,
    goal TEXT,  -- Sprint goal/objective

    start_date INTEGER NOT NULL,  -- Unix timestamp
    end_date INTEGER NOT NULL,

    status TEXT NOT NULL DEFAULT 'planned' CHECK(status IN ('planned', 'active', 'completed', 'cancelled')),

    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    created_by TEXT NOT NULL,
    updated_by TEXT NOT NULL,
    deleted_at INTEGER,

    FOREIGN KEY (project_id) REFERENCES pm_work_items(id) ON DELETE CASCADE,
    FOREIGN KEY (created_by) REFERENCES users(id),
    FOREIGN KEY (updated_by) REFERENCES users(id)
);

CREATE INDEX idx_pm_sprints_project ON pm_sprints(project_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_sprints_dates ON pm_sprints(start_date, end_date) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_sprints_status ON pm_sprints(status) WHERE deleted_at IS NULL;
```

---

## Comments

### pm_comments

Discussion threads on work items.

```sql
CREATE TABLE pm_comments (
    id TEXT PRIMARY KEY,
    work_item_id TEXT NOT NULL,

    content TEXT NOT NULL,  -- Markdown format

    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    created_by TEXT NOT NULL,
    updated_by TEXT NOT NULL,
    deleted_at INTEGER,

    FOREIGN KEY (work_item_id) REFERENCES pm_work_items(id) ON DELETE CASCADE,
    FOREIGN KEY (created_by) REFERENCES users(id),
    FOREIGN KEY (updated_by) REFERENCES users(id)
);

CREATE INDEX idx_pm_comments_work_item ON pm_comments(work_item_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_comments_created ON pm_comments(created_at DESC) WHERE deleted_at IS NULL;
```

---

## Time Tracking

### pm_time_entries

Log time spent on work items.

```sql
CREATE TABLE pm_time_entries (
    id TEXT PRIMARY KEY,
    work_item_id TEXT NOT NULL,
    user_id TEXT NOT NULL,

    started_at INTEGER NOT NULL,  -- Unix timestamp
    ended_at INTEGER,  -- NULL if currently running
    duration_seconds INTEGER,  -- Calculated: ended_at - started_at

    description TEXT,  -- What was worked on

    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    deleted_at INTEGER,

    FOREIGN KEY (work_item_id) REFERENCES pm_work_items(id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX idx_pm_time_entries_work_item ON pm_time_entries(work_item_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_time_entries_user ON pm_time_entries(user_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_time_entries_started ON pm_time_entries(started_at DESC) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_time_entries_running ON pm_time_entries(ended_at) WHERE ended_at IS NULL AND deleted_at IS NULL;
```

**Design Notes:**
- `ended_at` NULL means timer is currently running
- `duration_seconds` cached for query performance
- Can query active timers with `WHERE ended_at IS NULL`

---

## Dependencies

### pm_dependencies

2-way blocking relationships between work items.

```sql
CREATE TABLE pm_dependencies (
    id TEXT PRIMARY KEY,

    blocking_item_id TEXT NOT NULL,  -- The item that must be completed first
    blocked_item_id TEXT NOT NULL,   -- The item that is waiting

    dependency_type TEXT NOT NULL DEFAULT 'blocks' CHECK(dependency_type IN ('blocks', 'relates_to')),

    created_at INTEGER NOT NULL,
    created_by TEXT NOT NULL,
    deleted_at INTEGER,

    FOREIGN KEY (blocking_item_id) REFERENCES pm_work_items(id) ON DELETE CASCADE,
    FOREIGN KEY (blocked_item_id) REFERENCES pm_work_items(id) ON DELETE CASCADE,
    FOREIGN KEY (created_by) REFERENCES users(id),

    -- Prevent duplicate dependencies and self-referential dependencies
    UNIQUE(blocking_item_id, blocked_item_id),
    CHECK(blocking_item_id != blocked_item_id)
);

CREATE INDEX idx_pm_dependencies_blocking ON pm_dependencies(blocking_item_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_dependencies_blocked ON pm_dependencies(blocked_item_id) WHERE deleted_at IS NULL;
```

**Design Notes:**
- Directional: A blocks B (A must complete before B can start)
- Query "what is blocking this?" → `WHERE blocked_item_id = ?`
- Query "what does this block?" → `WHERE blocking_item_id = ?`
- `dependency_type` for future expansion (relates_to, duplicates, etc.)

---

## Activity Log

### pm_activity_log

Complete audit trail of all changes. Critical for LLM context.

```sql
CREATE TABLE pm_activity_log (
    id TEXT PRIMARY KEY,  -- UUID for consistency

    entity_type TEXT NOT NULL,  -- 'work_item', 'sprint', 'comment', 'time_entry', 'dependency'
    entity_id TEXT NOT NULL,

    action TEXT NOT NULL,  -- 'created', 'updated', 'deleted', 'status_changed', 'assigned', etc.

    -- Change details
    field_name TEXT,  -- Which field changed (NULL for create/delete)
    old_value TEXT,   -- Previous value (JSON if complex)
    new_value TEXT,   -- New value (JSON if complex)

    -- Metadata
    user_id TEXT NOT NULL,
    timestamp INTEGER NOT NULL,  -- Unix timestamp for ordering

    -- Optional context
    comment TEXT,  -- Human-readable description of change

    FOREIGN KEY (user_id) REFERENCES users(id)
);

CREATE INDEX idx_pm_activity_log_entity ON pm_activity_log(entity_type, entity_id);
CREATE INDEX idx_pm_activity_log_timestamp ON pm_activity_log(timestamp DESC);
CREATE INDEX idx_pm_activity_log_user ON pm_activity_log(user_id);
CREATE INDEX idx_pm_activity_log_action ON pm_activity_log(action);
```

**Design Notes:**
- Append-only table (no updates or deletes)
- Captures every change for audit and LLM context
- `field_name` granular tracking (e.g., "title", "status", "assignee_id")
- `comment` for human-readable summaries (e.g., "Moved to In Progress")
- Can reconstruct full history of any entity

---

## Swim Lanes

### pm_swim_lanes

Custom workflow states (v1 includes defaults, v1.1 adds custom).

```sql
CREATE TABLE pm_swim_lanes (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,

    name TEXT NOT NULL,
    status_value TEXT NOT NULL,  -- Used in work_items.status field
    position INTEGER NOT NULL DEFAULT 0,  -- Display order

    is_default BOOLEAN NOT NULL DEFAULT 0,  -- Default lanes: backlog, in_progress, in_review, done

    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    deleted_at INTEGER,

    FOREIGN KEY (project_id) REFERENCES pm_work_items(id) ON DELETE CASCADE,

    UNIQUE(project_id, status_value)
);

CREATE INDEX idx_pm_swim_lanes_project ON pm_swim_lanes(project_id) WHERE deleted_at IS NULL;
```

**Design Notes:**
- v1: Seed default lanes (backlog, in_progress, in_review, done)
- v1.1: Allow custom lanes per project
- `status_value` links to `work_items.status` column
- `is_default` prevents deletion of core lanes

---

## LLM Context & Instructions

### pm_llm_context

Instructions and metadata to help LLMs understand and interact with the project management database.

```sql
CREATE TABLE pm_llm_context (
    id TEXT PRIMARY KEY,
    context_type TEXT NOT NULL CHECK(context_type IN ('schema_doc', 'query_pattern', 'business_rule', 'example', 'instruction')),

    category TEXT NOT NULL,  -- 'work_items', 'sprints', 'time_tracking', 'dependencies', 'general'
    title TEXT NOT NULL,
    content TEXT NOT NULL,  -- Detailed documentation/instructions

    -- Optional SQL examples
    example_sql TEXT,
    example_description TEXT,

    priority INTEGER NOT NULL DEFAULT 0,  -- Higher priority shown first to LLMs

    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,

    -- Soft delete to preserve historical context
    deleted_at INTEGER
);

CREATE INDEX idx_pm_llm_context_type ON pm_llm_context(context_type) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_llm_context_category ON pm_llm_context(category) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_llm_context_priority ON pm_llm_context(priority DESC) WHERE deleted_at IS NULL;
```

**Design Notes:**
- Self-documenting database for LLM agents
- Contains schema explanations, common query patterns, business rules
- LLMs query this table first to understand how to interact with the system
- Examples:
  - **schema_doc**: "The pm_work_items table uses polymorphic types (project/epic/story/task)"
  - **query_pattern**: "To find all blocked tasks: SELECT wi.* FROM pm_work_items wi JOIN pm_dependencies d ON d.blocked_item_id = wi.id WHERE d.deleted_at IS NULL"
  - **business_rule**: "Tasks can only be assigned to one sprint at a time"
  - **instruction**: "Always use ORDER BY position when displaying work items within a parent"

**Example Seed Data:**
```sql
INSERT INTO pm_llm_context (id, context_type, category, title, content, example_sql, priority) VALUES
(
    'ctx_001',
    'schema_doc',
    'work_items',
    'Work Item Hierarchy',
    'The pm_work_items table uses a single-table polymorphic design with item_type discriminator (project/epic/story/task). parent_id creates the hierarchy tree. project_id is denormalized for fast project-scoped queries.',
    'SELECT * FROM pm_work_items WHERE parent_id = ? ORDER BY position',
    100
),
(
    'ctx_002',
    'query_pattern',
    'dependencies',
    'Find Blocked Tasks',
    'To find all tasks that are blocked and cannot proceed, join with pm_dependencies where the task is the blocked_item_id and the blocking item is not complete.',
    'SELECT wi.*, blocking.title AS blocked_by_title FROM pm_work_items wi JOIN pm_dependencies d ON d.blocked_item_id = wi.id JOIN pm_work_items blocking ON blocking.id = d.blocking_item_id WHERE blocking.status != ''done'' AND d.deleted_at IS NULL',
    90
),
(
    'ctx_003',
    'business_rule',
    'sprints',
    'Sprint Assignment Rules',
    'A work item can only be assigned to one sprint at a time. When moving between sprints, update sprint_id. Work items with sprint_id = NULL are in the backlog. Only stories and tasks should be assigned to sprints, not projects or epics.',
    NULL,
    80
);
```

---

## Summary Statistics

### Computed Views (Optional)

For performance, we can create views for common aggregations:

```sql
-- Sprint progress
CREATE VIEW pm_sprint_stats AS
SELECT
    s.id AS sprint_id,
    s.project_id,
    COUNT(wi.id) AS total_items,
    SUM(CASE WHEN wi.status = 'done' THEN 1 ELSE 0 END) AS completed_items,
    SUM(COALESCE(te.duration_seconds, 0)) AS total_time_seconds
FROM pm_sprints s
LEFT JOIN pm_work_items wi ON wi.sprint_id = s.id AND wi.deleted_at IS NULL
LEFT JOIN pm_time_entries te ON te.work_item_id = wi.id AND te.deleted_at IS NULL
WHERE s.deleted_at IS NULL
GROUP BY s.id;

-- Project progress
CREATE VIEW pm_project_stats AS
SELECT
    p.id AS project_id,
    COUNT(wi.id) AS total_items,
    SUM(CASE WHEN wi.status = 'done' THEN 1 ELSE 0 END) AS completed_items,
    COUNT(DISTINCT wi.assignee_id) AS unique_assignees,
    MAX(wi.updated_at) AS last_activity
FROM pm_work_items p
LEFT JOIN pm_work_items wi ON wi.project_id = p.id AND wi.id != p.id AND wi.deleted_at IS NULL
WHERE p.item_type = 'project' AND p.deleted_at IS NULL
GROUP BY p.id;
```

---

## Migration Strategy

1. **Initial Migration**: Create all tables with indexes
2. **Seed Data**: Insert default swim lanes for each new project
3. **Foreign Key Constraints**: Enable `PRAGMA foreign_keys = ON` on every connection
4. **Full-Text Search** (v1.1): Add FTS5 virtual table for work items

```sql
-- Enable foreign keys (must be done per connection)
PRAGMA foreign_keys = ON;

-- Insert default swim lanes when project is created
INSERT INTO pm_swim_lanes (id, project_id, name, status_value, position, is_default) VALUES
    (uuid(), ?, 'Backlog', 'backlog', 0, 1),
    (uuid(), ?, 'In Progress', 'in_progress', 1, 1),
    (uuid(), ?, 'In Review', 'in_review', 2, 1),
    (uuid(), ?, 'Done', 'done', 3, 1);
```

---

## LLM Context Queries

Example queries optimized for LLM analysis:

```sql
-- Get complete project context
SELECT
    wi.*,
    u1.name AS assignee_name,
    u2.name AS created_by_name,
    s.name AS sprint_name
FROM pm_work_items wi
LEFT JOIN users u1 ON u1.id = wi.assignee_id
LEFT JOIN users u2 ON u2.id = wi.created_by
LEFT JOIN pm_sprints s ON s.id = wi.sprint_id
WHERE wi.project_id = ? AND wi.deleted_at IS NULL
ORDER BY wi.position;

-- Get full activity history
SELECT
    al.*,
    u.name AS user_name
FROM pm_activity_log al
JOIN users u ON u.id = al.user_id
WHERE al.entity_type = 'work_item' AND al.entity_id = ?
ORDER BY al.timestamp DESC;
```
