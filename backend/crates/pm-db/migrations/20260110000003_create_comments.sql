-- Comments
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

-- Indexes
CREATE INDEX idx_pm_comments_work_item ON pm_comments(work_item_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_comments_created ON pm_comments(created_at DESC) WHERE deleted_at IS NULL;