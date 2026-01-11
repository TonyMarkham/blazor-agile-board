mod common;

use common::{
    create_test_dependency, create_test_pool, create_test_project, create_test_user,
    create_test_work_item,
};

use pm_db::DependencyRepository;
use pm_db::WorkItemRepository;

use chrono::Utc;
use googletest::prelude::*;
use uuid::Uuid;

#[tokio::test]
async fn given_valid_dependency_when_created_then_can_be_found_by_id() {
    // Given: A test database with a user, project, and two work items
    let pool = create_test_pool().await;
    let user_id = Uuid::new_v4();
    create_test_user(&pool, user_id).await;

    let project = create_test_project(user_id);
    let work_item_repo = WorkItemRepository::new(pool.clone());
    work_item_repo.create(&project).await.unwrap();

    let item1 = create_test_work_item(project.id, user_id);
    let item2 = create_test_work_item(project.id, user_id);
    work_item_repo.create(&item1).await.unwrap();
    work_item_repo.create(&item2).await.unwrap();

    let repo = DependencyRepository::new(pool.clone());
    let dependency = create_test_dependency(item1.id, item2.id, user_id);

    // When: Creating the dependency
    repo.create(&dependency).await.unwrap();

    // Then: Finding by ID returns the dependency
    let result = repo.find_by_id(dependency.id).await.unwrap();

    assert_that!(result, some(anything()));
    let found = result.unwrap();
    assert_that!(found.id, eq(dependency.id));
    assert_that!(found.blocking_item_id, eq(dependency.blocking_item_id));
    assert_that!(found.blocked_item_id, eq(dependency.blocked_item_id));
}

#[tokio::test]
async fn given_empty_database_when_finding_nonexistent_id_then_returns_none() {
    // Given: An empty database
    let pool = create_test_pool().await;
    let repo = DependencyRepository::new(pool);

    // When: Finding a dependency that doesn't exist
    let nonexistent_id = Uuid::new_v4();
    let result = repo.find_by_id(nonexistent_id).await.unwrap();

    // Then: Returns None
    assert_that!(result, none());
}

#[tokio::test]
async fn given_existing_dependency_when_soft_deleted_then_not_found_by_id() {
    // Given: A dependency exists in the database
    let pool = create_test_pool().await;
    let user_id = Uuid::new_v4();
    create_test_user(&pool, user_id).await;

    let project = create_test_project(user_id);
    let work_item_repo = WorkItemRepository::new(pool.clone());
    work_item_repo.create(&project).await.unwrap();

    let item1 = create_test_work_item(project.id, user_id);
    let item2 = create_test_work_item(project.id, user_id);
    work_item_repo.create(&item1).await.unwrap();
    work_item_repo.create(&item2).await.unwrap();

    let repo = DependencyRepository::new(pool.clone());
    let dependency = create_test_dependency(item1.id, item2.id, user_id);
    repo.create(&dependency).await.unwrap();

    // When: Soft deleting the dependency
    let deleted_at = Utc::now().timestamp();
    repo.delete(dependency.id, deleted_at).await.unwrap();

    // Then: find_by_id returns None
    let result = repo.find_by_id(dependency.id).await.unwrap();
    assert_that!(result, none());
}

#[tokio::test]
async fn given_item_with_blockers_when_finding_blocking_then_returns_blockers() {
    // Given: Item C is blocked by items A and B
    let pool = create_test_pool().await;
    let user_id = Uuid::new_v4();
    create_test_user(&pool, user_id).await;

    let project = create_test_project(user_id);
    let work_item_repo = WorkItemRepository::new(pool.clone());
    work_item_repo.create(&project).await.unwrap();

    let item_a = create_test_work_item(project.id, user_id);
    let item_b = create_test_work_item(project.id, user_id);
    let item_c = create_test_work_item(project.id, user_id);
    work_item_repo.create(&item_a).await.unwrap();
    work_item_repo.create(&item_b).await.unwrap();
    work_item_repo.create(&item_c).await.unwrap();

    let repo = DependencyRepository::new(pool.clone());

    // A blocks C
    let dep1 = create_test_dependency(item_a.id, item_c.id, user_id);
    // B blocks C
    let dep2 = create_test_dependency(item_b.id, item_c.id, user_id);

    repo.create(&dep1).await.unwrap();
    repo.create(&dep2).await.unwrap();

    // When: Finding what's blocking item C
    let blockers = repo.find_blocking(item_c.id).await.unwrap();

    // Then: Returns both A and B as blockers
    assert_that!(blockers, len(eq(2)));

    let blocking_ids: Vec<Uuid> = blockers.iter().map(|d| d.blocking_item_id).collect();
    assert_that!(blocking_ids, contains(eq(&item_a.id)));
    assert_that!(blocking_ids, contains(eq(&item_b.id)));
}

#[tokio::test]
async fn given_item_blocking_others_when_finding_blocked_then_returns_blocked_items() {
    // Given: Item A blocks items B and C
    let pool = create_test_pool().await;
    let user_id = Uuid::new_v4();
    create_test_user(&pool, user_id).await;

    let project = create_test_project(user_id);
    let work_item_repo = WorkItemRepository::new(pool.clone());
    work_item_repo.create(&project).await.unwrap();

    let item_a = create_test_work_item(project.id, user_id);
    let item_b = create_test_work_item(project.id, user_id);
    let item_c = create_test_work_item(project.id, user_id);
    work_item_repo.create(&item_a).await.unwrap();
    work_item_repo.create(&item_b).await.unwrap();
    work_item_repo.create(&item_c).await.unwrap();

    let repo = DependencyRepository::new(pool.clone());

    // A blocks B
    let dep1 = create_test_dependency(item_a.id, item_b.id, user_id);
    // A blocks C
    let dep2 = create_test_dependency(item_a.id, item_c.id, user_id);

    repo.create(&dep1).await.unwrap();
    repo.create(&dep2).await.unwrap();

    // When: Finding what items A is blocking
    let blocked = repo.find_blocked(item_a.id).await.unwrap();

    // Then: Returns both B and C as blocked items
    assert_that!(blocked, len(eq(2)));

    let blocked_ids: Vec<Uuid> = blocked.iter().map(|d| d.blocked_item_id).collect();
    assert_that!(blocked_ids, contains(eq(&item_b.id)));
    assert_that!(blocked_ids, contains(eq(&item_c.id)));
}

#[tokio::test]
async fn given_item_with_no_blockers_when_finding_blocking_then_returns_empty_vec() {
    // Given: An item with no blocking dependencies
    let pool = create_test_pool().await;
    let user_id = Uuid::new_v4();
    create_test_user(&pool, user_id).await;

    let project = create_test_project(user_id);
    let work_item_repo = WorkItemRepository::new(pool.clone());
    work_item_repo.create(&project).await.unwrap();

    let item = create_test_work_item(project.id, user_id);
    work_item_repo.create(&item).await.unwrap();

    let repo = DependencyRepository::new(pool);

    // When: Finding blocking dependencies
    let blockers = repo.find_blocking(item.id).await.unwrap();

    // Then: Returns empty vector
    assert_that!(blockers, is_empty());
}

#[tokio::test]
async fn given_item_blocking_none_when_finding_blocked_then_returns_empty_vec() {
    // Given: An item that doesn't block anything
    let pool = create_test_pool().await;
    let user_id = Uuid::new_v4();
    create_test_user(&pool, user_id).await;

    let project = create_test_project(user_id);
    let work_item_repo = WorkItemRepository::new(pool.clone());
    work_item_repo.create(&project).await.unwrap();

    let item = create_test_work_item(project.id, user_id);
    work_item_repo.create(&item).await.unwrap();

    let repo = DependencyRepository::new(pool);

    // When: Finding blocked items
    let blocked = repo.find_blocked(item.id).await.unwrap();

    // Then: Returns empty vector
    assert_that!(blocked, is_empty());
}

#[tokio::test]
async fn given_dependencies_with_one_deleted_when_finding_blocking_then_excludes_deleted() {
    // Given: Item C has two blockers, but one dependency is deleted
    let pool = create_test_pool().await;
    let user_id = Uuid::new_v4();
    create_test_user(&pool, user_id).await;

    let project = create_test_project(user_id);
    let work_item_repo = WorkItemRepository::new(pool.clone());
    work_item_repo.create(&project).await.unwrap();

    let item_a = create_test_work_item(project.id, user_id);
    let item_b = create_test_work_item(project.id, user_id);
    let item_c = create_test_work_item(project.id, user_id);
    work_item_repo.create(&item_a).await.unwrap();
    work_item_repo.create(&item_b).await.unwrap();
    work_item_repo.create(&item_c).await.unwrap();

    let repo = DependencyRepository::new(pool.clone());

    let dep1 = create_test_dependency(item_a.id, item_c.id, user_id);
    let dep2 = create_test_dependency(item_b.id, item_c.id, user_id);

    repo.create(&dep1).await.unwrap();
    repo.create(&dep2).await.unwrap();

    // When: Deleting dep1
    let deleted_at = Utc::now().timestamp();
    repo.delete(dep1.id, deleted_at).await.unwrap();

    // Then: find_blocking returns only dep2
    let blockers = repo.find_blocking(item_c.id).await.unwrap();
    assert_that!(blockers, len(eq(1)));
    assert_that!(blockers[0].blocking_item_id, eq(item_b.id));
}
