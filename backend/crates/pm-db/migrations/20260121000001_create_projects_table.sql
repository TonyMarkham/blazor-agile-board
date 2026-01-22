-- ============================================================
-- Migration: Create pm_projects table and migrate data
-- Strategy: With foreign_keys OFF, create FKs referencing final table names
-- WARNING: This migration recreates tables (data is preserved via INSERT)
-- ============================================================

PRAGMA foreign_keys = OFF;

  -- Step 1: Create pm_projects table (NEW TABLE - doesn't exist yet)
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

-- Step 3: Drop and recreate pm_sprints with FK to pm_projects
-- Save existing data first
CREATE TEMPORARY TABLE temp_sprints AS SELECT * FROM pm_sprints;
DROP TABLE pm_sprints;

CREATE TABLE pm_sprints (
                            id TEXT PRIMARY KEY,
                            project_id TEXT NOT NULL,
                            name TEXT NOT NULL,
                            goal TEXT,
                            start_date INTEGER NOT NULL,
                            end_date INTEGER NOT NULL,
                            status TEXT NOT NULL DEFAULT 'planned' CHECK(status IN ('planned', 'active', 'completed', 'cancelled')),
                            velocity INTEGER,
                            version INTEGER NOT NULL DEFAULT 1,
                            created_at INTEGER NOT NULL,
                            updated_at INTEGER NOT NULL,
                            created_by TEXT NOT NULL,
                            updated_by TEXT NOT NULL,
                            deleted_at INTEGER,
                            FOREIGN KEY (project_id) REFERENCES pm_projects(id) ON DELETE CASCADE
);
-- Explicitly list columns (old schema has no velocity/version)
INSERT INTO pm_sprints (id, project_id, name, goal, start_date, end_date, status, created_at, updated_at, created_by, updated_by, deleted_at)
SELECT id, project_id, name, goal, start_date, end_date, status, created_at, updated_at, created_by, updated_by, deleted_at
FROM temp_sprints;
DROP TABLE temp_sprints;

-- Step 4: Drop and recreate pm_swim_lanes with FK to pm_projects
CREATE TEMPORARY TABLE temp_swim_lanes AS SELECT * FROM pm_swim_lanes;
DROP TABLE pm_swim_lanes;

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
                               FOREIGN KEY (project_id) REFERENCES pm_projects(id) ON DELETE CASCADE,
                               UNIQUE(project_id, status_value)
);
INSERT INTO pm_swim_lanes SELECT * FROM temp_swim_lanes;
DROP TABLE temp_swim_lanes;

-- Step 5: Drop and recreate pm_project_members with FK to pm_projects
CREATE TEMPORARY TABLE temp_project_members AS SELECT * FROM pm_project_members;
DROP TABLE pm_project_members;

CREATE TABLE pm_project_members (
                                    id TEXT PRIMARY KEY,
                                    project_id TEXT NOT NULL,
                                    user_id TEXT NOT NULL,
                                    role TEXT NOT NULL CHECK(role IN ('viewer', 'editor', 'admin')),
                                    created_at INTEGER NOT NULL,
                                    FOREIGN KEY (project_id) REFERENCES pm_projects(id) ON DELETE CASCADE,
                                    UNIQUE(project_id, user_id)
);
INSERT INTO pm_project_members SELECT * FROM temp_project_members;
DROP TABLE temp_project_members;

-- Step 6: Drop and recreate pm_work_items (remove 'project' type, FK to pm_projects)
CREATE TEMPORARY TABLE temp_work_items AS SELECT * FROM pm_work_items WHERE item_type != 'project';
DROP TABLE pm_work_items;

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
                               version INTEGER NOT NULL DEFAULT 1,
                               created_at INTEGER NOT NULL,
                               updated_at INTEGER NOT NULL,
                               created_by TEXT NOT NULL,
                               updated_by TEXT NOT NULL,
                               deleted_at INTEGER,
                               FOREIGN KEY (project_id) REFERENCES pm_projects(id) ON DELETE CASCADE,
                               FOREIGN KEY (parent_id) REFERENCES pm_work_items(id) ON DELETE SET NULL,
                               FOREIGN KEY (sprint_id) REFERENCES pm_sprints(id) ON DELETE SET NULL,
                               FOREIGN KEY (assignee_id) REFERENCES users(id) ON DELETE SET NULL
);
INSERT INTO pm_work_items (id, item_type, parent_id, project_id, position, title, description, status, priority, assignee_id, sprint_id, story_points, version, created_at, updated_at, created_by, updated_by, deleted_at)
SELECT id, item_type, parent_id, project_id, position, title, description, status, priority, assignee_id, sprint_id, story_points, version, created_at, updated_at, created_by, updated_by, deleted_at
FROM temp_work_items;
DROP TABLE temp_work_items;

-- Step 7: Recreate all indexes
CREATE INDEX idx_pm_projects_status ON pm_projects(status) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_projects_key ON pm_projects(key) WHERE deleted_at IS NULL;

CREATE INDEX idx_pm_sprints_project ON pm_sprints(project_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_sprints_dates ON pm_sprints(start_date, end_date) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_sprints_status ON pm_sprints(status) WHERE deleted_at IS NULL;

CREATE INDEX idx_pm_swim_lanes_project ON pm_swim_lanes(project_id) WHERE deleted_at IS NULL;

CREATE UNIQUE INDEX idx_pm_project_members_project_user ON pm_project_members(project_id, user_id);
CREATE INDEX idx_pm_project_members_project ON pm_project_members(project_id);
CREATE INDEX idx_pm_project_members_user ON pm_project_members(user_id);

CREATE INDEX idx_pm_work_items_project ON pm_work_items(project_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_work_items_parent ON pm_work_items(parent_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_work_items_sprint ON pm_work_items(sprint_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_work_items_status ON pm_work_items(status) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_work_items_assignee ON pm_work_items(assignee_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_work_items_type ON pm_work_items(item_type) WHERE deleted_at IS NULL;

PRAGMA foreign_keys = ON;