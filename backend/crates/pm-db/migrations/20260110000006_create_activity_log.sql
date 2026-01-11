-- Activity Log
CREATE TABLE pm_activity_log (
    id TEXT PRIMARY KEY,

    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,

    action TEXT NOT NULL,

    field_name TEXT,
    old_value TEXT,
    new_value TEXT,

    user_id TEXT NOT NULL,
    timestamp INTEGER NOT NULL,

    comment TEXT,

    FOREIGN KEY (user_id) REFERENCES users(id)
);

-- Indexes
CREATE INDEX idx_pm_activity_log_entity ON pm_activity_log(entity_type, entity_id);
CREATE INDEX idx_pm_activity_log_timestamp ON pm_activity_log(timestamp DESC);
CREATE INDEX idx_pm_activity_log_user ON pm_activity_log(user_id);
CREATE INDEX idx_pm_activity_log_action ON pm_activity_log(action);