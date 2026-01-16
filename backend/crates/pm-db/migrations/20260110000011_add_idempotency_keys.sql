-- Migration: add_idempotency_keys
-- Prevents duplicate creates on network retries

CREATE TABLE pm_idempotency_keys (
    message_id TEXT PRIMARY KEY,
    operation TEXT NOT NULL,
    result_json TEXT NOT NULL,
    created_at INTEGER NOT NULL
);

-- Cleanup old entries (run periodically via background job)
-- DELETE FROM pm_idempotency_keys WHERE created_at < (unixepoch() - 3600);