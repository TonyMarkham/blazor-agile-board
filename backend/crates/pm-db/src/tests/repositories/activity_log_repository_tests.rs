use crate::ActivityLogRepository;

use chrono::Utc;
use pm_core::ActivityLog;
use sqlx::{SqlitePool, migrate};
use uuid::Uuid;

async fn setup_db() -> SqlitePool {
    let pool = SqlitePool::connect(":memory:")
        .await
        .expect("Failed to create test database");

    migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    // Seed minimal user (FK)
    sqlx::query("INSERT INTO users (id, email, name, created_at) VALUES (?, 'test@example.com', 'Test User', ?)")
        .bind(Uuid::new_v4().to_string())
        .bind(Utc::now().timestamp())
        .execute(&pool)
        .await
        .expect("Failed to seed user");

    pool
}

#[tokio::test]
async fn given_multiple_entries_when_paginated_then_returns_page_and_total() {
    let pool = setup_db().await;
    let user_id = Uuid::new_v4();

    // Seed user for FK
    sqlx::query("INSERT INTO users (id, email, name, created_at) VALUES (?, 'user2@example.com', 'User 2', ?)")
        .bind(user_id.to_string())
        .bind(Utc::now().timestamp())
        .execute(&pool)
        .await
        .expect("Failed to seed user");

    let entity_id = Uuid::new_v4();

    // Insert 5 entries
    for _ in 0..5 {
        let log = ActivityLog::created("work_item", entity_id, user_id);
        ActivityLogRepository::create(&pool, &log).await.unwrap();
    }

    let (page, total) =
        ActivityLogRepository::find_by_entity_paginated(&pool, "work_item", entity_id, 2, 0)
            .await
            .unwrap();

    assert_eq!(total, 5);
    assert_eq!(page.len(), 2);
}
