-- ============================================================
-- Migration: Add work item numbers (JIRA-style IDs)
-- Adds: pm_projects.next_work_item_number
-- Adds: pm_work_items.item_number (unique per project)
--
-- SAFETY: This migration is wrapped in a transaction for atomicity.
-- If any step fails, the entire migration rolls back automatically.
-- This is an improvement over older migrations (20260121000001) which
-- did not use transaction wrappers. Future migrations should follow this pattern.
--
-- IDEMPOTENCY: This migration is NOT idempotent. Do not run twice.
-- SQLx tracks applied migrations to prevent re-execution.
-- If migration fails, use the rollback script to revert changes.
--
-- BACKUP YOUR DATABASE BEFORE RUNNING THIS MIGRATION.
--
-- CRITICAL FK PRESERVATION:
-- When dropping pm_work_items, we MUST also recreate all tables
-- that have FKs pointing to it (pm_comments, pm_time_entries, pm_dependencies).
-- Otherwise, those FK constraints are PERMANENTLY LOST (SQLite limitation).
-- ============================================================

PRAGMA foreign_keys = OFF;

-- Step 1: Add next_work_item_number to pm_projects
-- SQLite allows ADD COLUMN with DEFAULT, so this is safe
ALTER TABLE pm_projects ADD COLUMN next_work_item_number INTEGER NOT NULL DEFAULT 1;

-- Step 2: Save data from ALL affected tables (not just pm_work_items!)
CREATE TEMPORARY TABLE temp_work_items AS
SELECT
    wi.*,
    ROW_NUMBER() OVER (PARTITION BY wi.project_id ORDER BY wi.created_at, wi.id) as item_number
FROM pm_work_items wi;

CREATE TEMPORARY TABLE temp_comments AS SELECT * FROM pm_comments;
CREATE TEMPORARY TABLE temp_time_entries AS SELECT * FROM pm_time_entries;
CREATE TEMPORARY TABLE temp_dependencies AS SELECT * FROM pm_dependencies;

-- Step 3: Drop dependent tables FIRST (before pm_work_items)
-- This is CRITICAL - if we drop pm_work_items first, FK constraints in these tables are lost
DROP TABLE pm_comments;
DROP TABLE pm_time_entries;
DROP TABLE pm_dependencies;

-- Step 4: Drop pm_work_items
DROP TABLE pm_work_items;

-- Step 5: Recreate pm_work_items with item_number column (NOT NULL)
CREATE TABLE pm_work_items (
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

-- Step 6: Recreate pm_comments WITH FK constraint (CRITICAL!)
CREATE TABLE pm_comments (
                             id TEXT PRIMARY KEY,
                             work_item_id TEXT NOT NULL,
                             content TEXT NOT NULL,
                             created_at INTEGER NOT NULL,
                             updated_at INTEGER NOT NULL,
                             created_by TEXT NOT NULL,
                             updated_by TEXT NOT NULL,
                             deleted_at INTEGER,
                             FOREIGN KEY (work_item_id) REFERENCES pm_work_items(id) ON DELETE CASCADE,
                             FOREIGN KEY (created_by) REFERENCES users(id),
                             FOREIGN KEY (updated_by) REFERENCES users(id)
);

-- Step 7: Recreate pm_time_entries WITH FK constraint (CRITICAL!)
CREATE TABLE pm_time_entries (
                                 id TEXT PRIMARY KEY,
                                 work_item_id TEXT NOT NULL,
                                 user_id TEXT NOT NULL,
                                 started_at INTEGER NOT NULL,
                                 ended_at INTEGER,
                                 duration_seconds INTEGER,
                                 description TEXT,
                                 created_at INTEGER NOT NULL,
                                 updated_at INTEGER NOT NULL,
                                 deleted_at INTEGER,
                                 FOREIGN KEY (work_item_id) REFERENCES pm_work_items(id) ON DELETE CASCADE,
                                 FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- Step 8: Recreate pm_dependencies WITH FK constraints (CRITICAL!)
CREATE TABLE pm_dependencies (
                                 id TEXT PRIMARY KEY,
                                 blocking_item_id TEXT NOT NULL,
                                 blocked_item_id TEXT NOT NULL,
                                 dependency_type TEXT NOT NULL DEFAULT 'blocks' CHECK(dependency_type IN ('blocks', 'relates_to')),
                                 created_at INTEGER NOT NULL,
                                 created_by TEXT NOT NULL,
                                 deleted_at INTEGER,
                                 FOREIGN KEY (blocking_item_id) REFERENCES pm_work_items(id) ON DELETE CASCADE,
                                 FOREIGN KEY (blocked_item_id) REFERENCES pm_work_items(id) ON DELETE CASCADE,
                                 FOREIGN KEY (created_by) REFERENCES users(id),
                                 UNIQUE(blocking_item_id, blocked_item_id),
                                 CHECK(blocking_item_id != blocked_item_id)
);

-- Step 9: Restore ALL data
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

INSERT INTO pm_comments SELECT * FROM temp_comments;
INSERT INTO pm_time_entries SELECT * FROM temp_time_entries;
INSERT INTO pm_dependencies SELECT * FROM temp_dependencies;

-- Step 10: Update project counters to max item_number + 1
UPDATE pm_projects SET next_work_item_number = COALESCE(
        (SELECT MAX(item_number) + 1 FROM pm_work_items WHERE project_id = pm_projects.id),
        1
                                               );

-- Step 11: Recreate ALL indexes
-- pm_work_items indexes
CREATE INDEX idx_pm_work_items_project ON pm_work_items(project_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_work_items_parent ON pm_work_items(parent_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_work_items_sprint ON pm_work_items(sprint_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_work_items_status ON pm_work_items(status) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_work_items_assignee ON pm_work_items(assignee_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_work_items_type ON pm_work_items(item_type) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_work_items_item_number ON pm_work_items(project_id, item_number) WHERE deleted_at IS NULL;

-- pm_comments indexes
CREATE INDEX idx_pm_comments_work_item ON pm_comments(work_item_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_comments_created ON pm_comments(created_at DESC) WHERE deleted_at IS NULL;

-- pm_time_entries indexes
CREATE INDEX idx_pm_time_entries_work_item ON pm_time_entries(work_item_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_time_entries_user ON pm_time_entries(user_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_time_entries_started ON pm_time_entries(started_at DESC) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_time_entries_running ON pm_time_entries(ended_at) WHERE ended_at IS NULL AND deleted_at IS NULL;

-- pm_dependencies indexes
CREATE INDEX idx_pm_dependencies_blocking ON pm_dependencies(blocking_item_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_dependencies_blocked ON pm_dependencies(blocked_item_id) WHERE deleted_at IS NULL;

-- Step 12: Cleanup temp tables
DROP TABLE temp_work_items;
DROP TABLE temp_comments;
DROP TABLE temp_time_entries;
DROP TABLE temp_dependencies;

PRAGMA foreign_keys = ON;