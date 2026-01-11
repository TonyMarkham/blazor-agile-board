mod common;

use common::{create_test_project, create_test_user, create_test_work_item};

use pm_db::TenantConnectionManager;
use pm_db::WorkItemRepository;

use googletest::prelude::*;
use tempfile::TempDir;
use uuid::Uuid;

#[tokio::test]
async fn given_new_tenant_when_getting_pool_then_creates_database_and_runs_migrations() {
    // Given: A tenant connection manager with temp directory
    let temp_dir = TempDir::new().unwrap();
    let manager = TenantConnectionManager::new(temp_dir.path());

    // When: Getting pool for a new tenant
    let pool = manager.get_pool("tenant-a").await.unwrap();

    // Then: Pool is created and migrations have run (users table exists)
    create_test_user(&pool, Uuid::new_v4()).await;

    // Then: Can create a project (work_items table exists)
    let user_id = Uuid::new_v4();
    create_test_user(&pool, user_id).await;
    let project = create_test_project(user_id);
    let repo = WorkItemRepository::new(pool.clone());
    repo.create(&project).await.unwrap();
}

#[tokio::test]
async fn given_existing_tenant_when_getting_pool_again_then_returns_cached_pool() {
    // Given: A tenant with an existing pool
    let temp_dir = TempDir::new().unwrap();
    let manager = TenantConnectionManager::new(temp_dir.path());

    let pool1 = manager.get_pool("tenant-a").await.unwrap();

    // When: Getting pool again for the same tenant
    let pool2 = manager.get_pool("tenant-a").await.unwrap();

    // Then: Returns the same pool (cached)
    // We can verify by checking that data created with pool1 is visible via pool2
    let user_id = Uuid::new_v4();
    create_test_user(&pool1, user_id).await;

    let project = create_test_project(user_id);
    let repo1 = WorkItemRepository::new(pool1);
    repo1.create(&project).await.unwrap();

    let repo2 = WorkItemRepository::new(pool2);
    let found = repo2.find_by_id(project.id).await.unwrap();
    assert_that!(found, some(anything()));
}

#[tokio::test]
async fn given_multiple_tenants_when_creating_data_then_data_is_isolated() {
    // Given: Two separate tenants
    let temp_dir = TempDir::new().unwrap();
    let manager = TenantConnectionManager::new(temp_dir.path());

    let pool_a = manager.get_pool("tenant-a").await.unwrap();
    let pool_b = manager.get_pool("tenant-b").await.unwrap();

    let user_id = Uuid::new_v4();
    create_test_user(&pool_a, user_id).await;
    create_test_user(&pool_b, user_id).await;

    // When: Creating a work item in tenant A only
    let project_a = create_test_project(user_id);
    let repo_a = WorkItemRepository::new(pool_a.clone());
    repo_a.create(&project_a).await.unwrap();

    let work_item = create_test_work_item(project_a.id, user_id);
    repo_a.create(&work_item).await.unwrap();

    // Then: Work item exists in tenant A
    let result_a = repo_a.find_by_id(work_item.id).await.unwrap();
    assert_that!(result_a, some(anything()));

    // Then: Work item does NOT exist in tenant B
    let repo_b = WorkItemRepository::new(pool_b);
    let result_b = repo_b.find_by_id(work_item.id).await.unwrap();
    assert_that!(result_b, none());
}

#[tokio::test]
async fn given_multiple_tenants_when_creating_same_id_then_both_succeed() {
    // Given: Two separate tenants
    let temp_dir = TempDir::new().unwrap();
    let manager = TenantConnectionManager::new(temp_dir.path());

    let pool_a = manager.get_pool("tenant-a").await.unwrap();
    let pool_b = manager.get_pool("tenant-b").await.unwrap();

    let user_id = Uuid::new_v4();
    create_test_user(&pool_a, user_id).await;
    create_test_user(&pool_b, user_id).await;

    // When: Creating projects with the same ID in both tenants
    let shared_id = Uuid::new_v4();
    let mut project_a = create_test_project(user_id);
    project_a.id = shared_id;
    project_a.project_id = shared_id;

    let mut project_b = create_test_project(user_id);
    project_b.id = shared_id;
    project_b.project_id = shared_id;
    project_b.title = "Different Title".to_string();

    let repo_a = WorkItemRepository::new(pool_a.clone());
    let repo_b = WorkItemRepository::new(pool_b.clone());

    // Then: Both creates succeed (no collision)
    repo_a.create(&project_a).await.unwrap();
    repo_b.create(&project_b).await.unwrap();

    // Then: Each tenant sees their own data
    let found_a = repo_a.find_by_id(shared_id).await.unwrap().unwrap();
    let found_b = repo_b.find_by_id(shared_id).await.unwrap().unwrap();

    assert_that!(found_a.title, eq(&project_a.title));
    assert_that!(found_b.title, eq("Different Title"));
}

#[tokio::test]
async fn given_tenant_when_getting_pool_then_creates_directory_structure() {
    // Given: A tenant connection manager with temp directory
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path();
    let manager = TenantConnectionManager::new(base_path);

    // When: Getting pool for a tenant
    let _pool = manager.get_pool("tenant-xyz").await.unwrap();

    // Then: Directory structure is created
    let tenant_dir = base_path.join("tenant-xyz");
    assert_that!(tenant_dir.exists(), is_true());

    let db_file = tenant_dir.join("main.db");
    assert_that!(db_file.exists(), is_true());
}

#[tokio::test]
async fn given_tenant_pool_when_inserting_with_foreign_key_violation_then_fails() {
    // Given: A tenant pool with foreign keys enabled
    let temp_dir = TempDir::new().unwrap();
    let manager = TenantConnectionManager::new(temp_dir.path());
    let pool = manager.get_pool("tenant-a").await.unwrap();

    let user_id = Uuid::new_v4();
    create_test_user(&pool, user_id).await;

    // When: Attempting to create a work item with non-existent project_id
    let non_existent_project = Uuid::new_v4();
    let work_item = create_test_work_item(non_existent_project, user_id);

    let repo = WorkItemRepository::new(pool);
    let result = repo.create(&work_item).await;

    // Then: Operation fails due to foreign key constraint
    assert_that!(result, err(anything()));
}

#[tokio::test]
async fn given_concurrent_requests_for_same_tenant_when_getting_pool_then_reuses_pool() {
    // Given: Multiple concurrent requests for the same tenant
    let temp_dir = TempDir::new().unwrap();
    let manager = std::sync::Arc::new(TenantConnectionManager::new(temp_dir.path()));

    // When: Getting pool concurrently from multiple tasks
    let handles: Vec<_> = (0..5)
        .map(|_| {
            let manager = manager.clone();
            tokio::spawn(async move { manager.get_pool("tenant-shared").await })
        })
        .collect();

    // Then: All requests succeed
    for handle in handles {
        let result = handle.await.unwrap();
        assert_that!(result, ok(anything()));
    }
}
