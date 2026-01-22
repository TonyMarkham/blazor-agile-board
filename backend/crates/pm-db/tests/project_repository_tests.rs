mod common;

use common::{create_test_pool, create_test_project, create_test_user};

use pm_core::ProjectStatus;
use pm_db::ProjectRepository;

use chrono::Utc;
use googletest::prelude::*;
use uuid::Uuid;

#[tokio::test]
async fn given_valid_project_when_created_then_can_be_found_by_id() {
    // Given: A test database with a user
    let pool = create_test_pool().await;
    let user_id = Uuid::new_v4();
    create_test_user(&pool, user_id).await;

    let project = create_test_project(user_id);
    let repo = ProjectRepository::new(pool.clone());

    // When: Creating the project
    repo.create(&project).await.unwrap();

    // Then: Finding by ID returns the project
    let result = repo.find_by_id(project.id).await.unwrap();

    assert_that!(result, some(anything()));
    let found = result.unwrap();
    assert_that!(found.id, eq(project.id));
    assert_that!(found.title, eq(&project.title));
    assert_that!(found.key, eq(&project.key));
    assert_that!(found.version, eq(1));
}

#[tokio::test]
async fn given_valid_project_when_created_then_can_be_found_by_key() {
    // Given: A test database with a user
    let pool = create_test_pool().await;
    let user_id = Uuid::new_v4();
    create_test_user(&pool, user_id).await;

    let project = create_test_project(user_id);
    let repo = ProjectRepository::new(pool.clone());

    // When: Creating the project
    repo.create(&project).await.unwrap();

    // Then: Finding by key returns the project
    let result = repo.find_by_key(&project.key).await.unwrap();

    assert_that!(result, some(anything()));
    let found = result.unwrap();
    assert_that!(found.id, eq(project.id));
    assert_that!(found.key, eq(&project.key));
}

#[tokio::test]
async fn given_empty_database_when_finding_nonexistent_id_then_returns_none() {
    // Given: An empty database
    let pool = create_test_pool().await;
    let repo = ProjectRepository::new(pool);

    // When: Finding a project that doesn't exist
    let nonexistent_id = Uuid::new_v4();
    let result = repo.find_by_id(nonexistent_id).await.unwrap();

    // Then: Returns None
    assert_that!(result, none());
}

#[tokio::test]
async fn given_empty_database_when_finding_nonexistent_key_then_returns_none() {
    // Given: An empty database
    let pool = create_test_pool().await;
    let repo = ProjectRepository::new(pool);

    // When: Finding a project key that doesn't exist
    let result = repo.find_by_key("NONEXISTENT").await.unwrap();

    // Then: Returns None
    assert_that!(result, none());
}

#[tokio::test]
async fn given_existing_project_when_updated_then_changes_are_persisted() {
    // Given: A project exists in the database
    let pool = create_test_pool().await;
    let user_id = Uuid::new_v4();
    create_test_user(&pool, user_id).await;

    let repo = ProjectRepository::new(pool.clone());
    let mut project = create_test_project(user_id);
    repo.create(&project).await.unwrap();

    // When: Updating the project's title and status
    project.title = "Updated Project".to_string();
    project.status = ProjectStatus::Archived;
    project.version = 2;
    project.updated_at = Utc::now();
    repo.update(&project).await.unwrap();

    // Then: The changes are persisted
    let result = repo.find_by_id(project.id).await.unwrap();
    let found = result.unwrap();
    assert_that!(found.title, eq("Updated Project"));
    assert_that!(found.status, eq(ProjectStatus::Archived));
    assert_that!(found.version, eq(2));
}

#[tokio::test]
async fn given_existing_project_when_soft_deleted_then_not_found_by_id() {
    // Given: A project exists in the database
    let pool = create_test_pool().await;
    let user_id = Uuid::new_v4();
    create_test_user(&pool, user_id).await;

    let repo = ProjectRepository::new(pool.clone());
    let project = create_test_project(user_id);
    repo.create(&project).await.unwrap();

    // When: Soft deleting the project
    let deleted_at = Utc::now().timestamp();
    repo.delete(project.id, deleted_at).await.unwrap();

    // Then: find_by_id returns None
    let result = repo.find_by_id(project.id).await.unwrap();
    assert_that!(result, none());
}

#[tokio::test]
async fn given_existing_project_when_soft_deleted_then_not_found_by_key() {
    // Given: A project exists in the database
    let pool = create_test_pool().await;
    let user_id = Uuid::new_v4();
    create_test_user(&pool, user_id).await;

    let repo = ProjectRepository::new(pool.clone());
    let project = create_test_project(user_id);
    repo.create(&project).await.unwrap();

    // When: Soft deleting the project
    let deleted_at = Utc::now().timestamp();
    repo.delete(project.id, deleted_at).await.unwrap();

    // Then: find_by_key returns None
    let result = repo.find_by_key(&project.key).await.unwrap();
    assert_that!(result, none());
}

#[tokio::test]
async fn given_multiple_projects_when_finding_all_then_returns_all() {
    // Given: Multiple projects
    let pool = create_test_pool().await;
    let user_id = Uuid::new_v4();
    create_test_user(&pool, user_id).await;

    let repo = ProjectRepository::new(pool.clone());

    let mut project1 = create_test_project(user_id);
    project1.key = "PROJ1".to_string();
    let mut project2 = create_test_project(user_id);
    project2.key = "PROJ2".to_string();

    // When: Creating all projects
    repo.create(&project1).await.unwrap();
    repo.create(&project2).await.unwrap();

    // Then: find_all returns both projects
    let projects = repo.find_all().await.unwrap();
    assert_that!(projects, len(eq(2)));

    let ids: Vec<Uuid> = projects.iter().map(|p| p.id).collect();
    assert_that!(ids, contains(eq(&project1.id)));
    assert_that!(ids, contains(eq(&project2.id)));
}

#[tokio::test]
async fn given_projects_with_one_deleted_when_finding_all_then_excludes_deleted() {
    // Given: Multiple projects, one of which is deleted
    let pool = create_test_pool().await;
    let user_id = Uuid::new_v4();
    create_test_user(&pool, user_id).await;

    let repo = ProjectRepository::new(pool.clone());

    let mut project1 = create_test_project(user_id);
    project1.key = "PROJ1".to_string();
    let mut project2 = create_test_project(user_id);
    project2.key = "PROJ2".to_string();

    repo.create(&project1).await.unwrap();
    repo.create(&project2).await.unwrap();

    // When: Soft deleting project1
    let deleted_at = Utc::now().timestamp();
    repo.delete(project1.id, deleted_at).await.unwrap();

    // Then: find_all returns only project2
    let projects = repo.find_all().await.unwrap();
    assert_that!(projects, len(eq(1)));
    assert_that!(projects[0].id, eq(project2.id));
}

#[tokio::test]
async fn given_empty_database_when_finding_all_then_returns_empty_vec() {
    // Given: An empty database
    let pool = create_test_pool().await;
    let repo = ProjectRepository::new(pool);

    // When: Finding all projects
    let projects = repo.find_all().await.unwrap();

    // Then: Returns empty vector
    assert_that!(projects, is_empty());
}
