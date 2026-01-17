mod common;

use common::{
    create_running_time_entry, create_test_pool, create_test_project, create_test_time_entry,
    create_test_user, create_test_work_item,
};

use pm_db::TimeEntryRepository;
use pm_db::WorkItemRepository;

use chrono::Utc;
use googletest::prelude::*;
use uuid::Uuid;

#[tokio::test]
async fn given_valid_time_entry_when_created_then_can_be_found_by_id() {
    // Given: A test database with a user, project, and work item
    let pool = create_test_pool().await;
    let user_id = Uuid::new_v4();
    create_test_user(&pool, user_id).await;

    let project = create_test_project(user_id);
    // WorkItemRepository is now stateless
    WorkItemRepository::create(&pool, &project).await.unwrap();

    let work_item = create_test_work_item(project.id, user_id);
    WorkItemRepository::create(&pool, &work_item).await.unwrap();

    let repo = TimeEntryRepository::new(pool.clone());
    let time_entry = create_test_time_entry(work_item.id, user_id);

    // When: Creating the time entry
    repo.create(&time_entry).await.unwrap();

    // Then: Finding by ID returns the time entry
    let result = repo.find_by_id(time_entry.id).await.unwrap();

    assert_that!(result, some(anything()));
    let found = result.unwrap();
    assert_that!(found.id, eq(time_entry.id));
    assert_that!(found.work_item_id, eq(time_entry.work_item_id));
    assert_that!(found.user_id, eq(time_entry.user_id));
}

#[tokio::test]
async fn given_empty_database_when_finding_nonexistent_id_then_returns_none() {
    // Given: An empty database
    let pool = create_test_pool().await;
    let repo = TimeEntryRepository::new(pool);

    // When: Finding a time entry that doesn't exist
    let nonexistent_id = Uuid::new_v4();
    let result = repo.find_by_id(nonexistent_id).await.unwrap();

    // Then: Returns None
    assert_that!(result, none());
}

#[tokio::test]
async fn given_running_timer_when_updated_to_stopped_then_changes_are_persisted() {
    // Given: A running time entry exists
    let pool = create_test_pool().await;
    let user_id = Uuid::new_v4();
    create_test_user(&pool, user_id).await;

    let project = create_test_project(user_id);
    // WorkItemRepository is now stateless
    WorkItemRepository::create(&pool, &project).await.unwrap();

    let work_item = create_test_work_item(project.id, user_id);
    WorkItemRepository::create(&pool, &work_item).await.unwrap();

    let repo = TimeEntryRepository::new(pool.clone());
    let mut time_entry = create_running_time_entry(work_item.id, user_id);
    repo.create(&time_entry).await.unwrap();

    // When: Stopping the timer (setting ended_at and duration)
    let now = Utc::now();
    time_entry.ended_at = Some(now);
    time_entry.duration_seconds = Some(3600); // 1 hour
    time_entry.updated_at = now;
    repo.update(&time_entry).await.unwrap();

    // Then: The changes are persisted
    let result = repo.find_by_id(time_entry.id).await.unwrap();
    let found = result.unwrap();
    assert_that!(found.ended_at, some(anything()));
    assert_that!(found.duration_seconds, some(eq(3600)));
}

#[tokio::test]
async fn given_existing_time_entry_when_soft_deleted_then_not_found_by_id() {
    // Given: A time entry exists in the database
    let pool = create_test_pool().await;
    let user_id = Uuid::new_v4();
    create_test_user(&pool, user_id).await;

    let project = create_test_project(user_id);
    // WorkItemRepository is now stateless
    WorkItemRepository::create(&pool, &project).await.unwrap();

    let work_item = create_test_work_item(project.id, user_id);
    WorkItemRepository::create(&pool, &work_item).await.unwrap();

    let repo = TimeEntryRepository::new(pool.clone());
    let time_entry = create_test_time_entry(work_item.id, user_id);
    repo.create(&time_entry).await.unwrap();

    // When: Soft deleting the time entry
    let deleted_at = Utc::now().timestamp();
    repo.delete(time_entry.id, deleted_at).await.unwrap();

    // Then: find_by_id returns None
    let result = repo.find_by_id(time_entry.id).await.unwrap();
    assert_that!(result, none());
}

#[tokio::test]
async fn given_multiple_time_entries_on_work_item_when_finding_by_work_item_then_returns_all() {
    // Given: Multiple time entries on the same work item
    let pool = create_test_pool().await;
    let user_id = Uuid::new_v4();
    create_test_user(&pool, user_id).await;

    let project = create_test_project(user_id);
    // WorkItemRepository is now stateless
    WorkItemRepository::create(&pool, &project).await.unwrap();

    let work_item = create_test_work_item(project.id, user_id);
    WorkItemRepository::create(&pool, &work_item).await.unwrap();

    let repo = TimeEntryRepository::new(pool.clone());

    let entry1 = create_test_time_entry(work_item.id, user_id);
    let entry2 = create_test_time_entry(work_item.id, user_id);
    let entry3 = create_test_time_entry(work_item.id, user_id);

    // When: Creating all time entries
    repo.create(&entry1).await.unwrap();
    repo.create(&entry2).await.unwrap();
    repo.create(&entry3).await.unwrap();

    // Then: find_by_work_item returns all 3 entries
    let entries = repo.find_by_work_item(work_item.id).await.unwrap();
    assert_that!(entries, len(eq(3)));

    let ids: Vec<Uuid> = entries.iter().map(|e| e.id).collect();
    assert_that!(ids, contains(eq(&entry1.id)));
    assert_that!(ids, contains(eq(&entry2.id)));
    assert_that!(ids, contains(eq(&entry3.id)));
}

#[tokio::test]
async fn given_running_and_stopped_timers_when_finding_running_then_returns_only_running() {
    // Given: Multiple time entries, some running and some stopped
    let pool = create_test_pool().await;
    let user_id = Uuid::new_v4();
    create_test_user(&pool, user_id).await;

    let project = create_test_project(user_id);
    // WorkItemRepository is now stateless
    WorkItemRepository::create(&pool, &project).await.unwrap();

    let work_item = create_test_work_item(project.id, user_id);
    WorkItemRepository::create(&pool, &work_item).await.unwrap();

    let repo = TimeEntryRepository::new(pool.clone());

    // Create stopped timer
    let stopped = create_test_time_entry(work_item.id, user_id);
    repo.create(&stopped).await.unwrap();

    // Create running timers
    let running1 = create_running_time_entry(work_item.id, user_id);
    let running2 = create_running_time_entry(work_item.id, user_id);
    repo.create(&running1).await.unwrap();
    repo.create(&running2).await.unwrap();

    // When: Finding running timers for the user
    let running_entries = repo.find_running(user_id).await.unwrap();

    // Then: Returns only the 2 running timers
    assert_that!(running_entries, len(eq(2)));

    let ids: Vec<Uuid> = running_entries.iter().map(|e| e.id).collect();
    assert_that!(ids, contains(eq(&running1.id)));
    assert_that!(ids, contains(eq(&running2.id)));
    assert_that!(ids, not(contains(eq(&stopped.id))));
}

#[tokio::test]
async fn given_no_running_timers_when_finding_running_then_returns_empty_vec() {
    // Given: Only stopped timers exist
    let pool = create_test_pool().await;
    let user_id = Uuid::new_v4();
    create_test_user(&pool, user_id).await;

    let project = create_test_project(user_id);
    // WorkItemRepository is now stateless
    WorkItemRepository::create(&pool, &project).await.unwrap();

    let work_item = create_test_work_item(project.id, user_id);
    WorkItemRepository::create(&pool, &work_item).await.unwrap();

    let repo = TimeEntryRepository::new(pool.clone());

    // Create only stopped timers
    let stopped = create_test_time_entry(work_item.id, user_id);
    repo.create(&stopped).await.unwrap();

    // When: Finding running timers
    let running_entries = repo.find_running(user_id).await.unwrap();

    // Then: Returns empty vector
    assert_that!(running_entries, is_empty());
}

#[tokio::test]
async fn given_time_entries_with_one_deleted_when_finding_by_work_item_then_excludes_deleted() {
    // Given: Multiple time entries, one of which is deleted
    let pool = create_test_pool().await;
    let user_id = Uuid::new_v4();
    create_test_user(&pool, user_id).await;

    let project = create_test_project(user_id);
    // WorkItemRepository is now stateless
    WorkItemRepository::create(&pool, &project).await.unwrap();

    let work_item = create_test_work_item(project.id, user_id);
    WorkItemRepository::create(&pool, &work_item).await.unwrap();

    let repo = TimeEntryRepository::new(pool.clone());

    let entry1 = create_test_time_entry(work_item.id, user_id);
    let entry2 = create_test_time_entry(work_item.id, user_id);

    repo.create(&entry1).await.unwrap();
    repo.create(&entry2).await.unwrap();

    // When: Soft deleting entry1
    let deleted_at = Utc::now().timestamp();
    repo.delete(entry1.id, deleted_at).await.unwrap();

    // Then: find_by_work_item returns only entry2
    let entries = repo.find_by_work_item(work_item.id).await.unwrap();
    assert_that!(entries, len(eq(1)));
    assert_that!(entries[0].id, eq(entry2.id));
}
