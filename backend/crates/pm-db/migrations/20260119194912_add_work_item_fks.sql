-- Migration: Add FK constraints to pm_work_items and fix all dependent tables
-- Strategy: With foreign_keys OFF, create FKs referencing final table names (not temp names)
-- WARNING: This migration drops all data in the affected tables

PRAGMA foreign_keys = OFF;

-- Step 1: Drop all existing tables
DROP TABLE IF EXISTS pm_work_items;
DROP TABLE IF EXISTS pm_sprints;
DROP TABLE IF EXISTS pm_comments;
DROP TABLE IF EXISTS pm_time_entries;
DROP TABLE IF EXISTS pm_dependencies;
DROP TABLE IF EXISTS pm_swim_lanes;
DROP TABLE IF EXISTS pm_project_members;

-- Step 2: Recreate pm_work_items with all FKs (including sprint_id)
-- NOTE: FK references pm_sprints which doesn't exist yet - OK because foreign_keys = OFF
CREATE TABLE pm_work_items (
    id TEXT PRIMARY KEY,
    item_type TEXT NOT NULL CHECK(item_type IN ('project', 'epic', 'story', 'task')),
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
    FOREIGN KEY (parent_id) REFERENCES pm_work_items(id) ON DELETE CASCADE,
    FOREIGN KEY (project_id) REFERENCES pm_work_items(id) ON DELETE CASCADE,
    FOREIGN KEY (sprint_id) REFERENCES pm_sprints(id) ON DELETE SET NULL,
    FOREIGN KEY (assignee_id) REFERENCES users(id) ON DELETE SET NULL
);

-- Step 3: Recreate pm_sprints with FK to pm_work_items (which now exists)
CREATE TABLE pm_sprints (
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
    FOREIGN KEY (project_id) REFERENCES pm_work_items(id) ON DELETE CASCADE,
    FOREIGN KEY (created_by) REFERENCES users(id),
    FOREIGN KEY (updated_by) REFERENCES users(id)
);

-- Step 4: Recreate all other dependent tables with correct FKs
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

CREATE TABLE pm_swim_lanes (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    name TEXT NOT NULL,
    status_value TEXT NOT NULL,
    position INTEGER NOT NULL DEFAULT 0,
    is_default BOOLEAN NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    deleted_at INTEGER,
    FOREIGN KEY (project_id) REFERENCES pm_work_items(id) ON DELETE CASCADE,
    UNIQUE(project_id, status_value)
);

CREATE TABLE pm_project_members (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    role TEXT NOT NULL CHECK(role IN ('viewer', 'editor', 'admin')),
    created_at INTEGER NOT NULL,
    FOREIGN KEY (project_id) REFERENCES pm_work_items(id) ON DELETE CASCADE,
    UNIQUE(project_id, user_id)
);

-- Step 5: Recreate all indexes
CREATE INDEX idx_pm_work_items_parent ON pm_work_items(parent_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_work_items_project ON pm_work_items(project_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_work_items_assignee ON pm_work_items(assignee_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_work_items_type ON pm_work_items(item_type) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_work_items_status ON pm_work_items(status) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_work_items_sprint ON pm_work_items(sprint_id) WHERE deleted_at IS NULL;

CREATE INDEX idx_pm_sprints_project ON pm_sprints(project_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_sprints_status ON pm_sprints(status) WHERE deleted_at IS NULL;

CREATE INDEX idx_pm_comments_work_item ON pm_comments(work_item_id) WHERE deleted_at IS NULL;

CREATE INDEX idx_pm_time_entries_work_item ON pm_time_entries(work_item_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_time_entries_user ON pm_time_entries(user_id) WHERE deleted_at IS NULL;

CREATE INDEX idx_pm_dependencies_blocking ON pm_dependencies(blocking_item_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_dependencies_blocked ON pm_dependencies(blocked_item_id) WHERE deleted_at IS NULL;

CREATE INDEX idx_pm_swim_lanes_project ON pm_swim_lanes(project_id) WHERE deleted_at IS NULL;

CREATE UNIQUE INDEX idx_pm_project_members_project_user ON pm_project_members(project_id, user_id);

PRAGMA foreign_keys = ON;
