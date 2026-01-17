mod common;

use common::{
    create_field_change_log, create_field_change_log_at, create_test_activity_log,
    create_test_activity_log_at, create_test_pool, create_test_user,
};

use pm_db::ActivityLogRepository;

use googletest::prelude::*;
use uuid::Uuid;

#[tokio::test]
async fn given_valid_activity_log_when_created_then_can_be_found_by_entity() {
    // Given: A test database with a user
    let pool = create_test_pool().await;
    let user_id = Uuid::new_v4();
    create_test_user(&pool, user_id).await;

    let entity_id = Uuid::new_v4();
    let log = create_test_activity_log_at("work_item", entity_id, user_id, 0);

    // When: Creating the activity log
    ActivityLogRepository::create(&pool, &log).await.unwrap();

    // Then: Finding by entity returns the log
    let logs = ActivityLogRepository::find_by_entity(&pool, "work_item", entity_id)
        .await
        .unwrap();

    assert_that!(logs, len(eq(1)));
    assert_that!(logs[0].id, eq(log.id));
    assert_that!(logs[0].entity_type, eq(&log.entity_type));
    assert_that!(logs[0].action, eq(&log.action));
}

#[tokio::test]
async fn given_multiple_logs_for_entity_when_finding_by_entity_then_returns_all_ordered_by_timestamp()
 {
    // Given: Multiple activity logs for the same entity
    let pool = create_test_pool().await;
    let user_id = Uuid::new_v4();
    create_test_user(&pool, user_id).await;

    let entity_id = Uuid::new_v4();

    // Create logs with explicit timestamp offsets (seconds ago)
    let log1 = create_test_activity_log_at("work_item", entity_id, user_id, -2);
    let log2 = create_field_change_log_at(
        "work_item",
        entity_id,
        "status",
        "todo",
        "in_progress",
        user_id,
        -1,
    );
    let log3 = create_field_change_log_at(
        "work_item",
        entity_id,
        "status",
        "in_progress",
        "done",
        user_id,
        0,
    );

    // When: Creating all logs
    ActivityLogRepository::create(&pool, &log1).await.unwrap();
    ActivityLogRepository::create(&pool, &log2).await.unwrap();
    ActivityLogRepository::create(&pool, &log3).await.unwrap();

    // Then: find_by_entity returns all 3 logs, newest first
    let logs = ActivityLogRepository::find_by_entity(&pool, "work_item", entity_id)
        .await
        .unwrap();
    assert_that!(logs, len(eq(3)));

    // Ordered by timestamp DESC (newest first)
    assert_that!(logs[0].id, eq(log3.id));
    assert_that!(logs[1].id, eq(log2.id));
    assert_that!(logs[2].id, eq(log1.id));
}

#[tokio::test]
async fn given_logs_for_different_entities_when_finding_by_entity_then_returns_only_matching() {
    // Given: Activity logs for different entities
    let pool = create_test_pool().await;
    let user_id = Uuid::new_v4();
    create_test_user(&pool, user_id).await;

    let entity_a = Uuid::new_v4();
    let entity_b = Uuid::new_v4();

    let log_a = create_test_activity_log("work_item", entity_a, user_id);
    let log_b = create_test_activity_log("work_item", entity_b, user_id);

    // When: Creating logs for both entities
    ActivityLogRepository::create(&pool, &log_a).await.unwrap();
    ActivityLogRepository::create(&pool, &log_b).await.unwrap();

    // Then: find_by_entity returns only entity A's logs
    let logs = ActivityLogRepository::find_by_entity(&pool, "work_item", entity_a)
        .await
        .unwrap();
    assert_that!(logs, len(eq(1)));
    assert_that!(logs[0].id, eq(log_a.id));
}

#[tokio::test]
async fn given_logs_for_different_entity_types_when_finding_by_entity_then_returns_only_matching_type()
 {
    // Given: Activity logs for different entity types with same entity_id
    let pool = create_test_pool().await;
    let user_id = Uuid::new_v4();
    create_test_user(&pool, user_id).await;

    let entity_id = Uuid::new_v4();

    let log_work_item = create_test_activity_log("work_item", entity_id, user_id);
    let log_sprint = create_test_activity_log("sprint", entity_id, user_id);

    // When: Creating logs for different entity types
    ActivityLogRepository::create(&pool, &log_work_item)
        .await
        .unwrap();
    ActivityLogRepository::create(&pool, &log_sprint)
        .await
        .unwrap();

    // Then: find_by_entity filters by both entity_type and entity_id
    let logs = ActivityLogRepository::find_by_entity(&pool, "work_item", entity_id)
        .await
        .unwrap();
    assert_that!(logs, len(eq(1)));
    assert_that!(logs[0].entity_type, eq("work_item"));
}

#[tokio::test]
async fn given_entity_with_no_logs_when_finding_by_entity_then_returns_empty_vec() {
    // Given: An entity with no activity logs
    let pool = create_test_pool().await;
    let entity_id = Uuid::new_v4();

    // When: Finding logs for the entity
    let logs = ActivityLogRepository::find_by_entity(&pool, "work_item", entity_id)
        .await
        .unwrap();

    // Then: Returns empty vector
    assert_that!(logs, is_empty());
}

#[tokio::test]
async fn given_multiple_user_activities_when_finding_by_user_then_returns_limited_results() {
    // Given: Multiple activity logs for a user
    let pool = create_test_pool().await;
    let user_id = Uuid::new_v4();
    create_test_user(&pool, user_id).await;

    // Create 5 logs for the user
    for _i in 0..5 {
        let entity_id = Uuid::new_v4();
        let log = create_test_activity_log("work_item", entity_id, user_id);
        ActivityLogRepository::create(&pool, &log).await.unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }

    // When: Finding user's activities with limit of 3
    let logs = ActivityLogRepository::find_by_user(&pool, user_id, 3)
        .await
        .unwrap();

    // Then: Returns only 3 most recent logs
    assert_that!(logs, len(eq(3)));
}

#[tokio::test]
async fn given_logs_from_multiple_users_when_finding_by_user_then_returns_only_user_logs() {
    // Given: Activity logs from different users
    let pool = create_test_pool().await;
    let user_a = Uuid::new_v4();
    let user_b = Uuid::new_v4();
    create_test_user(&pool, user_a).await;
    create_test_user(&pool, user_b).await;

    let entity_id = Uuid::new_v4();
    let log_a = create_test_activity_log("work_item", entity_id, user_a);
    let log_b = create_test_activity_log("work_item", entity_id, user_b);

    // When: Creating logs from both users
    ActivityLogRepository::create(&pool, &log_a).await.unwrap();
    ActivityLogRepository::create(&pool, &log_b).await.unwrap();

    // Then: find_by_user returns only user A's logs
    let logs = ActivityLogRepository::find_by_user(&pool, user_a, 10)
        .await
        .unwrap();
    assert_that!(logs, len(eq(1)));
    assert_that!(logs[0].user_id, eq(user_a));
}

#[tokio::test]
async fn given_field_change_log_when_created_then_captures_old_and_new_values() {
    // Given: A field change activity log
    let pool = create_test_pool().await;
    let user_id = Uuid::new_v4();
    create_test_user(&pool, user_id).await;

    let entity_id = Uuid::new_v4();
    let log = create_field_change_log(
        "work_item",
        entity_id,
        "title",
        "Old Title",
        "New Title",
        user_id,
    );

    // When: Creating the field change log
    ActivityLogRepository::create(&pool, &log).await.unwrap();

    // Then: The field change details are persisted
    let logs = ActivityLogRepository::find_by_entity(&pool, "work_item", entity_id)
        .await
        .unwrap();
    assert_that!(logs, len(eq(1)));
    assert_that!(logs[0].field_name, some(eq("title")));
    assert_that!(logs[0].old_value, some(eq("Old Title")));
    assert_that!(logs[0].new_value, some(eq("New Title")));
}
