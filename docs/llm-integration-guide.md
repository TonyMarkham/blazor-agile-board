# LLM Integration Guide

This document explains how Large Language Models (LLMs) should interact with the Project Management database to provide intelligent assistance.

## Overview

The database is designed to be **LLM-first**, meaning:
1. All data is in a single SQLite file per tenant (easy to read entire context)
2. Complete audit trail in `pm_activity_log` (understand how project evolved)
3. Self-documenting via `pm_llm_context` table (LLM learns the schema)
4. Descriptive naming (no cryptic abbreviations)
5. Unix timestamps and TEXT UUIDs (simple, universal formats)

---

## Phase 1: Learning the Schema

### Step 1: Read the LLM Context Table

Before doing anything else, query the `pm_llm_context` table to understand the database structure:

```sql
SELECT context_type, category, title, content, example_sql
FROM pm_llm_context
WHERE deleted_at IS NULL
ORDER BY priority DESC, category, context_type;
```

This returns:
- **schema_doc**: How tables are structured
- **query_pattern**: Common queries with examples
- **business_rule**: Constraints and workflow rules
- **instruction**: Best practices for querying

### Step 2: Understand the Core Tables

The schema uses these main tables:

| Table | Purpose | Key Concept |
|-------|---------|-------------|
| `pm_work_items` | Projects, Epics, Stories, Tasks | Polymorphic single-table with `item_type` discriminator |
| `pm_sprints` | Time-boxed iterations | Links to work items via `sprint_id` |
| `pm_comments` | Discussion threads | Links to work items |
| `pm_time_entries` | Time tracking logs | Can have `ended_at = NULL` for running timers |
| `pm_dependencies` | Blocking relationships | Directional: `blocking_item_id` blocks `blocked_item_id` |
| `pm_activity_log` | Complete change history | Append-only audit trail |
| `pm_swim_lanes` | Workflow states | Defines valid status values |

---

## Phase 2: Understanding Project Context

### Get Complete Project Overview

```sql
-- Get all work items for a project with enriched context
SELECT
    wi.id,
    wi.item_type,
    wi.title,
    wi.description,
    wi.status,
    wi.parent_id,
    parent.title AS parent_title,
    assignee.name AS assignee_name,
    sprint.name AS sprint_name,
    creator.name AS created_by_name,
    wi.created_at,
    wi.updated_at
FROM pm_work_items wi
LEFT JOIN pm_work_items parent ON parent.id = wi.parent_id
LEFT JOIN users assignee ON assignee.id = wi.assignee_id
LEFT JOIN pm_sprints sprint ON sprint.id = wi.sprint_id
LEFT JOIN users creator ON creator.id = wi.created_by
WHERE wi.project_id = ? AND wi.deleted_at IS NULL
ORDER BY wi.item_type, wi.position;
```

### Get Project Activity Timeline

```sql
-- Reconstruct what happened on this project
SELECT
    al.timestamp,
    al.action,
    al.entity_type,
    al.field_name,
    al.old_value,
    al.new_value,
    al.comment,
    u.name AS user_name,
    wi.title AS related_item_title
FROM pm_activity_log al
JOIN users u ON u.id = al.user_id
LEFT JOIN pm_work_items wi ON wi.id = al.entity_id AND al.entity_type = 'work_item'
WHERE al.entity_id IN (
    SELECT id FROM pm_work_items WHERE project_id = ? AND deleted_at IS NULL
)
ORDER BY al.timestamp DESC
LIMIT 100;
```

---

## Phase 3: Answering Common Questions

### "What tasks are blocked?"

```sql
SELECT
    blocked.id,
    blocked.title,
    blocked.status,
    blocking.title AS blocked_by_title,
    blocking.status AS blocked_by_status,
    blocker_assignee.name AS blocked_by_assignee
FROM pm_work_items blocked
JOIN pm_dependencies d ON d.blocked_item_id = blocked.id
JOIN pm_work_items blocking ON blocking.id = d.blocking_item_id
LEFT JOIN users blocker_assignee ON blocker_assignee.id = blocking.assignee_id
WHERE
    d.dependency_type = 'blocks'
    AND blocking.status != 'done'
    AND d.deleted_at IS NULL
    AND blocked.deleted_at IS NULL
    AND blocking.deleted_at IS NULL
ORDER BY blocked.created_at;
```

### "How is the current sprint progressing?"

```sql
SELECT
    s.name AS sprint_name,
    s.start_date,
    s.end_date,
    s.status AS sprint_status,
    COUNT(wi.id) AS total_tasks,
    SUM(CASE WHEN wi.status = 'done' THEN 1 ELSE 0 END) AS completed_tasks,
    SUM(CASE WHEN wi.status = 'in_progress' THEN 1 ELSE 0 END) AS in_progress_tasks,
    SUM(CASE WHEN wi.status = 'in_review' THEN 1 ELSE 0 END) AS in_review_tasks,
    SUM(COALESCE(te.duration_seconds, 0)) AS total_time_seconds
FROM pm_sprints s
LEFT JOIN pm_work_items wi ON wi.sprint_id = s.id AND wi.deleted_at IS NULL
LEFT JOIN pm_time_entries te ON te.work_item_id = wi.id AND te.deleted_at IS NULL
WHERE s.id = ? AND s.deleted_at IS NULL
GROUP BY s.id;
```

### "Who is working on what?"

```sql
SELECT
    u.name AS assignee_name,
    COUNT(wi.id) AS total_assigned,
    SUM(CASE WHEN wi.status = 'in_progress' THEN 1 ELSE 0 END) AS active_tasks,
    SUM(CASE WHEN wi.status = 'done' THEN 1 ELSE 0 END) AS completed_tasks,
    GROUP_CONCAT(
        CASE WHEN wi.status = 'in_progress'
        THEN wi.title ELSE NULL END,
        ', '
    ) AS active_task_titles
FROM pm_work_items wi
JOIN users u ON u.id = wi.assignee_id
WHERE
    wi.project_id = ?
    AND wi.deleted_at IS NULL
GROUP BY u.id, u.name
ORDER BY active_tasks DESC, u.name;
```

### "What changed recently?"

```sql
SELECT
    al.timestamp,
    al.action,
    u.name AS who,
    wi.title AS what,
    al.field_name,
    al.old_value AS from_value,
    al.new_value AS to_value,
    al.comment
FROM pm_activity_log al
JOIN users u ON u.id = al.user_id
LEFT JOIN pm_work_items wi ON wi.id = al.entity_id
WHERE
    wi.project_id = ?
    AND al.timestamp > ?  -- e.g., last 7 days
ORDER BY al.timestamp DESC
LIMIT 50;
```

### "Show me the project hierarchy"

```sql
-- Recursive CTE to build full tree
WITH RECURSIVE work_item_tree AS (
    -- Root: the project itself
    SELECT
        id,
        title,
        item_type,
        parent_id,
        0 AS depth,
        title AS path
    FROM pm_work_items
    WHERE item_type = 'project' AND id = ? AND deleted_at IS NULL

    UNION ALL

    -- Children
    SELECT
        wi.id,
        wi.title,
        wi.item_type,
        wi.parent_id,
        tree.depth + 1,
        tree.path || ' > ' || wi.title
    FROM pm_work_items wi
    JOIN work_item_tree tree ON wi.parent_id = tree.id
    WHERE wi.deleted_at IS NULL
)
SELECT * FROM work_item_tree
ORDER BY path;
```

---

## Phase 4: Time Tracking Analysis

### Total Time Spent

```sql
SELECT
    wi.title,
    wi.item_type,
    COUNT(te.id) AS time_entry_count,
    SUM(te.duration_seconds) AS total_seconds,
    SUM(te.duration_seconds) / 3600.0 AS total_hours,
    AVG(te.duration_seconds) / 3600.0 AS avg_hours_per_entry
FROM pm_work_items wi
JOIN pm_time_entries te ON te.work_item_id = wi.id
WHERE
    wi.project_id = ?
    AND te.deleted_at IS NULL
    AND wi.deleted_at IS NULL
GROUP BY wi.id, wi.title, wi.item_type
ORDER BY total_seconds DESC;
```

### Currently Running Timers

```sql
SELECT
    wi.title,
    u.name AS user_name,
    te.started_at,
    (strftime('%s', 'now') - te.started_at) AS elapsed_seconds,
    te.description
FROM pm_time_entries te
JOIN pm_work_items wi ON wi.id = te.work_item_id
JOIN users u ON u.id = te.user_id
WHERE
    te.ended_at IS NULL
    AND te.deleted_at IS NULL
    AND wi.project_id = ?;
```

---

## Phase 5: Generating Reports

### Sprint Burndown Data

```sql
-- Get completed tasks per day during sprint
SELECT
    DATE(al.timestamp, 'unixepoch') AS completion_date,
    COUNT(DISTINCT al.entity_id) AS tasks_completed
FROM pm_activity_log al
JOIN pm_work_items wi ON wi.id = al.entity_id
WHERE
    al.action = 'status_changed'
    AND al.new_value = 'done'
    AND wi.sprint_id = ?
    AND al.timestamp BETWEEN ? AND ?  -- sprint start/end
GROUP BY completion_date
ORDER BY completion_date;
```

### Velocity Tracking

```sql
-- Average tasks completed per sprint (for forecasting)
SELECT
    s.name AS sprint_name,
    s.start_date,
    s.end_date,
    COUNT(wi.id) AS total_tasks,
    SUM(CASE WHEN wi.status = 'done' THEN 1 ELSE 0 END) AS completed_tasks,
    ROUND(
        CAST(SUM(CASE WHEN wi.status = 'done' THEN 1 ELSE 0 END) AS FLOAT) / COUNT(wi.id) * 100,
        1
    ) AS completion_percentage
FROM pm_sprints s
LEFT JOIN pm_work_items wi ON wi.sprint_id = s.id AND wi.deleted_at IS NULL
WHERE
    s.project_id = ?
    AND s.status = 'completed'
    AND s.deleted_at IS NULL
GROUP BY s.id
ORDER BY s.end_date DESC;
```

---

## Best Practices for LLM Queries

### 1. Always Filter Soft Deletes

```sql
-- GOOD
WHERE deleted_at IS NULL

-- BAD (includes deleted records)
WHERE id = ?
```

### 2. Use Descriptive Column Aliases

```sql
-- GOOD (human-readable results)
SELECT
    u.name AS assignee_name,
    creator.name AS created_by_name

-- BAD (confusing output)
SELECT u.name, u2.name
```

### 3. Include Context in Joins

```sql
-- GOOD (rich context)
LEFT JOIN users assignee ON assignee.id = wi.assignee_id
LEFT JOIN users creator ON creator.id = wi.created_by

-- BAD (missing context)
SELECT * FROM pm_work_items
```

### 4. Order Results Meaningfully

```sql
-- For UI display
ORDER BY position, created_at

-- For chronological analysis
ORDER BY timestamp DESC

-- For priority analysis
ORDER BY status, updated_at DESC
```

### 5. Limit Large Result Sets

```sql
-- Always limit unless you need everything
LIMIT 100
```

---

## Writing to the Database (Future: v2.0)

### Creating Work Items

```sql
-- 1. Generate UUID
-- 2. Insert with full audit fields
INSERT INTO pm_work_items (
    id, item_type, parent_id, project_id, title, description,
    status, assignee_id, sprint_id, position,
    created_at, updated_at, created_by, updated_by
) VALUES (
    ?, ?, ?, ?, ?, ?,
    'backlog', NULL, NULL, 0,
    strftime('%s', 'now'), strftime('%s', 'now'), ?, ?
);

-- 3. Log activity
INSERT INTO pm_activity_log (
    id, entity_type, entity_id, action,
    user_id, timestamp, comment
) VALUES (
    ?, 'work_item', ?,  'created',
    ?, strftime('%s', 'now'), 'Created via LLM'
);
```

### Updating Status

```sql
-- 1. Get old value first
SELECT status FROM pm_work_items WHERE id = ?;

-- 2. Update
UPDATE pm_work_items
SET
    status = ?,
    updated_at = strftime('%s', 'now'),
    updated_by = ?
WHERE id = ?;

-- 3. Log activity
INSERT INTO pm_activity_log (
    id, entity_type, entity_id, action,
    field_name, old_value, new_value,
    user_id, timestamp, comment
) VALUES (
    ?, 'work_item', ?, 'status_changed',
    'status', ?, ?,
    ?, strftime('%s', 'now'), 'Status changed via LLM'
);
```

---

## Error Handling

### Check Constraints

Before updating, validate:

```sql
-- Check if item exists and is not deleted
SELECT id FROM pm_work_items
WHERE id = ? AND deleted_at IS NULL;

-- Check if sprint assignment is valid
SELECT item_type FROM pm_work_items WHERE id = ?;
-- Only 'story' and 'task' can be assigned to sprints

-- Check if dependency would create a cycle
-- (Requires recursive CTE to detect)
```

### Foreign Key Violations

```sql
-- Enable foreign keys to catch violations
PRAGMA foreign_keys = ON;

-- SQLite will return error if:
-- - Assigning to non-existent sprint
-- - Referencing deleted parent
-- - Creating circular dependencies
```

---

## Performance Tips

### 1. Use Indexes

All foreign keys and common query columns are indexed. Use `EXPLAIN QUERY PLAN` to verify:

```sql
EXPLAIN QUERY PLAN
SELECT * FROM pm_work_items WHERE project_id = ?;
```

### 2. Avoid SELECT *

Only select needed columns to reduce I/O:

```sql
-- GOOD
SELECT id, title, status FROM pm_work_items

-- BAD
SELECT * FROM pm_work_items
```

### 3. Use Views for Complex Queries

The schema includes views like `pm_sprint_stats` and `pm_project_stats` for common aggregations.

---

## API Integration

For production use, LLMs should interact via REST API endpoints rather than direct SQL:

- `GET /api/v1/llm/context` - Returns database schema and instructions
- `GET /api/v1/projects/{id}/context` - Returns full project context as JSON
- `GET /api/v1/analytics/blocked-tasks` - Pre-built queries as endpoints
- `POST /api/v1/llm/query` - Execute safe, parameterized queries (v2.0)

This provides:
- Security (parameterized queries, validation)
- Caching (reduce database load)
- Rate limiting (prevent abuse)
- Logging (audit LLM interactions)
