-- Users table (platform table stub for foreign keys)
-- In production, this would be provided by the host platform
-- For local development and testing, we create a minimal version

CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,
    email TEXT,
    name TEXT,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
    );

CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);