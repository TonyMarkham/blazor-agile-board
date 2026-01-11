-- Swim Lanes
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

-- Indexes
CREATE INDEX idx_pm_swim_lanes_project ON pm_swim_lanes(project_id) WHERE deleted_at IS NULL;