-- Dependencies
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

-- Indexes
CREATE INDEX idx_pm_dependencies_blocking ON pm_dependencies(blocking_item_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_dependencies_blocked ON pm_dependencies(blocked_item_id) WHERE deleted_at IS NULL;