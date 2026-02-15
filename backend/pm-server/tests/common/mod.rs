#![allow(dead_code)]

//! Test infrastructure for pm-server API tests

use pm_auth::RateLimiterFactory;
use pm_config::ApiConfig;
use pm_ws::AppState;
use pm_ws::{
    CircuitBreaker, CircuitBreakerConfig, ConnectionConfig, ConnectionLimits, ConnectionRegistry,
    Metrics, ShutdownCoordinator,
};

use std::sync::Arc;

use sqlx::SqlitePool;

/// Create a test pool with in-memory SQLite
pub async fn create_test_pool() -> SqlitePool {
    let pool = SqlitePool::connect(":memory:")
        .await
        .expect("Failed to create test database");

    sqlx::migrate!("../crates/pm-db/migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    pool
}

/// Create AppState for testing
pub async fn create_test_app_state() -> AppState {
    let pool = create_test_pool().await;
    let circuit_breaker = Arc::new(CircuitBreaker::new(CircuitBreakerConfig::default()));
    let limits = ConnectionLimits { max_total: 10000 };
    let registry = ConnectionRegistry::new(limits);
    let shutdown = ShutdownCoordinator::new();

    AppState {
        pool,
        circuit_breaker,
        jwt_validator: None,
        desktop_user_id: "test-user".to_string(),
        rate_limiter_factory: RateLimiterFactory::default(),
        registry,
        metrics: Metrics::new(),
        shutdown,
        config: ConnectionConfig::default(),
        api_config: ApiConfig::default(),
        validation: pm_config::ValidationConfig::default(),
    }
}

/// Create a test user
pub async fn create_test_user(pool: &SqlitePool, user_id: &str) {
    sqlx::query("INSERT INTO users (id, email, created_at) VALUES (?, ?, ?)")
        .bind(user_id)
        .bind(format!("{}@test.local", user_id))
        .bind(chrono::Utc::now().timestamp())
        .execute(pool)
        .await
        .expect("Failed to create test user");
}

/// Create a test project
pub async fn create_test_project(pool: &SqlitePool, user_id: &str) -> uuid::Uuid {
    let project_id = uuid::Uuid::new_v4();
    let now = chrono::Utc::now().timestamp();

    sqlx::query(
        r#"
                INSERT INTO pm_projects(id, key, title, description, status, created_at, updated_at, created_by, updated_by, version)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#
    )
        .bind(project_id.to_string())
        .bind("TEST")
        .bind("Test Project")
        .bind("A test project")
        .bind("active")
        .bind(now)
        .bind(now)
        .bind(user_id)
        .bind(user_id)
        .bind(1)
        .execute(pool)
        .await
        .expect("Failed to create test project");

    project_id
}

/// Create a test work item
pub async fn create_test_work_item(
    pool: &SqlitePool,
    project_id: uuid::Uuid,
    item_number: i32,
    user_id: &str,
) -> uuid::Uuid {
    let work_item_id = uuid::Uuid::new_v4();
    let now = chrono::Utc::now().timestamp();

    sqlx::query(
        r#"
          INSERT INTO pm_work_items (
              id, item_type, item_number, project_id, parent_id,
              title, description, status, priority, position,
              created_at, updated_at, created_by, updated_by, version
          )
          VALUES (?, ?, ?, ?, NULL, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
          "#,
    )
    .bind(work_item_id.to_string())
    .bind("task") // item_type
    .bind(item_number)
    .bind(project_id.to_string())
    .bind(format!("Test Work Item {}", item_number)) // title
    .bind("A test work item") // description
    .bind("todo") // status
    .bind("medium") // priority
    .bind(1000) // position
    .bind(now)
    .bind(now)
    .bind(user_id)
    .bind(user_id)
    .bind(1) // version
    .execute(pool)
    .await
    .expect("Failed to create test work item");

    work_item_id
}
