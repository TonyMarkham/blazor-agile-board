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
           │ created_by/updated_by           │ user_id                         │ user_id
           │                                 │                                 │
           ▼                                 ▼                                 ▼
┌─────────────────────────────────────────────────────────────────────────────────────────────┐
│                                      pm_work_items                                           │
│─────────────────────────────────────────────────────────────────────────────────────────────│
│ id (PK)              TEXT                                                                    │
│ item_type            TEXT     CHECK('project','epic','story','task')                         │
│ parent_id (FK)       TEXT     ──────────────────────┐ SELF-REFERENCE                         │
│ project_id (FK)      TEXT     ──────────────────────┤ (to pm_work_items where type=project)  │
│ position             INTEGER                        │                                        │
│ title                TEXT                           │                                        │
│ description          TEXT                           │                                        │
│ status               TEXT     ◄─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ┼ ─ ─ ─ ─ ─ ─ ─ ┐ JOINED BY VALUE        │
│ priority             TEXT                           │               │ (not FK)               │
│ assignee_id          TEXT     → users.id            │               │                        │
│ sprint_id (FK)       TEXT     ─────────────────┐    │               │                        │
│ story_points         INTEGER                   │    │               │                        │
│ version              INTEGER                   │    │               │                        │
│ created_at/updated_at INTEGER                  │    │               │                        │
│ created_by/updated_by TEXT    → users.id       │    │               │                        │
│ deleted_at           INTEGER                   │    │               │                        │
└────────────────────────────────────────────────┼────┼───────────────┼────────────────────────┘
           │                │              │     │    │               │
           │                │              │     │    │               │
           │ work_item_id   │ work_item_id │     │    │               │ status_value
           │                │              │     │    │               │
           ▼                ▼              │     │    │               ▼
┌──────────────────┐ ┌──────────────────┐  │     │    │    ┌──────────────────────┐
│   pm_comments    │ │ pm_time_entries  │  │     │    │    │    pm_swim_lanes     │
│──────────────────│ │──────────────────│  │     │    │    │──────────────────────│
│ id (PK)          │ │ id (PK)          │  │     │    │    │ id (PK)              │
│ work_item_id(FK) │ │ work_item_id(FK) │  │     │    │    │ project_id (FK) ─────┼───┘
│ content          │ │ user_id (FK)     │  │     │    │    │ name                 │
│ created_at       │ │ started_at       │  │     │    │    │ status_value ─ ─ ─ ─ ┘ (matches work_item.status)
│ updated_at       │ │ ended_at         │  │     │    │    │ position             │
│ created_by (FK)  │ │ duration_seconds │  │     │    │    │ is_default           │
│ updated_by (FK)  │ │ description      │  │     │    │    │ created_at           │
│ deleted_at       │ │ created_at       │  │     │    │    │ updated_at           │
└──────────────────┘ │ updated_at       │  │     │    │    │ deleted_at           │
                     │ deleted_at       │  │     │    │    └──────────────────────┘
                     └──────────────────┘  │     │    │
                                           │     │    │
           ┌───────────────────────────────┘     │    └────────────────────┐
           │                                     │                         │
           │ blocking_item_id / blocked_item_id  │ project_id              │ project_id
           │                                     │                         │
           ▼                                     ▼                         ▼
┌────────────────────────┐            ┌──────────────────┐     ┌─────────────────────┐
│    pm_dependencies     │            │    pm_sprints    │     │  pm_project_members │
│────────────────────────│            │──────────────────│     │─────────────────────│
│ id (PK)                │            │ id (PK)          │     │ id (PK)             │
│ blocking_item_id (FK)  │──┐         │ project_id (FK) ─┼─────┤ project_id (FK)     │
│ blocked_item_id (FK)   │──┼─► both  │ name             │     │ user_id (FK)        │
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
| `pm_project_members` | `project_id` | `pm_work_items` | `id` | - |

## Non-FK Relationships (Joined by Value)

| From Table | Column | To Table | Column | Relationship |
|------------|--------|----------|--------|--------------|
| `pm_work_items` | `status` | `pm_swim_lanes` | `status_value` | **Loose coupling** - swim lane defines display config for status values |
| `pm_work_items` | `sprint_id` | `pm_sprints` | `id` | **Missing FK!** - should reference sprints |
| `pm_work_items` | `assignee_id` | `users` | `id` | **Missing FK!** - should reference users |

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

### 3. Sprint ↔ WorkItem IS a Direct Reference
```sql
-- WorkItem stores sprint_id directly
work_item.sprint_id → sprint.id
```

But note: **There's no FK constraint in the migration!** The `sprint_id` column exists but isn't constrained. This might be intentional (allow orphaned references) or an oversight.

### 4. Dependencies are Bidirectional in Storage
```sql
-- Single row captures: A blocks B
pm_dependencies (blocking_item_id=A, blocked_item_id=B)

-- To find what blocks item B:
SELECT * FROM pm_dependencies WHERE blocked_item_id = B

-- To find what item A blocks:
SELECT * FROM pm_dependencies WHERE blocking_item_id = A
```

### 5. Missing FK Constraints (Potential Issues)

| Column | Points To | Status |
|--------|-----------|--------|
| `pm_work_items.sprint_id` | `pm_sprints.id` | **No FK constraint** |
| `pm_work_items.assignee_id` | `users.id` | **No FK constraint** |

These should probably have FK constraints added for referential integrity.

## Summary Table: What Each Entity "Knows"

| Entity | Knows About (via FK) | Knows About (via value) |
|--------|---------------------|------------------------|
| **WorkItem** | parent (self), project (self) | sprint (no FK), assignee (no FK), status → swim_lane |
| **Sprint** | project | - |
| **Comment** | work_item, creator, updater | - |
| **TimeEntry** | work_item, user | - |
| **Dependency** | blocking_item, blocked_item, creator | - |
| **SwimLane** | project | work_items (via status_value match) |
| **ProjectMember** | project, user | - |
