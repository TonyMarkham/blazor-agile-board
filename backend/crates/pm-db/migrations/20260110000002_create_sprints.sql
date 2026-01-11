-- Sprints
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

-- Indexes
CREATE INDEX idx_pm_sprints_project ON pm_sprints(project_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_sprints_dates ON pm_sprints(start_date, end_date) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_sprints_status ON pm_sprints(status) WHERE deleted_at IS NULL;