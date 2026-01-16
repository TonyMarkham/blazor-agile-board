-- Migration: add_version_column
-- Enables optimistic locking to prevent silent data loss from concurrent updates

ALTER TABLE pm_work_items ADD COLUMN version INTEGER NOT NULL DEFAULT 0;