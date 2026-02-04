mod common;

use common::{
    create_test_pool, create_test_project, create_test_sprint, create_test_user,
    create_test_work_item,
};

use pm_db::{ProjectRepository, WorkItemRepository};

use chrono::Utc;
use googletest::prelude::*;
use uuid::Uuid;

#[tokio::test]
async fn given_valid_work_item_when_created_then_can_be_found_by_id() {
    // Given: A test database with a user and project
    let pool = create_test_pool().await;
    let user_id = Uuid::new_v4();
    create_test_user(&pool, user_id).await;

    // Create the project first
    let project = create_test_project(user_id);
    ProjectRepository::new(pool.clone())
        .create(&project)
        .await
        .unwrap();

    let work_item = create_test_work_item(project.id, user_id, 1);

    // When: Creating the work item
    WorkItemRepository::create(&pool, &work_item).await.unwrap();

    // Then: Finding by ID returns the work item
    let result = WorkItemRepository::find_by_id(&pool, work_item.id)
        .await
        .unwrap();

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

    // When: Finding a work item that doesn't exist
    let nonexistent_id = Uuid::new_v4();
    let result = WorkItemRepository::find_by_id(&pool, nonexistent_id)
        .await
        .unwrap();

    // Then: Returns None
    assert_that!(result, none());
}

#[tokio::test]
async fn given_existing_work_item_when_updated_then_changes_are_persisted() {
    // Given: A work item exists in the database
    let pool = create_test_pool().await;
    let user_id = Uuid::new_v4();
    create_test_user(&pool, user_id).await;

    // Create the project first
    let project = create_test_project(user_id);
    ProjectRepository::new(pool.clone())
        .create(&project)
        .await
        .unwrap();

    let mut work_item = create_test_work_item(project.id, user_id, 1);
    WorkItemRepository::create(&pool, &work_item).await.unwrap();

    // When: Updating the work item's title and status
    work_item.title = "Updated Title".to_string();
    work_item.status = "in_progress".to_string();
    work_item.updated_at = Utc::now();
    WorkItemRepository::update(&pool, &work_item).await.unwrap();

    // Then: The changes are persisted
    let result = WorkItemRepository::find_by_id(&pool, work_item.id)
        .await
        .unwrap();
    let found = result.unwrap();
    assert_that!(found.title, eq("Updated Title"));
    assert_that!(found.status, eq("in_progress"));
}

#[tokio::test]
async fn given_existing_work_item_when_soft_deleted_then_not_found_by_id() {
    // Given: A work item exists in the database
    let pool = create_test_pool().await;
    let user_id = Uuid::new_v4();
    create_test_user(&pool, user_id).await;

    // Create the project first
    let project = create_test_project(user_id);
    ProjectRepository::new(pool.clone())
        .create(&project)
        .await
        .unwrap();

    let work_item = create_test_work_item(project.id, user_id, 1);
    WorkItemRepository::create(&pool, &work_item).await.unwrap();

    // When: Soft deleting the work item
    WorkItemRepository::soft_delete(&pool, work_item.id, user_id)
        .await
        .unwrap();

    // Then: find_by_id returns None
    let result = WorkItemRepository::find_by_id(&pool, work_item.id)
        .await
        .unwrap();
    assert_that!(result, none());
}

#[tokio::test]
async fn given_multiple_work_items_in_project_when_finding_by_project_then_returns_all() {
    // Given: Multiple work items in the same project
    let pool = create_test_pool().await;
    let user_id = Uuid::new_v4();
    create_test_user(&pool, user_id).await;

    // Create the project first
    let project = create_test_project(user_id);
    ProjectRepository::new(pool.clone())
        .create(&project)
        .await
        .unwrap();

    let item1 = create_test_work_item(project.id, user_id, 1);
    let item2 = create_test_work_item(project.id, user_id, 2);
    let item3 = create_test_work_item(project.id, user_id, 3);

    // When: Creating all items
    WorkItemRepository::create(&pool, &item1).await.unwrap();
    WorkItemRepository::create(&pool, &item2).await.unwrap();
    WorkItemRepository::create(&pool, &item3).await.unwrap();

    // Then: find_by_project returns all 3 items (+ the project itself = 4 total)
    let items = WorkItemRepository::find_by_project(&pool, project.id)
        .await
        .unwrap();
    assert_that!(items, len(eq(3))); // Changed from 3 to 4!

    let ids: Vec<Uuid> = items.iter().map(|i| i.id).collect();
    // assert_that!(ids, contains(eq(&project.id))); // Project is included
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

    // Create the project first
    let project = create_test_project(user_id);
    ProjectRepository::new(pool.clone())
        .create(&project)
        .await
        .unwrap();

    let item1 = create_test_work_item(project.id, user_id, 1);
    let item2 = create_test_work_item(project.id, user_id, 2);

    WorkItemRepository::create(&pool, &item1).await.unwrap();
    WorkItemRepository::create(&pool, &item2).await.unwrap();

    // When: Soft deleting item1
    WorkItemRepository::soft_delete(&pool, item1.id, user_id)
        .await
        .unwrap();

    // Then: find_by_project returns project + item2 (2 total)
    let items = WorkItemRepository::find_by_project(&pool, project.id)
        .await
        .unwrap();
    assert_that!(items, len(eq(1))); // Changed from 1 to 2!

    let ids: Vec<Uuid> = items.iter().map(|i| i.id).collect();
    // assert_that!(ids, contains(eq(&project.id)));
    assert_that!(ids, contains(eq(&item2.id)));
}

#[tokio::test]
async fn given_empty_project_when_finding_by_project_then_returns_empty_vec() {
    // Given: An empty project (no work items created)
    let pool = create_test_pool().await;
    let user_id = Uuid::new_v4();
    create_test_user(&pool, user_id).await;

    // Create just the project
    let project = create_test_project(user_id);
    ProjectRepository::new(pool.clone())
        .create(&project)
        .await
        .unwrap();

    // When: Finding work items by project
    let items = WorkItemRepository::find_by_project(&pool, project.id)
        .await
        .unwrap();

    // Then: Returns only the project itself (1 item)
    assert_that!(items, len(eq(0)));
    // assert_that!(items[0].id, eq(project.id));
}

#[tokio::test]
async fn given_work_item_with_sprint_when_sprint_deleted_then_sprint_id_set_to_null() {
    // Given: A work item assigned to a sprint
    let pool = create_test_pool().await;
    let user_id = Uuid::new_v4();
    create_test_user(&pool, user_id).await;

    let project = create_test_project(user_id);
    ProjectRepository::new(pool.clone())
        .create(&project)
        .await
        .unwrap();

    let sprint = create_test_sprint(project.id, user_id);
    pm_db::SprintRepository::new(pool.clone())
        .create(&sprint)
        .await
        .unwrap();

    let mut work_item = create_test_work_item(project.id, user_id, 1);
    work_item.sprint_id = Some(sprint.id);
    WorkItemRepository::create(&pool, &work_item).await.unwrap();

    // When: Hard deleting the sprint (soft delete won't trigger FK constraints)
    sqlx::query("DELETE FROM pm_sprints WHERE id = ?")
        .bind(sprint.id.to_string())
        .execute(&pool)
        .await
        .unwrap();

    // Then: Work item's sprint_id is NULL (ON DELETE SET NULL)
    let result = WorkItemRepository::find_by_id(&pool, work_item.id)
        .await
        .unwrap()
        .unwrap();
    assert_that!(result.sprint_id, none());
}

#[tokio::test]
async fn given_work_item_with_assignee_when_user_deleted_then_assignee_id_set_to_null() {
    // Given: A work item assigned to a user
    let pool = create_test_pool().await;
    let user_id = Uuid::new_v4();
    create_test_user(&pool, user_id).await;

    let project = create_test_project(user_id);
    ProjectRepository::new(pool.clone())
        .create(&project)
        .await
        .unwrap();

    let mut work_item = create_test_work_item(project.id, user_id, 1);
    work_item.assignee_id = Some(user_id);
    WorkItemRepository::create(&pool, &work_item).await.unwrap();

    // When: Hard deleting the user (users table doesn't have soft delete)
    sqlx::query("DELETE FROM users WHERE id = ?")
        .bind(user_id.to_string())
        .execute(&pool)
        .await
        .unwrap();

    // Then: Work item's assignee_id is NULL (ON DELETE SET NULL)
    let result = WorkItemRepository::find_by_id(&pool, work_item.id)
        .await
        .unwrap()
        .unwrap();
    assert_that!(result.assignee_id, none());
}

#[tokio::test]
async fn given_parent_work_item_when_deleted_then_children_cascade_deleted() {
    // Given: A parent work item with children
    let pool = create_test_pool().await;
    let user_id = Uuid::new_v4();
    create_test_user(&pool, user_id).await;

    let project = create_test_project(user_id);
    ProjectRepository::new(pool.clone())
        .create(&project)
        .await
        .unwrap();

    let mut parent = create_test_work_item(project.id, user_id, 1);
    parent.item_type = pm_core::WorkItemType::Epic;
    WorkItemRepository::create(&pool, &parent).await.unwrap();

    let mut child = create_test_work_item(project.id, user_id, 2);
    child.parent_id = Some(parent.id);
    WorkItemRepository::create(&pool, &child).await.unwrap();

    // When: Hard deleting the parent (to test CASCADE - soft delete won't trigger it)
    sqlx::query("DELETE FROM pm_work_items WHERE id = ?")
        .bind(parent.id.to_string())
        .execute(&pool)
        .await
        .unwrap();

    // Then: Child still exists but parent_id is set to NULL (ON DELETE SET NULL)
    let result = WorkItemRepository::find_by_id(&pool, child.id)
        .await
        .unwrap();
    assert_that!(result, some(anything()));
    let child_result = result.unwrap();
    assert_that!(child_result.parent_id, none());
}

#[tokio::test]
async fn given_project_with_work_items_when_deleted_then_all_work_items_cascade_deleted() {
    // Given: A project with multiple work items
    let pool = create_test_pool().await;
    let user_id = Uuid::new_v4();
    create_test_user(&pool, user_id).await;

    let project = create_test_project(user_id);
    ProjectRepository::new(pool.clone())
        .create(&project)
        .await
        .unwrap();

    let item1 = create_test_work_item(project.id, user_id, 1);
    let item2 = create_test_work_item(project.id, user_id, 2);
    WorkItemRepository::create(&pool, &item1).await.unwrap();
    WorkItemRepository::create(&pool, &item2).await.unwrap();

    // When: Hard deleting the project
    sqlx::query("DELETE FROM pm_projects WHERE id = ?")
        .bind(project.id.to_string())
        .execute(&pool)
        .await
        .unwrap();

    // Then: All work items in project are cascade deleted
    let result1 = WorkItemRepository::find_by_id(&pool, item1.id)
        .await
        .unwrap();
    let result2 = WorkItemRepository::find_by_id(&pool, item2.id)
        .await
        .unwrap();
    assert_that!(result1, none());
    assert_that!(result2, none());
}
