-- Time Entries
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

-- Indexes
CREATE INDEX idx_pm_time_entries_work_item ON pm_time_entries(work_item_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_time_entries_user ON pm_time_entries(user_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_time_entries_started ON pm_time_entries(started_at DESC) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_time_entries_running ON pm_time_entries(ended_at) WHERE ended_at IS NULL AND deleted_at IS NULL;