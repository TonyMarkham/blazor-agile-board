-- Seed LLM context with self-documenting database information
-- This migration is idempotent (uses INSERT OR IGNORE)

-- Schema Documentation (Priority 100) - 10 entries
INSERT OR IGNORE INTO pm_llm_context
(id, context_type, category, title, content, priority, created_at, updated_at)
VALUES
('11111111-1111-1111-1111-000000000001', 'schema_doc', 'work_items', 'Work Item Hierarchy',
 'Single-table polymorphic design with item_type (project/epic/story/task). Uses parent_id for hierarchy and project_id for filtering. Position field for drag-and-drop ordering.',
 100, strftime('%s','now'), strftime('%s','now')),

('11111111-1111-1111-1111-000000000002', 'schema_doc', 'sprints', 'Sprint Lifecycle',
 'Status transitions: planned → active → completed/cancelled. Version field for optimistic locking. Start/end dates and velocity tracking.',
 100, strftime('%s','now'), strftime('%s','now')),

('11111111-1111-1111-1111-000000000003', 'schema_doc', 'comments', 'Comment Threading',
 'work_item_id FK for parent work item. author_id for ownership checks. parent_id for threaded replies (optional).',
 100, strftime('%s','now'), strftime('%s','now')),

('11111111-1111-1111-1111-000000000004', 'schema_doc', 'time_entries', 'Time Entry Structure',
 'Running timers have NULL ended_at. Duration computed on stop. One active timer per user enforced.',
 100, strftime('%s','now'), strftime('%s','now')),

('11111111-1111-1111-1111-000000000005', 'schema_doc', 'dependencies', 'Dependency Graph',
 'blocking_item_id → blocked_item_id relationship. dependency_type (blocks/relates). Same project constraint enforced.',
 100, strftime('%s','now'), strftime('%s','now')),

('11111111-1111-1111-1111-000000000006', 'schema_doc', 'activity_log', 'Activity Audit Trail',
 'entity_type + entity_id + action + field_name/old_value/new_value. Immutable append-only log. timestamp is Unix seconds.',
 100, strftime('%s','now'), strftime('%s','now')),

('11111111-1111-1111-1111-000000000007', 'schema_doc', 'general', 'Soft Delete Pattern',
 'All tables have deleted_at column. ALWAYS filter WHERE deleted_at IS NULL in queries. Preserves audit trail.',
 100, strftime('%s','now'), strftime('%s','now')),

('11111111-1111-1111-1111-000000000008', 'schema_doc', 'general', 'Audit Columns',
 'All mutable tables have: created_at, created_by, updated_at, updated_by. Tracks full change history.',
 100, strftime('%s','now'), strftime('%s','now')),

('11111111-1111-1111-1111-000000000009', 'schema_doc', 'general', 'UUID Primary Keys',
 'All IDs are TEXT storing UUID v4. Compare as strings. Enables distributed/offline creation.',
 100, strftime('%s','now'), strftime('%s','now')),

('11111111-1111-1111-1111-000000000010', 'schema_doc', 'swim_lanes', 'Swim Lanes',
 'Custom Kanban columns per project. Position field for ordering. Status mapping to work item statuses.',
 100, strftime('%s','now'), strftime('%s','now'));

-- Query Patterns (Priority 90) - 8 entries
INSERT OR IGNORE INTO pm_llm_context
(id, context_type, category, title, content, example_sql, example_description, priority, created_at, updated_at)
VALUES
('11111111-1111-1111-1111-000000000011', 'query_pattern', 'work_items', 'Project Hierarchy',
 'Recursive query to get full work item tree',
 'WITH RECURSIVE hierarchy AS (SELECT * FROM pm_work_items WHERE parent_id IS NULL UNION ALL SELECT wi.* FROM pm_work_items wi JOIN hierarchy h ON wi.parent_id = h.id) SELECT * FROM hierarchy WHERE deleted_at IS NULL',
 'Returns all work items in hierarchical order starting from top-level projects',
 90, strftime('%s','now'), strftime('%s','now')),

('11111111-1111-1111-1111-000000000012', 'query_pattern', 'dependencies', 'Find Blocked Items',
 'Query items blocked by other items',
 'SELECT wi.* FROM pm_work_items wi JOIN pm_dependencies d ON d.blocked_item_id = wi.id WHERE d.dependency_type = ''blocks'' AND d.deleted_at IS NULL AND wi.deleted_at IS NULL',
 'Returns work items that are blocked by dependencies',
 90, strftime('%s','now'), strftime('%s','now')),

('11111111-1111-1111-1111-000000000013', 'query_pattern', 'sprints', 'Sprint Velocity',
 'Calculate completed story points in a sprint',
 'SELECT SUM(story_points) as velocity FROM pm_work_items WHERE sprint_id = ?1 AND status = ''done'' AND deleted_at IS NULL',
 'Returns total story points completed in a sprint for velocity tracking',
 90, strftime('%s','now'), strftime('%s','now')),

('11111111-1111-1111-1111-000000000014', 'query_pattern', 'time_entries', 'Time Summary',
 'Sum time spent per work item',
 'SELECT work_item_id, SUM(duration_seconds) as total_seconds FROM pm_time_entries WHERE deleted_at IS NULL GROUP BY work_item_id',
 'Returns total time tracked for each work item',
 90, strftime('%s','now'), strftime('%s','now')),

('11111111-1111-1111-1111-000000000015', 'query_pattern', 'activity_log', 'Recent Activity',
 'Get recent changes across all entities',
 'SELECT * FROM pm_activity_log ORDER BY timestamp DESC LIMIT 50',
 'Returns 50 most recent activity log entries for dashboard',
 90, strftime('%s','now'), strftime('%s','now')),

('11111111-1111-1111-1111-000000000016', 'query_pattern', 'work_items', 'User Workload',
 'Count active items per assignee',
 'SELECT assignee_id, COUNT(*) as item_count FROM pm_work_items WHERE status != ''done'' AND deleted_at IS NULL GROUP BY assignee_id',
 'Returns workload distribution across team members',
 90, strftime('%s','now'), strftime('%s','now')),

('11111111-1111-1111-1111-000000000017', 'query_pattern', 'dependencies', 'Dependency Chain',
 'Find transitive dependency path',
 'WITH RECURSIVE deps AS (SELECT * FROM pm_dependencies WHERE blocking_item_id = ?1 UNION SELECT d.* FROM pm_dependencies d JOIN deps ON d.blocking_item_id = deps.blocked_item_id) SELECT * FROM deps',
 'Returns full chain of dependencies for an item (used for cycle detection)',
 90, strftime('%s','now'), strftime('%s','now')),

('11111111-1111-1111-1111-000000000018', 'query_pattern', 'work_items', 'Backlog Items',
 'Get unassigned items ordered by priority',
 'SELECT * FROM pm_work_items WHERE sprint_id IS NULL AND deleted_at IS NULL ORDER BY position ASC',
 'Returns backlog items in priority order for sprint planning',
 90, strftime('%s','now'), strftime('%s','now'));

-- Business Rules (Priority 80) - 5 entries
INSERT OR IGNORE INTO pm_llm_context
(id, context_type, category, title, content, priority, created_at, updated_at)
VALUES
('11111111-1111-1111-1111-000000000019', 'business_rule', 'work_items', 'Sprint Assignment',
 'Only stories and tasks can be assigned to sprints. Epics and projects cannot be sprint-assigned.',
 80, strftime('%s','now'), strftime('%s','now')),

('11111111-1111-1111-1111-000000000020', 'business_rule', 'time_entries', 'Timer Exclusivity',
 'Maximum one running timer per user (ended_at IS NULL). Must stop current timer before starting new one.',
 80, strftime('%s','now'), strftime('%s','now')),

('11111111-1111-1111-1111-000000000021', 'business_rule', 'comments', 'Comment Ownership',
 'Only comment author can edit or delete their comments (author_id check required).',
 80, strftime('%s','now'), strftime('%s','now')),

('11111111-1111-1111-1111-000000000022', 'business_rule', 'dependencies', 'Same-Project Constraint',
 'Both work items in a dependency must belong to same project. Cross-project dependencies not allowed.',
 80, strftime('%s','now'), strftime('%s','now')),

('11111111-1111-1111-1111-000000000023', 'business_rule', 'work_items', 'Status Transitions',
 'Valid status flow: todo → in_progress → review → done. Blocked can be set from any status. Customizable per project.',
 80, strftime('%s','now'), strftime('%s','now'));

-- Instructions (Priority 70) - 5 entries
INSERT OR IGNORE INTO pm_llm_context
(id, context_type, category, title, content, priority, created_at, updated_at)
VALUES
('11111111-1111-1111-1111-000000000024', 'instruction', 'general', 'Filter Deleted Records',
 'Always include WHERE deleted_at IS NULL in queries to exclude soft-deleted records.',
 70, strftime('%s','now'), strftime('%s','now')),

('11111111-1111-1111-1111-000000000025', 'instruction', 'work_items', 'Use Position for Ordering',
 'Use ORDER BY position ASC for display ordering in lists and boards. Position is user-managed.',
 70, strftime('%s','now'), strftime('%s','now')),

('11111111-1111-1111-1111-000000000026', 'instruction', 'general', 'UUID Comparison',
 'Compare UUIDs as TEXT strings (string equality). SQLite stores them as TEXT.',
 70, strftime('%s','now'), strftime('%s','now')),

('11111111-1111-1111-1111-000000000027', 'instruction', 'general', 'Limit Results',
 'Always use LIMIT in queries to prevent unbounded result sets. Default limit: 100.',
 70, strftime('%s','now'), strftime('%s','now')),

('11111111-1111-1111-1111-000000000028', 'instruction', 'activity_log', 'Use Activity Log for History',
 'Query pm_activity_log for change history. Do not query historical snapshots or maintain separate audit tables.',
 70, strftime('%s','now'), strftime('%s','now'));
