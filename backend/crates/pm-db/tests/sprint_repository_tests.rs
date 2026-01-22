mod common;

use common::{create_test_pool, create_test_project, create_test_sprint, create_test_user};

use pm_core::SprintStatus;
use pm_db::{ProjectRepository, SprintRepository, WorkItemRepository};

use chrono::Utc;
use googletest::prelude::*;
use uuid::Uuid;

#[tokio::test]
async fn given_valid_sprint_when_created_then_can_be_found_by_id() {
    // Given: A test database with a user and project
    let pool = create_test_pool().await;
    let user_id = Uuid::new_v4();
    create_test_user(&pool, user_id).await;

    let project = create_test_project(user_id);
    // WorkItemRepository is now stateless
    ProjectRepository::new(pool.clone())
        .create(&project)
        .await
        .unwrap();

    let repo = SprintRepository::new(pool.clone());
    let sprint = create_test_sprint(project.id, user_id);

    // When: Creating the sprint
    repo.create(&sprint).await.unwrap();

    // Then: Finding by ID returns the sprint
    let result = repo.find_by_id(sprint.id).await.unwrap();

    assert_that!(result, some(anything()));
    let found = result.unwrap();
    assert_that!(found.id, eq(sprint.id));
    assert_that!(found.name, eq(&sprint.name));
    assert_that!(found.status, eq(&sprint.status));
}

#[tokio::test]
async fn given_empty_database_when_finding_nonexistent_id_then_returns_none() {
    // Given: An empty database
    let pool = create_test_pool().await;
    let repo = SprintRepository::new(pool);

    // When: Finding a sprint that doesn't exist
    let nonexistent_id = Uuid::new_v4();
    let result = repo.find_by_id(nonexistent_id).await.unwrap();

    // Then: Returns None
    assert_that!(result, none());
}

#[tokio::test]
async fn given_existing_sprint_when_updated_then_changes_are_persisted() {
    // Given: A sprint exists in the database
    let pool = create_test_pool().await;
    let user_id = Uuid::new_v4();
    create_test_user(&pool, user_id).await;

    let project = create_test_project(user_id);
    // WorkItemRepository is now stateless
    ProjectRepository::new(pool.clone())
        .create(&project)
        .await
        .unwrap();

    let repo = SprintRepository::new(pool.clone());
    let mut sprint = create_test_sprint(project.id, user_id);
    repo.create(&sprint).await.unwrap();

    // When: Updating the sprint's name and status
    sprint.name = "Updated Sprint".to_string();
    sprint.status = SprintStatus::Active;
    sprint.updated_at = Utc::now();
    repo.update(&sprint).await.unwrap();

    // Then: The changes are persisted
    let result = repo.find_by_id(sprint.id).await.unwrap();
    let found = result.unwrap();
    assert_that!(found.name, eq("Updated Sprint"));
    assert_that!(found.status, eq(&SprintStatus::Active));
}

#[tokio::test]
async fn given_existing_sprint_when_soft_deleted_then_not_found_by_id() {
    // Given: A sprint exists in the database
    let pool = create_test_pool().await;
    let user_id = Uuid::new_v4();
    create_test_user(&pool, user_id).await;

    let project = create_test_project(user_id);
    // WorkItemRepository is now stateless
    ProjectRepository::new(pool.clone())
        .create(&project)
        .await
        .unwrap();

    let repo = SprintRepository::new(pool.clone());
    let sprint = create_test_sprint(project.id, user_id);
    repo.create(&sprint).await.unwrap();

    // When: Soft deleting the sprint
    let deleted_at = Utc::now().timestamp();
    repo.delete(sprint.id, deleted_at).await.unwrap();

    // Then: find_by_id returns None
    let result = repo.find_by_id(sprint.id).await.unwrap();
    assert_that!(result, none());
}

#[tokio::test]
async fn given_multiple_sprints_in_project_when_finding_by_project_then_returns_all() {
    // Given: Multiple sprints in the same project
    let pool = create_test_pool().await;
    let user_id = Uuid::new_v4();
    create_test_user(&pool, user_id).await;

    let project = create_test_project(user_id);
    // WorkItemRepository is now stateless
    ProjectRepository::new(pool.clone())
        .create(&project)
        .await
        .unwrap();

    let repo = SprintRepository::new(pool.clone());

    let sprint1 = create_test_sprint(project.id, user_id);
    let sprint2 = create_test_sprint(project.id, user_id);
    let sprint3 = create_test_sprint(project.id, user_id);

    // When: Creating all sprints
    repo.create(&sprint1).await.unwrap();
    repo.create(&sprint2).await.unwrap();
    repo.create(&sprint3).await.unwrap();

    // Then: find_by_project returns all 3 sprints
    let sprints = repo.find_by_project(project.id).await.unwrap();
    assert_that!(sprints, len(eq(3)));

    let ids: Vec<Uuid> = sprints.iter().map(|s| s.id).collect();
    assert_that!(ids, contains(eq(&sprint1.id)));
    assert_that!(ids, contains(eq(&sprint2.id)));
    assert_that!(ids, contains(eq(&sprint3.id)));
}

#[tokio::test]
async fn given_sprints_with_one_deleted_when_finding_by_project_then_excludes_deleted() {
    // Given: Multiple sprints, one of which is deleted
    let pool = create_test_pool().await;
    let user_id = Uuid::new_v4();
    create_test_user(&pool, user_id).await;

    let project = create_test_project(user_id);
    // WorkItemRepository is now stateless
    ProjectRepository::new(pool.clone())
        .create(&project)
        .await
        .unwrap();

    let repo = SprintRepository::new(pool.clone());

    let sprint1 = create_test_sprint(project.id, user_id);
    let sprint2 = create_test_sprint(project.id, user_id);

    repo.create(&sprint1).await.unwrap();
    repo.create(&sprint2).await.unwrap();

    // When: Soft deleting sprint1
    let deleted_at = Utc::now().timestamp();
    repo.delete(sprint1.id, deleted_at).await.unwrap();

    // Then: find_by_project returns only sprint2
    let sprints = repo.find_by_project(project.id).await.unwrap();
    assert_that!(sprints, len(eq(1)));
    assert_that!(sprints[0].id, eq(sprint2.id));
}

#[tokio::test]
async fn given_empty_project_when_finding_by_project_then_returns_empty_vec() {
    // Given: A project with no sprints
    let pool = create_test_pool().await;
    let user_id = Uuid::new_v4();
    create_test_user(&pool, user_id).await;

    let project = create_test_project(user_id);
    // WorkItemRepository is now stateless
    ProjectRepository::new(pool.clone())
        .create(&project)
        .await
        .unwrap();

    let repo = SprintRepository::new(pool);

    // When: Finding sprints by project
    let sprints = repo.find_by_project(project.id).await.unwrap();

    // Then: Returns empty vector
    assert_that!(sprints, is_empty());
}
