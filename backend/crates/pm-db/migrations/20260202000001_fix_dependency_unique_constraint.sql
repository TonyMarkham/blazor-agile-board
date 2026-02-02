-- Fix UNIQUE constraint on pm_dependencies to respect soft deletes
--
-- Problem: The UNIQUE(blocking_item_id, blocked_item_id) constraint
-- applies to ALL rows, including soft-deleted ones. This prevents
-- re-adding a dependency after deleting it.
--
-- Solution: Drop the table-level UNIQUE constraint and create a partial
-- UNIQUE index that only applies to non-deleted rows.

-- Step 1: Create a new table with the corrected schema (without UNIQUE constraint)
CREATE TABLE pm_dependencies_new (
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

                                     CHECK(blocking_item_id != blocked_item_id)
    );

-- Step 2: Copy all data from old table to new table
INSERT INTO pm_dependencies_new
SELECT id, blocking_item_id, blocked_item_id, dependency_type, created_at, created_by, deleted_at
FROM pm_dependencies;

-- Step 3: Drop old table
DROP TABLE pm_dependencies;

-- Step 4: Rename new table to original name
ALTER TABLE pm_dependencies_new RENAME TO pm_dependencies;

-- Step 5: Recreate indexes (including the new partial UNIQUE index)
CREATE INDEX idx_pm_dependencies_blocking ON pm_dependencies(blocking_item_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_dependencies_blocked ON pm_dependencies(blocked_item_id) WHERE deleted_at IS NULL;

-- Step 6: Create partial UNIQUE index that only applies to non-deleted rows
CREATE UNIQUE INDEX idx_pm_dependencies_unique_pair
    ON pm_dependencies(blocking_item_id, blocked_item_id)
    WHERE deleted_at IS NULL;