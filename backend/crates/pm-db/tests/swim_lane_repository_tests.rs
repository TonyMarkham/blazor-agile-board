mod common;

use common::{
    create_default_swim_lane, create_test_pool, create_test_project, create_test_swim_lane,
    create_test_swim_lane_with_status, create_test_user,
};

use pm_db::SwimLaneRepository;
use pm_db::WorkItemRepository;

use chrono::Utc;
use googletest::prelude::*;
use uuid::Uuid;

#[tokio::test]
async fn given_valid_swim_lane_when_created_then_can_be_found_by_id() {
    // Given: A test database with a user and project
    let pool = create_test_pool().await;
    let user_id = Uuid::new_v4();
    create_test_user(&pool, user_id).await;

    let project = create_test_project(user_id);
    // WorkItemRepository is now stateless
    WorkItemRepository::create(&pool, &project).await.unwrap();

    let repo = SwimLaneRepository::new(pool.clone());
    let swim_lane = create_test_swim_lane(project.id);

    // When: Creating the swim lane
    repo.create(&swim_lane).await.unwrap();

    // Then: Finding by ID returns the swim lane
    let result = repo.find_by_id(swim_lane.id).await.unwrap();

    assert_that!(result, some(anything()));
    let found = result.unwrap();
    assert_that!(found.id, eq(swim_lane.id));
    assert_that!(found.name, eq(&swim_lane.name));
    assert_that!(found.status_value, eq(&swim_lane.status_value));
}

#[tokio::test]
async fn given_empty_database_when_finding_nonexistent_id_then_returns_none() {
    // Given: An empty database
    let pool = create_test_pool().await;
    let repo = SwimLaneRepository::new(pool);

    // When: Finding a swim lane that doesn't exist
    let nonexistent_id = Uuid::new_v4();
    let result = repo.find_by_id(nonexistent_id).await.unwrap();

    // Then: Returns None
    assert_that!(result, none());
}

#[tokio::test]
async fn given_existing_swim_lane_when_updated_then_changes_are_persisted() {
    // Given: A swim lane exists in the database
    let pool = create_test_pool().await;
    let user_id = Uuid::new_v4();
    create_test_user(&pool, user_id).await;

    let project = create_test_project(user_id);
    // WorkItemRepository is now stateless
    WorkItemRepository::create(&pool, &project).await.unwrap();

    let repo = SwimLaneRepository::new(pool.clone());
    let mut swim_lane = create_test_swim_lane(project.id);
    repo.create(&swim_lane).await.unwrap();

    // When: Updating the swim lane's name and status
    swim_lane.name = "Updated Lane".to_string();
    swim_lane.status_value = "done".to_string();
    swim_lane.updated_at = Utc::now();
    repo.update(&swim_lane).await.unwrap();

    // Then: The changes are persisted
    let result = repo.find_by_id(swim_lane.id).await.unwrap();
    let found = result.unwrap();
    assert_that!(found.name, eq("Updated Lane"));
    assert_that!(found.status_value, eq("done"));
}

#[tokio::test]
async fn given_existing_swim_lane_when_soft_deleted_then_not_found_by_id() {
    // Given: A swim lane exists in the database
    let pool = create_test_pool().await;
    let user_id = Uuid::new_v4();
    create_test_user(&pool, user_id).await;

    let project = create_test_project(user_id);
    // WorkItemRepository is now stateless
    WorkItemRepository::create(&pool, &project).await.unwrap();

    let repo = SwimLaneRepository::new(pool.clone());
    let swim_lane = create_test_swim_lane(project.id);
    repo.create(&swim_lane).await.unwrap();

    // When: Soft deleting the swim lane
    let deleted_at = Utc::now().timestamp();
    repo.delete(swim_lane.id, deleted_at).await.unwrap();

    // Then: find_by_id returns None
    let result = repo.find_by_id(swim_lane.id).await.unwrap();
    assert_that!(result, none());
}

#[tokio::test]
async fn given_default_swim_lane_when_attempting_delete_then_not_deleted() {
    // Given: A default swim lane exists
    let pool = create_test_pool().await;
    let user_id = Uuid::new_v4();
    create_test_user(&pool, user_id).await;

    let project = create_test_project(user_id);
    // WorkItemRepository is now stateless
    WorkItemRepository::create(&pool, &project).await.unwrap();

    let repo = SwimLaneRepository::new(pool.clone());
    let default_lane = create_default_swim_lane(project.id);
    repo.create(&default_lane).await.unwrap();

    // When: Attempting to delete the default lane
    let deleted_at = Utc::now().timestamp();
    repo.delete(default_lane.id, deleted_at).await.unwrap();

    // Then: The default lane is still found (not deleted)
    let result = repo.find_by_id(default_lane.id).await.unwrap();
    assert_that!(result, some(anything()));
}

#[tokio::test]
async fn given_multiple_swim_lanes_in_project_when_finding_by_project_then_returns_all_ordered_by_position()
 {
    // Given: Multiple swim lanes with different positions
    let pool = create_test_pool().await;
    let user_id = Uuid::new_v4();
    create_test_user(&pool, user_id).await;

    let project = create_test_project(user_id);
    // WorkItemRepository is now stateless
    WorkItemRepository::create(&pool, &project).await.unwrap();

    let repo = SwimLaneRepository::new(pool.clone());

    let mut lane1 = create_test_swim_lane_with_status(project.id, "todo");
    lane1.position = 2;
    let mut lane2 = create_test_swim_lane_with_status(project.id, "in-progress");
    lane2.position = 0;
    let mut lane3 = create_test_swim_lane_with_status(project.id, "done");
    lane3.position = 1;

    // When: Creating lanes in random order
    repo.create(&lane1).await.unwrap();
    repo.create(&lane2).await.unwrap();
    repo.create(&lane3).await.unwrap();

    // Then: find_by_project returns lanes ordered by position ASC
    let lanes = repo.find_by_project(project.id).await.unwrap();
    assert_that!(lanes, len(eq(3)));
    assert_that!(lanes[0].id, eq(lane2.id)); // position 0
    assert_that!(lanes[1].id, eq(lane3.id)); // position 1
    assert_that!(lanes[2].id, eq(lane1.id)); // position 2
}

#[tokio::test]
async fn given_swim_lanes_with_one_deleted_when_finding_by_project_then_excludes_deleted() {
    // Given: Multiple swim lanes, one of which is deleted
    let pool = create_test_pool().await;
    let user_id = Uuid::new_v4();
    create_test_user(&pool, user_id).await;

    let project = create_test_project(user_id);
    // WorkItemRepository is now stateless
    WorkItemRepository::create(&pool, &project).await.unwrap();

    let repo = SwimLaneRepository::new(pool.clone());

    let lane1 = create_test_swim_lane_with_status(project.id, "todo");
    let lane2 = create_test_swim_lane_with_status(project.id, "in-progress");

    repo.create(&lane1).await.unwrap();
    repo.create(&lane2).await.unwrap();

    // When: Soft deleting lane1
    let deleted_at = Utc::now().timestamp();
    repo.delete(lane1.id, deleted_at).await.unwrap();

    // Then: find_by_project returns only lane2
    let lanes = repo.find_by_project(project.id).await.unwrap();
    assert_that!(lanes, len(eq(1)));
    assert_that!(lanes[0].id, eq(lane2.id));
}

#[tokio::test]
async fn given_empty_project_when_finding_by_project_then_returns_empty_vec() {
    // Given: A project with no swim lanes
    let pool = create_test_pool().await;
    let user_id = Uuid::new_v4();
    create_test_user(&pool, user_id).await;

    let project = create_test_project(user_id);
    // WorkItemRepository is now stateless
    WorkItemRepository::create(&pool, &project).await.unwrap();

    let repo = SwimLaneRepository::new(pool);

    // When: Finding swim lanes by project
    let lanes = repo.find_by_project(project.id).await.unwrap();

    // Then: Returns empty vector
    assert_that!(lanes, is_empty());
}
