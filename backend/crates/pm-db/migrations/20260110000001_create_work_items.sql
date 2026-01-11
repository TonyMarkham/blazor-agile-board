-- Enable foreign keys
PRAGMA foreign_keys = ON;

-- Work Items (Projects, Epics, Stories, Tasks)
CREATE TABLE pm_work_items (
    id TEXT PRIMARY KEY,
    item_type TEXT NOT NULL CHECK(item_type IN ('project', 'epic', 'story', 'task')),

    -- Hierarchy
    parent_id TEXT,
    project_id TEXT NOT NULL,
    position INTEGER NOT NULL DEFAULT 0,

    -- Core fields
    title TEXT NOT NULL,
    description TEXT,

    -- Workflow
    status TEXT NOT NULL DEFAULT 'backlog',

    -- Assignment
    assignee_id TEXT,

    -- Sprint
    sprint_id TEXT,

    -- Audit columns
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    created_by TEXT NOT NULL,
    updated_by TEXT NOT NULL,
    deleted_at INTEGER,

    FOREIGN KEY (parent_id) REFERENCES pm_work_items(id) ON DELETE CASCADE,
    FOREIGN KEY (project_id) REFERENCES pm_work_items(id) ON DELETE CASCADE
);

-- Indexes
CREATE INDEX idx_pm_work_items_parent ON pm_work_items(parent_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_work_items_project ON pm_work_items(project_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_work_items_assignee ON pm_work_items(assignee_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_work_items_type ON pm_work_items(item_type) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_work_items_status ON pm_work_items(status) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_work_items_sprint ON pm_work_items(sprint_id) WHERE deleted_at IS NULL;