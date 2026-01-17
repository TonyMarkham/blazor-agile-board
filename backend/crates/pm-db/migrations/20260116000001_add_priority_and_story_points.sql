-- Migration: add_priority_and_story_points
-- Adds priority level and agile story points to work items

ALTER TABLE pm_work_items ADD COLUMN priority TEXT NOT NULL DEFAULT 'medium';
ALTER TABLE pm_work_items ADD COLUMN story_points INTEGER;