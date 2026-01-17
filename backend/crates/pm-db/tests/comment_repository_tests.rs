mod common;

use common::{
    create_test_comment, create_test_pool, create_test_project, create_test_user,
    create_test_work_item,
};

use pm_db::CommentRepository;
use pm_db::WorkItemRepository;

use chrono::Utc;
use googletest::prelude::*;
use uuid::Uuid;

#[tokio::test]
async fn given_valid_comment_when_created_then_can_be_found_by_id() {
    // Given: A test database with a user, project, and work item
    let pool = create_test_pool().await;
    let user_id = Uuid::new_v4();
    create_test_user(&pool, user_id).await;

    let project = create_test_project(user_id);
    // WorkItemRepository is now stateless
    WorkItemRepository::create(&pool, &project).await.unwrap();

    let work_item = create_test_work_item(project.id, user_id);
    WorkItemRepository::create(&pool, &work_item).await.unwrap();

    let repo = CommentRepository::new(pool.clone());
    let comment = create_test_comment(work_item.id, user_id);

    // When: Creating the comment
    repo.create(&comment).await.unwrap();

    // Then: Finding by ID returns the comment
    let result = repo.find_by_id(comment.id).await.unwrap();

    assert_that!(result, some(anything()));
    let found = result.unwrap();
    assert_that!(found.id, eq(comment.id));
    assert_that!(found.content, eq(&comment.content));
    assert_that!(found.work_item_id, eq(comment.work_item_id));
}

#[tokio::test]
async fn given_empty_database_when_finding_nonexistent_id_then_returns_none() {
    // Given: An empty database
    let pool = create_test_pool().await;
    let repo = CommentRepository::new(pool);

    // When: Finding a comment that doesn't exist
    let nonexistent_id = Uuid::new_v4();
    let result = repo.find_by_id(nonexistent_id).await.unwrap();

    // Then: Returns None
    assert_that!(result, none());
}

#[tokio::test]
async fn given_existing_comment_when_updated_then_changes_are_persisted() {
    // Given: A comment exists in the database
    let pool = create_test_pool().await;
    let user_id = Uuid::new_v4();
    create_test_user(&pool, user_id).await;

    let project = create_test_project(user_id);
    // WorkItemRepository is now stateless
    WorkItemRepository::create(&pool, &project).await.unwrap();

    let work_item = create_test_work_item(project.id, user_id);
    WorkItemRepository::create(&pool, &work_item).await.unwrap();

    let repo = CommentRepository::new(pool.clone());
    let mut comment = create_test_comment(work_item.id, user_id);
    repo.create(&comment).await.unwrap();

    // When: Updating the comment's content
    comment.content = "Updated comment content".to_string();
    comment.updated_at = Utc::now();
    repo.update(&comment).await.unwrap();

    // Then: The changes are persisted
    let result = repo.find_by_id(comment.id).await.unwrap();
    let found = result.unwrap();
    assert_that!(found.content, eq("Updated comment content"));
}

#[tokio::test]
async fn given_existing_comment_when_soft_deleted_then_not_found_by_id() {
    // Given: A comment exists in the database
    let pool = create_test_pool().await;
    let user_id = Uuid::new_v4();
    create_test_user(&pool, user_id).await;

    let project = create_test_project(user_id);
    // WorkItemRepository is now stateless
    WorkItemRepository::create(&pool, &project).await.unwrap();

    let work_item = create_test_work_item(project.id, user_id);
    WorkItemRepository::create(&pool, &work_item).await.unwrap();

    let repo = CommentRepository::new(pool.clone());
    let comment = create_test_comment(work_item.id, user_id);
    repo.create(&comment).await.unwrap();

    // When: Soft deleting the comment
    let deleted_at = Utc::now().timestamp();
    repo.delete(comment.id, deleted_at).await.unwrap();

    // Then: find_by_id returns None
    let result = repo.find_by_id(comment.id).await.unwrap();
    assert_that!(result, none());
}

#[tokio::test]
async fn given_multiple_comments_on_work_item_when_finding_by_work_item_then_returns_all() {
    // Given: Multiple comments on the same work item
    let pool = create_test_pool().await;
    let user_id = Uuid::new_v4();
    create_test_user(&pool, user_id).await;

    let project = create_test_project(user_id);
    // WorkItemRepository is now stateless
    WorkItemRepository::create(&pool, &project).await.unwrap();

    let work_item = create_test_work_item(project.id, user_id);
    WorkItemRepository::create(&pool, &work_item).await.unwrap();

    let repo = CommentRepository::new(pool.clone());

    let comment1 = create_test_comment(work_item.id, user_id);
    let comment2 = create_test_comment(work_item.id, user_id);
    let comment3 = create_test_comment(work_item.id, user_id);

    // When: Creating all comments
    repo.create(&comment1).await.unwrap();
    repo.create(&comment2).await.unwrap();
    repo.create(&comment3).await.unwrap();

    // Then: find_by_work_item returns all 3 comments
    let comments = repo.find_by_work_item(work_item.id).await.unwrap();
    assert_that!(comments, len(eq(3)));

    let ids: Vec<Uuid> = comments.iter().map(|c| c.id).collect();
    assert_that!(ids, contains(eq(&comment1.id)));
    assert_that!(ids, contains(eq(&comment2.id)));
    assert_that!(ids, contains(eq(&comment3.id)));
}

#[tokio::test]
async fn given_comments_with_one_deleted_when_finding_by_work_item_then_excludes_deleted() {
    // Given: Multiple comments, one of which is deleted
    let pool = create_test_pool().await;
    let user_id = Uuid::new_v4();
    create_test_user(&pool, user_id).await;

    let project = create_test_project(user_id);
    // WorkItemRepository is now stateless
    WorkItemRepository::create(&pool, &project).await.unwrap();

    let work_item = create_test_work_item(project.id, user_id);
    WorkItemRepository::create(&pool, &work_item).await.unwrap();

    let repo = CommentRepository::new(pool.clone());

    let comment1 = create_test_comment(work_item.id, user_id);
    let comment2 = create_test_comment(work_item.id, user_id);

    repo.create(&comment1).await.unwrap();
    repo.create(&comment2).await.unwrap();

    // When: Soft deleting comment1
    let deleted_at = Utc::now().timestamp();
    repo.delete(comment1.id, deleted_at).await.unwrap();

    // Then: find_by_work_item returns only comment2
    let comments = repo.find_by_work_item(work_item.id).await.unwrap();
    assert_that!(comments, len(eq(1)));
    assert_that!(comments[0].id, eq(comment2.id));
}

#[tokio::test]
async fn given_work_item_with_no_comments_when_finding_by_work_item_then_returns_empty_vec() {
    // Given: A work item with no comments
    let pool = create_test_pool().await;
    let user_id = Uuid::new_v4();
    create_test_user(&pool, user_id).await;

    let project = create_test_project(user_id);
    // WorkItemRepository is now stateless
    WorkItemRepository::create(&pool, &project).await.unwrap();

    let work_item = create_test_work_item(project.id, user_id);
    WorkItemRepository::create(&pool, &work_item).await.unwrap();

    let repo = CommentRepository::new(pool);

    // When: Finding comments by work item
    let comments = repo.find_by_work_item(work_item.id).await.unwrap();

    // Then: Returns empty vector
    assert_that!(comments, is_empty());
}
