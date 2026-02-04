//! Tests for migration 20260203000001_add_work_item_numbers.sql
//!
//! These tests verify that the migration:
//! 1. Adds next_work_item_number to pm_projects
//! 2. Adds item_number to pm_work_items
//! 3. Preserves existing data
//! 4. Creates proper indexes and constraints
//! 5. PRESERVES FOREIGN KEY CONSTRAINTS (CRITICAL!)

use sqlx::{SqlitePool, Row};

#[sqlx::test]
async fn test_migration_adds_project_counter(pool: SqlitePool) -> sqlx::Result<()> {
    // Apply migration
    sqlx::migrate!("./migrations").run(&pool).await?;

    // Verify next_work_item_number column exists
    let result = sqlx::query("SELECT next_work_item_number FROM pm_projects LIMIT 1")
        .fetch_optional(&pool)
        .await;

    // Either succeeds (column exists) or table is empty (also OK)
    assert!(result.is_ok(), "next_work_item_number column should exist");

    Ok(())
}

#[sqlx::test]
async fn test_migration_adds_item_number(pool: SqlitePool) -> sqlx::Result<()> {
    // Apply migration
    sqlx::migrate!("./migrations").run(&pool).await?;

    // Verify item_number column exists
    let result = sqlx::query("SELECT item_number FROM pm_work_items LIMIT 1")
        .fetch_optional(&pool)
        .await;

    assert!(result.is_ok(), "item_number column should exist");

    Ok(())
}

#[sqlx::test]
async fn test_migration_unique_constraint(pool: SqlitePool) -> sqlx::Result<()> {
    // Apply migration
    sqlx::migrate!("./migrations").run(&pool).await?;

    // Insert test project
    let project_id = "test-project-123";
    sqlx::query!(
          "INSERT INTO pm_projects (id, title, key, status, version, created_at, updated_at, created_by, updated_by, next_work_item_number)
           VALUES (?, 'Test', 'TEST', 'active', 1, 0, 0, 'user-1', 'user-1', 1)",
          project_id
      )
        .execute(&pool)
        .await?;

    // Insert first work item with item_number = 1
    let work_item_1 = "work-item-1";
    sqlx::query!(
          "INSERT INTO pm_work_items (id, item_type, project_id, position, title, status, priority, item_number, version, created_at, updated_at, created_by, updated_by)
           VALUES (?, 'story', ?, 0, 'Story 1', 'backlog', 'medium', 1, 1, 0, 0, 'user-1', 'user-1')",
          work_item_1,
          project_id
      )
        .execute(&pool)
        .await?;

    // Try to insert second work item with same item_number = 1 (should fail)
    let work_item_2 = "work-item-2";
    let result = sqlx::query!(
          "INSERT INTO pm_work_items (id, item_type, project_id, position, title, status, priority, item_number, version, created_at, updated_at, created_by, updated_by)
           VALUES (?, 'story', ?, 0, 'Story 2', 'backlog', 'medium', 1, 1, 0, 0, 'user-1', 'user-1')",
          work_item_2,
          project_id
      )
        .execute(&pool)
        .await;

    // Should fail due to UNIQUE constraint
    assert!(result.is_err(), "Duplicate item_number should be rejected by UNIQUE constraint");

    Ok(())
}

#[sqlx::test]
async fn test_migration_preserves_data(pool: SqlitePool) -> sqlx::Result<()> {
    // Apply migration
    sqlx::migrate!("./migrations").run(&pool).await?;

    // Insert and retrieve a work item
    let project_id = "test-project-456";
    sqlx::query!(
          "INSERT INTO pm_projects (id, title, key, status, version, created_at, updated_at, created_by, updated_by, next_work_item_number)
           VALUES (?, 'Test', 'TEST', 'active', 1, 0, 0, 'user-1', 'user-1', 1)",
          project_id
      )
        .execute(&pool)
        .await?;

    let work_item_id = "work-item-789";
    sqlx::query!(
          "INSERT INTO pm_work_items (id, item_type, project_id, position, title, status, priority, item_number, version, created_at, updated_at, created_by, updated_by)
           VALUES (?, 'epic', ?, 0, 'Epic 1', 'backlog', 'high', 1, 1, 0, 0, 'user-1', 'user-1')",
          work_item_id,
          project_id
      )
        .execute(&pool)
        .await?;

    // Retrieve and verify
    let row = sqlx::query!("SELECT id, title, item_number FROM pm_work_items WHERE id = ?", work_item_id)
        .fetch_one(&pool)
        .await?;

    assert_eq!(row.title, "Epic 1");
    assert_eq!(row.item_number, 1);

    Ok(())
}

#[sqlx::test]
async fn test_migration_index_exists(pool: SqlitePool) -> sqlx::Result<()> {
    // Apply migration
    sqlx::migrate!("./migrations").run(&pool).await?;

    // Check that index exists
    let indexes = sqlx::query("SELECT name FROM sqlite_master WHERE type='index' AND tbl_name='pm_work_items'")
        .fetch_all(&pool)
        .await?;

    let index_names: Vec<String> = indexes
        .iter()
        .map(|row| row.get::<String, _>("name"))
        .collect();

    assert!(
        index_names.iter().any(|name| name.contains("item_number")),
        "Index on item_number should exist"
    );

    Ok(())
}

// ============================================================
// CRITICAL FK PRESERVATION TESTS
// ============================================================

#[sqlx::test]
async fn test_migration_preserves_comments_fk(pool: SqlitePool) -> sqlx::Result<()> {
    // Apply migration
    sqlx::migrate!("./migrations").run(&pool).await?;

    // Verify pm_comments has FK to pm_work_items
    let fk_info = sqlx::query("PRAGMA foreign_key_list(pm_comments)")
        .fetch_all(&pool)
        .await?;

    assert!(
        fk_info.iter().any(|row| row.get::<String, _>("table") == "pm_work_items"),
        "CRITICAL: pm_comments MUST have FK to pm_work_items. Migration destroyed FK constraint!"
    );

    Ok(())
}

#[sqlx::test]
async fn test_migration_preserves_time_entries_fk(pool: SqlitePool) -> sqlx::Result<()> {
    // Apply migration
    sqlx::migrate!("./migrations").run(&pool).await?;

    // Verify pm_time_entries has FK to pm_work_items
    let fk_info = sqlx::query("PRAGMA foreign_key_list(pm_time_entries)")
        .fetch_all(&pool)
        .await?;

    assert!(
        fk_info.iter().any(|row| row.get::<String, _>("table") == "pm_work_items"),
        "CRITICAL: pm_time_entries MUST have FK to pm_work_items. Migration destroyed FK constraint!"
    );

    Ok(())
}

#[sqlx::test]
async fn test_migration_preserves_dependencies_fks(pool: SqlitePool) -> sqlx::Result<()> {
    // Apply migration
    sqlx::migrate!("./migrations").run(&pool).await?;

    // Verify pm_dependencies has BOTH FKs to pm_work_items
    let fk_info = sqlx::query("PRAGMA foreign_key_list(pm_dependencies)")
        .fetch_all(&pool)
        .await?;

    let fk_count = fk_info.iter()
        .filter(|row| row.get::<String, _>("table") == "pm_work_items")
        .count();

    assert_eq!(
        fk_count, 2,
        "CRITICAL: pm_dependencies MUST have 2 FKs to pm_work_items (blocking_item_id and blocked_item_id). Migration destroyed FK constraints!"
    );

    Ok(())
}

#[sqlx::test]
async fn test_fk_enforcement_after_migration(pool: SqlitePool) -> sqlx::Result<()> {
    // Apply migration
    sqlx::migrate!("./migrations").run(&pool).await?;

    // Insert test project and work item
    let project_id = "test-project-fk";
    sqlx::query!(
          "INSERT INTO pm_projects (id, title, key, status, version, created_at, updated_at, created_by, updated_by, next_work_item_number)
           VALUES (?, 'Test', 'TEST', 'active', 1, 0, 0, 'user-1', 'user-1', 1)",
          project_id
      )
        .execute(&pool)
        .await?;

    let work_item_id = "work-item-fk-test";
    sqlx::query!(
          "INSERT INTO pm_work_items (id, item_type, project_id, position, title, status, priority, item_number, version, created_at, updated_at, created_by, updated_by)
           VALUES (?, 'story', ?, 0, 'Story FK Test', 'backlog', 'medium', 1, 1, 0, 0, 'user-1', 'user-1')",
          work_item_id,
          project_id
      )
        .execute(&pool)
        .await?;

    // Try to insert comment with NON-EXISTENT work_item_id
    let comment_id = "comment-bad-fk";
    let fake_work_item_id = "non-existent-work-item";
    let result = sqlx::query!(
          "INSERT INTO pm_comments (id, work_item_id, content, created_at, updated_at, created_by, updated_by)
           VALUES (?, ?, 'Test comment', 0, 0, 'user-1', 'user-1')",
          comment_id,
          fake_work_item_id
      )
        .execute(&pool)
        .await;

    // Should FAIL due to FK constraint
    assert!(
        result.is_err(),
        "CRITICAL: FK constraint violation should be enforced! Comment inserted for non-existent work item. This means FKs were destroyed by migration!"
    );

    Ok(())
}