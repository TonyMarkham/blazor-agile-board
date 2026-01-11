mod common;

use common::{create_test_pool, create_test_project, create_test_user, create_test_work_item};

use pm_db::WorkItemRepository;

use chrono::Utc;
use googletest::prelude::*;
use uuid::Uuid;

#[tokio::test]
async fn given_valid_work_item_when_created_then_can_be_found_by_id() {
    // Given: A test database with a user and project
    let pool = create_test_pool().await;
    let user_id = Uuid::new_v4();
    create_test_user(&pool, user_id).await;

    let repo = WorkItemRepository::new(pool.clone());

    // Create the project first
    let project = create_test_project(user_id);
    repo.create(&project).await.unwrap();

    let work_item = create_test_work_item(project.id, user_id);

    // When: Creating the work item
    repo.create(&work_item).await.unwrap();

    // Then: Finding by ID returns the work item
    let result = repo.find_by_id(work_item.id).await.unwrap();

    assert_that!(result, some(anything()));
    let found = result.unwrap();
    assert_that!(found.id, eq(work_item.id));
    assert_that!(found.title, eq(&work_item.title));
    assert_that!(found.item_type, eq(&work_item.item_type));
}

#[tokio::test]
async fn given_empty_database_when_finding_nonexistent_id_then_returns_none() {
    // Given: An empty database
    let pool = create_test_pool().await;
    let repo = WorkItemRepository::new(pool);

    // When: Finding a work item that doesn't exist
    let nonexistent_id = Uuid::new_v4();
    let result = repo.find_by_id(nonexistent_id).await.unwrap();

    // Then: Returns None
    assert_that!(result, none());
}

#[tokio::test]
async fn given_existing_work_item_when_updated_then_changes_are_persisted() {
    // Given: A work item exists in the database
    let pool = create_test_pool().await;
    let user_id = Uuid::new_v4();
    create_test_user(&pool, user_id).await;

    let repo = WorkItemRepository::new(pool.clone());

    // Create the project first
    let project = create_test_project(user_id);
    repo.create(&project).await.unwrap();

    let mut work_item = create_test_work_item(project.id, user_id);
    repo.create(&work_item).await.unwrap();

    // When: Updating the work item's title and status
    work_item.title = "Updated Title".to_string();
    work_item.status = "in-progress".to_string();
    work_item.updated_at = Utc::now();
    repo.update(&work_item).await.unwrap();

    // Then: The changes are persisted
    let result = repo.find_by_id(work_item.id).await.unwrap();
    let found = result.unwrap();
    assert_that!(found.title, eq("Updated Title"));
    assert_that!(found.status, eq("in-progress"));
}

#[tokio::test]
async fn given_existing_work_item_when_soft_deleted_then_not_found_by_id() {
    // Given: A work item exists in the database
    let pool = create_test_pool().await;
    let user_id = Uuid::new_v4();
    create_test_user(&pool, user_id).await;

    let repo = WorkItemRepository::new(pool.clone());

    // Create the project first
    let project = create_test_project(user_id);
    repo.create(&project).await.unwrap();

    let work_item = create_test_work_item(project.id, user_id);
    repo.create(&work_item).await.unwrap();

    // When: Soft deleting the work item
    let deleted_at = Utc::now().timestamp();
    repo.delete(work_item.id, deleted_at).await.unwrap();

    // Then: find_by_id returns None
    let result = repo.find_by_id(work_item.id).await.unwrap();
    assert_that!(result, none());
}

#[tokio::test]
async fn given_multiple_work_items_in_project_when_finding_by_project_then_returns_all() {
    // Given: Multiple work items in the same project
    let pool = create_test_pool().await;
    let user_id = Uuid::new_v4();
    create_test_user(&pool, user_id).await;

    let repo = WorkItemRepository::new(pool.clone());

    // Create the project first
    let project = create_test_project(user_id);
    repo.create(&project).await.unwrap();

    let item1 = create_test_work_item(project.id, user_id);
    let item2 = create_test_work_item(project.id, user_id);
    let item3 = create_test_work_item(project.id, user_id);

    // When: Creating all items
    repo.create(&item1).await.unwrap();
    repo.create(&item2).await.unwrap();
    repo.create(&item3).await.unwrap();

    // Then: find_by_project returns all 3 items (+ the project itself = 4 total)
    let items = repo.find_by_project(project.id).await.unwrap();
    assert_that!(items, len(eq(4))); // Changed from 3 to 4!

    let ids: Vec<Uuid> = items.iter().map(|i| i.id).collect();
    assert_that!(ids, contains(eq(&project.id))); // Project is included
    assert_that!(ids, contains(eq(&item1.id)));
    assert_that!(ids, contains(eq(&item2.id)));
    assert_that!(ids, contains(eq(&item3.id)));
}

#[tokio::test]
async fn given_work_items_with_one_deleted_when_finding_by_project_then_excludes_deleted() {
    // Given: Multiple work items, one of which is deleted
    let pool = create_test_pool().await;
    let user_id = Uuid::new_v4();
    create_test_user(&pool, user_id).await;

    let repo = WorkItemRepository::new(pool.clone());

    // Create the project first
    let project = create_test_project(user_id);
    repo.create(&project).await.unwrap();

    let item1 = create_test_work_item(project.id, user_id);
    let item2 = create_test_work_item(project.id, user_id);

    repo.create(&item1).await.unwrap();
    repo.create(&item2).await.unwrap();

    // When: Soft deleting item1
    let deleted_at = Utc::now().timestamp();
    repo.delete(item1.id, deleted_at).await.unwrap();

    // Then: find_by_project returns project + item2 (2 total)
    let items = repo.find_by_project(project.id).await.unwrap();
    assert_that!(items, len(eq(2))); // Changed from 1 to 2!

    let ids: Vec<Uuid> = items.iter().map(|i| i.id).collect();
    assert_that!(ids, contains(eq(&project.id)));
    assert_that!(ids, contains(eq(&item2.id)));
}

#[tokio::test]
async fn given_empty_project_when_finding_by_project_then_returns_empty_vec() {
    // Given: An empty project (no work items created)
    let pool = create_test_pool().await;
    let user_id = Uuid::new_v4();
    create_test_user(&pool, user_id).await;

    let repo = WorkItemRepository::new(pool);

    // Create just the project
    let project = create_test_project(user_id);
    repo.create(&project).await.unwrap();

    // When: Finding work items by project
    let items = repo.find_by_project(project.id).await.unwrap();

    // Then: Returns only the project itself (1 item)
    assert_that!(items, len(eq(1)));
    assert_that!(items[0].id, eq(project.id));
}
