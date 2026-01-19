# Session 20.01: Database FK Constraint Fixes

**Parent Plan**: [session-20-plan.md](session-20-plan.md)
**Target**: ~10k tokens
**Prerequisites**: Session 10 complete (backend with 166 tests passing)

---

## Scope

This sub-session fixes structural debt in the database schema before building the frontend:
- Add missing FK constraint: `pm_work_items.sprint_id` → `pm_sprints.id`
- Add missing FK constraint: `pm_work_items.assignee_id` → `users.id`

---

## Phase 0.1: Migration File

SQLite does not support `ALTER TABLE ADD CONSTRAINT` for foreign keys. We must recreate the table.

**Create**: `backend/crates/pm-db/migrations/20260119000001_add_work_item_fks.sql`

> **Note**: Migration number uses date 20260119 to avoid collision with existing migration 20260110000011 (add_idempotency_keys).

```sql
-- Migration: Add missing FK constraints to pm_work_items
-- SQLite requires table recreation to add FK constraints

PRAGMA foreign_keys = OFF;

-- Step 1: Rename old table
ALTER TABLE pm_work_items RENAME TO pm_work_items_old;

-- Step 2: Create new table with final name (critical for self-referential FKs)
-- IMPORTANT: Must include ALL columns from original table
CREATE TABLE pm_work_items (
    id TEXT PRIMARY KEY,
    item_type TEXT NOT NULL CHECK(item_type IN ('project', 'epic', 'story', 'task')),

    -- Hierarchy
    parent_id TEXT,
    project_id TEXT NOT NULL,
    position INTEGER NOT NULL DEFAULT 0,

    -- Core fields
    title TEXT NOT NULL,
    description TEXT,

    -- Workflow
    status TEXT NOT NULL DEFAULT 'backlog',
    priority TEXT NOT NULL DEFAULT 'medium',

    -- Assignment (NOW WITH FK)
    assignee_id TEXT,

    -- Sprint (NOW WITH FK)
    sprint_id TEXT,

    -- Estimation
    story_points INTEGER,

    -- Optimistic concurrency
    version INTEGER NOT NULL DEFAULT 1,

    -- Audit columns
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    created_by TEXT NOT NULL,
    updated_by TEXT NOT NULL,
    deleted_at INTEGER,

    -- Self-referential FKs (must reference final table name)
    FOREIGN KEY (parent_id) REFERENCES pm_work_items(id) ON DELETE CASCADE,
    FOREIGN KEY (project_id) REFERENCES pm_work_items(id) ON DELETE CASCADE,

    -- NEW FKs
    FOREIGN KEY (sprint_id) REFERENCES pm_sprints(id) ON DELETE SET NULL,
    FOREIGN KEY (assignee_id) REFERENCES users(id) ON DELETE SET NULL
);

-- Step 3: Copy data
INSERT INTO pm_work_items SELECT * FROM pm_work_items_old;

-- Step 4: Drop old table
DROP TABLE pm_work_items_old;

-- Step 5: Recreate indexes
CREATE INDEX idx_pm_work_items_parent ON pm_work_items(parent_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_work_items_project ON pm_work_items(project_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_work_items_assignee ON pm_work_items(assignee_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_work_items_type ON pm_work_items(item_type) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_work_items_status ON pm_work_items(status) WHERE deleted_at IS NULL;
CREATE INDEX idx_pm_work_items_sprint ON pm_work_items(sprint_id) WHERE deleted_at IS NULL;

PRAGMA foreign_keys = ON;
```

**Verification**: File created, syntax valid

---

## Phase 0.2: Design Decisions

**ON DELETE SET NULL vs CASCADE**:
- `sprint_id` → SET NULL: When a sprint is deleted, work items should remain but become unassigned
- `assignee_id` → SET NULL: When a user is deleted, work items should remain but become unassigned

This differs from `parent_id`/`project_id` which use CASCADE because deleting a parent should cascade to children.

**Migration approach**: Rename old → create new with final name → copy → drop old. This ensures self-referential FKs correctly reference `pm_work_items`, not a temp table name.

---

## Phase 0.3: Prepare and Execute Migration

The existing test infrastructure uses in-memory SQLite with `sqlx::migrate!("./migrations")` macro, which automatically picks up all migration files from the `migrations/` directory.

**Steps** (from repo root):

```bash
# 1. Build to verify compilation (sqlx offline mode uses .sqlx/ cache)
cargo build --workspace

# 2. Run existing tests - migrations run automatically via create_test_pool()
cargo test --workspace

# 3. If offline cache needs updating (compile errors about schema):
DATABASE_URL=sqlite:backend/crates/pm-db/.sqlx-test/test.db cargo sqlx prepare --workspace
```

**How it works**:
- `create_test_pool()` creates an in-memory SQLite database
- `sqlx::migrate!("./migrations")` runs ALL migrations in order
- Each test gets a fresh database with the new FK constraints automatically

---

## Phase 0.4: Test Updates

**Modify**: `backend/crates/pm-db/tests/work_item_repository_tests.rs`

Add FK constraint tests:

```rust
#[tokio::test]
async fn test_work_item_sprint_fk_constraint() {
    let pool = setup_test_db().await;
    let repo = WorkItemRepository::new(pool.clone());

    // Create project first
    let project = create_test_project(&repo).await;

    // Create story with non-existent sprint_id should fail
    let story = WorkItem {
        id: Uuid::new_v4(),
        item_type: WorkItemType::Story,
        parent_id: Some(project.id),
        project_id: project.id,
        sprint_id: Some(Uuid::new_v4()), // Non-existent sprint
        // ... other fields
    };

    let result = repo.create(&story).await;
    assert!(result.is_err(), "Should reject invalid sprint_id FK");
}

#[tokio::test]
async fn test_sprint_delete_nullifies_work_item_sprint() {
    let pool = setup_test_db().await;
    let work_item_repo = WorkItemRepository::new(pool.clone());
    let sprint_repo = SprintRepository::new(pool.clone());

    // Create project, sprint, and story assigned to sprint
    let project = create_test_project(&work_item_repo).await;
    let sprint = create_test_sprint(&sprint_repo, project.id).await;
    let story = create_test_story(&work_item_repo, project.id, Some(sprint.id)).await;

    // Delete sprint
    sprint_repo.delete(sprint.id).await.unwrap();

    // Verify story's sprint_id is now NULL
    let updated_story = work_item_repo.find_by_id(story.id).await.unwrap().unwrap();
    assert!(updated_story.sprint_id.is_none(), "Sprint deletion should SET NULL on work items");
}
```

---

## Phase 0.5: Documentation Update

**Modify**: `docs/database-relationships.md`

Update to reflect the fix:

```markdown
## Foreign Key Constraints (Complete)

| From Table | Column | To Table | Column | ON DELETE |
|------------|--------|----------|--------|-----------|
| `pm_work_items` | `sprint_id` | `pm_sprints` | `id` | SET NULL |
| `pm_work_items` | `assignee_id` | `users` | `id` | SET NULL |
```

---

## Files Summary

| File | Action | Description |
|------|--------|-------------|
| `backend/crates/pm-db/migrations/20260119000001_add_work_item_fks.sql` | Create | Migration to add FK constraints |
| `backend/crates/pm-db/tests/work_item_repository_tests.rs` | Modify | Add FK constraint tests |
| `docs/database-relationships.md` | Modify | Update to show FKs are complete |

**Total**: 3 files

---

## Success Criteria

- [ ] Migration runs successfully on fresh database
- [ ] Migration runs successfully on database with existing data
- [ ] Existing tests still pass
- [ ] New FK constraint tests pass
- [ ] `cargo test --workspace` passes (166+ tests)
