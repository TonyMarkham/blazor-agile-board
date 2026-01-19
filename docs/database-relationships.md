# Database Relationship Map

## Entity Relationship Diagram (ASCII)

```
                                    ┌─────────────────┐
                                    │     users       │
                                    │─────────────────│
                                    │ id (PK)         │
                                    │ email           │
                                    │ name            │
                                    └────────┬────────┘
                                             │
           ┌─────────────────────────────────┼─────────────────────────────────┐
           │                                 │                                 │
           │ created_by/updated_by           │ user_id                         │ assignee_id
           │                                 │                                 │ (SET NULL)
           │                                 │                                 │
           ▼                                 ▼                                 ▼
┌─────────────────────────────────────────────────────────────────────────────────────────────┐
│                                      pm_work_items                                           │
│─────────────────────────────────────────────────────────────────────────────────────────────│
│ id (PK)              TEXT                                                                    │
│ item_type            TEXT     CHECK('project','epic','story','task')                         │
│ parent_id (FK)       TEXT     ──────────────────────┐ SELF-REFERENCE (CASCADE)               │
│ project_id (FK)      TEXT     ──────────────────────┤ (to pm_work_items where type=project)  │
│ position             INTEGER                        │ (CASCADE)                              │
│ title                TEXT                           │                                        │
│ description          TEXT                           │                                        │
│ status               TEXT     ◄─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ┼ ─ ─ ─ ─ ─ ─ ─ ┐ JOINED BY VALUE        │
│ priority             TEXT                           │               │ (not FK)               │
│ assignee_id (FK)     TEXT     → users.id (SET NULL) │               │                        │
│ sprint_id (FK)       TEXT     ───────────────┐      │               │                        │
│ story_points         INTEGER              (SET NULL)│               │                        │
│ version              INTEGER                 │      │               │                        │
│ created_at/updated_at INTEGER                │      │               │                        │
│ created_by/updated_by TEXT    → users.id     │      │               │                        │
│ deleted_at           INTEGER                 │      │               │                        │
└──────────────────────────────────────────────┼──────┼───────────────┼────────────────────────┘
           │                │              │   │      │               │
           │                │              │   │      │               │
           │ work_item_id   │ work_item_id │   │      │               │ status_value
           │ (CASCADE)      │ (CASCADE)    │   │      │               │
           │                │              │   │      │               │
           ▼                ▼              │   │      │               ▼
┌──────────────────┐ ┌──────────────────┐  │   │      │    ┌──────────────────────┐
│   pm_comments    │ │ pm_time_entries  │  │   │      │    │    pm_swim_lanes     │
│──────────────────│ │──────────────────│  │   │      │    │──────────────────────│
│ id (PK)          │ │ id (PK)          │  │   │      │    │ id (PK)              │
│ work_item_id(FK) │ │ work_item_id(FK) │  │   │      │    │ project_id (FK) ─────┼───┘
│ content          │ │ user_id (FK)     │  │   │      │    │ name            (CASCADE)
│ created_at       │ │   (CASCADE)      │  │   │      │    │ status_value ─ ─ ─ ─ ┘
│ updated_at       │ │ started_at       │  │   │      │    │ position             │
│ created_by (FK)  │ │ ended_at         │  │   │      │    │ is_default           │
│ updated_by (FK)  │ │ duration_seconds │  │   │      │    │ created_at           │
│ deleted_at       │ │ description      │  │   │      │    │ updated_at           │
└──────────────────┘ │ created_at       │  │   │      │    │ deleted_at           │
                     │ updated_at       │  │   │      │    └──────────────────────┘
                     │ deleted_at       │  │   │      │
                     └──────────────────┘  │   │      │
                                           │   │      │
           ┌───────────────────────────────┘   │      └────────────────────┐
           │                                   │                           │
           │ blocking_item_id / blocked_item_id│ project_id                │ project_id
           │           (CASCADE)                │ (CASCADE)                 │ (CASCADE)
           │                                   │                           │
           ▼                                   ▼                           ▼
┌────────────────────────┐            ┌──────────────────┐     ┌─────────────────────┐
│    pm_dependencies     │            │    pm_sprints    │     │  pm_project_members │
│────────────────────────│            │──────────────────│     │─────────────────────│
│ id (PK)                │            │ id (PK)          │     │ id (PK)             │
│ blocking_item_id (FK)  │──┐         │ project_id (FK)  │     │ project_id (FK)     │
│ blocked_item_id (FK)   │──┼─► both  │ name             │     │ user_id             │
│ dependency_type        │  │  point  │ goal             │     │ role                │
│ created_at             │  │  to     │ start_date       │     │ created_at          │
│ created_by (FK)        │  │  work   │ end_date         │     └─────────────────────┘
│ deleted_at             │  │  items  │ status           │
└────────────────────────┘  │         │ created_at       │
                            │         │ updated_at       │
                            │         │ created_by (FK)  │
                            └────────►│ updated_by (FK)  │
                                      │ deleted_at       │
                                      └──────────────────┘
```

## Foreign Key Relationships (Actual DB Constraints)

| From Table | Column | To Table | Column | ON DELETE |
|------------|--------|----------|--------|-----------|
| `pm_work_items` | `parent_id` | `pm_work_items` | `id` | CASCADE |
| `pm_work_items` | `project_id` | `pm_work_items` | `id` | CASCADE |
| `pm_work_items` | `sprint_id` | `pm_sprints` | `id` | SET NULL |
| `pm_work_items` | `assignee_id` | `users` | `id` | SET NULL |
| `pm_sprints` | `project_id` | `pm_work_items` | `id` | CASCADE |
| `pm_sprints` | `created_by` | `users` | `id` | - |
| `pm_sprints` | `updated_by` | `users` | `id` | - |
| `pm_comments` | `work_item_id` | `pm_work_items` | `id` | CASCADE |
| `pm_comments` | `created_by` | `users` | `id` | - |
| `pm_comments` | `updated_by` | `users` | `id` | - |
| `pm_time_entries` | `work_item_id` | `pm_work_items` | `id` | CASCADE |
| `pm_time_entries` | `user_id` | `users` | `id` | CASCADE |
| `pm_dependencies` | `blocking_item_id` | `pm_work_items` | `id` | CASCADE |
| `pm_dependencies` | `blocked_item_id` | `pm_work_items` | `id` | CASCADE |
| `pm_dependencies` | `created_by` | `users` | `id` | - |
| `pm_swim_lanes` | `project_id` | `pm_work_items` | `id` | CASCADE |
| `pm_project_members` | `project_id` | `pm_work_items` | `id` | CASCADE |

## Non-FK Relationships (Joined by Value)

| From Table | Column | To Table | Column | Relationship |
|------------|--------|----------|--------|--------------|
| `pm_work_items` | `status` | `pm_swim_lanes` | `status_value` | **Loose coupling** - swim lane defines display config for status values |

## Key Observations

### 1. WorkItem is Self-Referential (Polymorphic)
```
pm_work_items (item_type='project')
    └── pm_work_items (item_type='epic', parent_id=project.id, project_id=project.id)
        └── pm_work_items (item_type='story', parent_id=epic.id, project_id=project.id)
            └── pm_work_items (item_type='task', parent_id=story.id, project_id=project.id)
```

- `project_id` is **denormalized** - always points to the root project, even for deeply nested items
- `parent_id` is the immediate parent in the hierarchy

### 2. SwimLane ↔ WorkItem is NOT a Foreign Key
```sql
-- There is NO: work_item.swim_lane_id → swim_lanes.id
-- Instead, it's a value-based join:
SELECT w.*, sl.name as lane_name
FROM pm_work_items w
LEFT JOIN pm_swim_lanes sl
  ON w.status = sl.status_value
 AND w.project_id = sl.project_id
```

**Why?** SwimLanes are **display configuration**, not data ownership:
- Work items have a `status` (business data)
- Swim lanes define how to display items with certain statuses
- A status can exist without a swim lane (item just won't show on board)
- Swim lanes can be renamed/reordered without touching work items

### 3. Sprint ↔ WorkItem Circular Dependency
```sql
-- Circular FK relationship:
pm_work_items.sprint_id → pm_sprints.id (ON DELETE SET NULL)
pm_sprints.project_id → pm_work_items.id (ON DELETE CASCADE)
```

**FK Behavior:**
- When a **sprint is deleted**: Work items remain, but their `sprint_id` is cleared (SET NULL)
- When a **project is deleted**: All sprints cascade delete, which triggers SET NULL on work items (then work items cascade delete from project_id)

This circular dependency is intentional and safe because:
1. The work item → sprint reference is nullable and uses SET NULL
2. The sprint → project reference is non-nullable but uses CASCADE
3. Project deletion triggers a chain: project deleted → sprints cascade delete → work items' sprint_id set to NULL → work items cascade delete

### 4. Assignee References Use SET NULL
```sql
pm_work_items.assignee_id → users.id (ON DELETE SET NULL)
```

When a user is deleted, all work items assigned to them have `assignee_id` cleared. This prevents orphaned references while preserving the work item's history.

### 5. Dependencies are Bidirectional in Storage
```sql
-- Single row captures: A blocks B
pm_dependencies (blocking_item_id=A, blocked_item_id=B)

-- To find what blocks item B:
SELECT * FROM pm_dependencies WHERE blocked_item_id = B

-- To find what item A blocks:
SELECT * FROM pm_dependencies WHERE blocking_item_id = A
```

Both `blocking_item_id` and `blocked_item_id` cascade delete - if either work item is deleted, the dependency relationship is removed.

### 6. Audit Trail FK Constraints
Most tables have `created_by` and `updated_by` columns that reference `users.id`, but **do not have ON DELETE actions**. This means:
- If a user is deleted, their audit trail entries remain (user_id becomes a dangling reference)
- This preserves audit history even after user deletion
- Application code should handle displaying "[Deleted User]" for dangling references

Affected columns:
- `pm_sprints.{created_by, updated_by}`
- `pm_comments.{created_by, updated_by}`
- `pm_dependencies.created_by`

**Exception:** `pm_time_entries.user_id` uses CASCADE (time entries are deleted with the user).

### 7. Cascade Delete Chains

**Deleting a Project (work_item with type='project'):**
1. All child work items cascade delete (via `project_id` FK)
2. All sprints cascade delete (via `project_id` FK)
3. All swim lanes cascade delete (via `project_id` FK)
4. All project members cascade delete (via `project_id` FK)
5. For each deleted work item:
   - Comments cascade delete
   - Time entries cascade delete
   - Dependencies cascade delete (both directions)

**Deleting a User:**
1. Time entries cascade delete (via `user_id` FK)
2. Work item assignees are cleared (via `assignee_id` FK with SET NULL)
3. Audit trail references (`created_by`, `updated_by`) become dangling (no ON DELETE action)

## Summary Table: What Each Entity "Knows"

| Entity | Knows About (via FK) | Knows About (via value) |
|--------|---------------------|------------------------|
| **WorkItem** | parent (self), project (self), sprint, assignee | status → swim_lane |
| **Sprint** | project, creator, updater | - |
| **Comment** | work_item, creator, updater | - |
| **TimeEntry** | work_item, user | - |
| **Dependency** | blocking_item, blocked_item, creator | - |
| **SwimLane** | project | work_items (via status_value match) |
| **ProjectMember** | project | - |
