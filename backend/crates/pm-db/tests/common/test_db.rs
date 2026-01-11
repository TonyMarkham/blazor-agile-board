use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use uuid::Uuid;

/// Creates an in-memory SQLite pool with migrations run
pub async fn create_test_pool() -> SqlitePool {
    // Create in-memory database connection options
    let options = SqliteConnectOptions::new()
        .filename(":memory:")
        .create_if_missing(true);

    // Create pool
    let pool = SqlitePoolOptions::new()
        .max_connections(1) // In-memory needs single connection
        .connect_with(options)
        .await
        .expect("Failed to create test pool");

    // Enable foreign keys
    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(&pool)
        .await
        .expect("Failed to enable foreign keys");

    // Run migrations
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    pool
}

/// Inserts a stub user for foreign key constraints
pub async fn create_test_user(pool: &SqlitePool, user_id: Uuid) {
    let id = user_id.to_string();
    let email = format!("test-{}@example.com", user_id);

    // Use sqlx::query (not query!) to avoid offline mode issues in tests
    sqlx::query("INSERT INTO users (id, email) VALUES (?, ?)")
        .bind(&id)
        .bind(&email)
        .execute(pool)
        .await
        .expect("Failed to create test user");
}
