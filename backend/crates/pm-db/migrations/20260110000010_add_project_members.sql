-- Migration: add_project_members
-- Enforces project-level access control

CREATE TABLE pm_project_members (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    role TEXT NOT NULL CHECK(role IN ('viewer', 'editor', 'admin')),
    created_at INTEGER NOT NULL,
    FOREIGN KEY (project_id) REFERENCES pm_work_items(id),
    UNIQUE(project_id, user_id)
);

CREATE INDEX idx_pm_project_members_project ON pm_project_members(project_id);
CREATE INDEX idx_pm_project_members_user ON pm_project_members(user_id);