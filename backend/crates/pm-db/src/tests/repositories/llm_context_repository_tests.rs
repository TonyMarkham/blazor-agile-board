use crate::LlmContextRepository;

use sqlx::{SqlitePool, migrate};

async fn setup_db() -> SqlitePool {
    let pool = SqlitePool::connect(":memory:")
        .await
        .expect("Failed to create test database");

    migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    pool
}

#[tokio::test]
async fn when_list_all_then_returns_seeded_context() {
    let pool = setup_db().await;

    let entries = LlmContextRepository::list_all(&pool).await.unwrap();

    assert_eq!(entries.len(), 28);
}

#[tokio::test]
async fn when_filtered_by_category_then_only_matches_returned() {
    let pool = setup_db().await;

    let entries = LlmContextRepository::list_filtered(&pool, Some("work_items"), None, None)
        .await
        .unwrap();

    assert!(entries.iter().all(|e| e.category == "work_items"));
}
