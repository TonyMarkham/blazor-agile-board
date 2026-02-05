//! Integration tests for comment API handlers

mod common;

use crate::common::{create_test_app_state, create_test_project, create_test_user};

use pm_core::{Comment, WorkItem};
use pm_db::{CommentRepository, WorkItemRepository};
use pm_server::routes::build_router;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use serde_json::json;
use tower::ServiceExt;
use uuid::Uuid;

#[tokio::test]
async fn test_create_comment_success() {
    let state = create_test_app_state().await;
    let user_id = "00000000-0000-0000-0000-000000000001";
    create_test_user(&state.pool, user_id).await;
    let project_id = create_test_project(&state.pool, user_id).await;

    // Create work item
    let work_item = WorkItem::new(
        pm_core::WorkItemType::Task,
        "Test".to_string(),
        None,
        None,
        project_id,
        Uuid::parse_str(user_id).unwrap(),
    );
    WorkItemRepository::create(&state.pool, &work_item)
        .await
        .unwrap();

    let app = build_router(state.clone());

    let request = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/work-items/{}/comments", work_item.id))
        .header("Content-Type", "application/json")
        .header("X-User-Id", user_id)
        .body(Body::from(
            json!({
                "content": "This is a test comment"
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["comment"]["content"], "This is a test comment");
    assert_eq!(json["comment"]["work_item_id"], work_item.id.to_string());
}

#[tokio::test]
async fn test_create_comment_work_item_not_found() {
    let state = create_test_app_state().await;
    let user_id = "00000000-0000-0000-0000-000000000001";
    create_test_user(&state.pool, user_id).await;

    let app = build_router(state.clone());

    let fake_id = Uuid::new_v4();
    let request = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/work-items/{}/comments", fake_id))
        .header("Content-Type", "application/json")
        .header("X-User-Id", user_id)
        .body(Body::from(
            json!({
                "content": "This is a test comment"
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["error"]["code"], "NOT_FOUND");
}

#[tokio::test]
async fn test_list_comments_empty() {
    let state = create_test_app_state().await;
    let user_id = "00000000-0000-0000-0000-000000000001";
    create_test_user(&state.pool, user_id).await;
    let project_id = create_test_project(&state.pool, user_id).await;

    // Create work item
    let work_item = WorkItem::new(
        pm_core::WorkItemType::Task,
        "Test".to_string(),
        None,
        None,
        project_id,
        Uuid::parse_str(user_id).unwrap(),
    );
    WorkItemRepository::create(&state.pool, &work_item)
        .await
        .unwrap();

    let app = build_router(state.clone());

    let request = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/work-items/{}/comments", work_item.id))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    let comments = json["comments"].as_array().unwrap();
    assert_eq!(comments.len(), 0);
}

#[tokio::test]
async fn test_list_comments_returns_all() {
    let state = create_test_app_state().await;
    let user_id = "00000000-0000-0000-0000-000000000001";
    create_test_user(&state.pool, user_id).await;
    let project_id = create_test_project(&state.pool, user_id).await;

    // Create work item
    let work_item = WorkItem::new(
        pm_core::WorkItemType::Task,
        "Test".to_string(),
        None,
        None,
        project_id,
        Uuid::parse_str(user_id).unwrap(),
    );
    WorkItemRepository::create(&state.pool, &work_item)
        .await
        .unwrap();

    // Create comment
    let comment = Comment::new(
        work_item.id,
        "Test comment".to_string(),
        Uuid::parse_str(user_id).unwrap(),
    );
    let repo = CommentRepository::new(state.pool.clone());
    repo.create(&comment).await.unwrap();

    let app = build_router(state.clone());

    let request = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/work-items/{}/comments", work_item.id))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    let comments = json["comments"].as_array().unwrap();
    assert_eq!(comments.len(), 1);
    assert_eq!(comments[0]["content"], "Test comment");
}

#[tokio::test]
async fn test_update_comment_success() {
    let state = create_test_app_state().await;
    let user_id = "00000000-0000-0000-0000-000000000001";
    create_test_user(&state.pool, user_id).await;
    let project_id = create_test_project(&state.pool, user_id).await;

    // Create work item
    let work_item = WorkItem::new(
        pm_core::WorkItemType::Task,
        "Test".to_string(),
        None,
        None,
        project_id,
        Uuid::parse_str(user_id).unwrap(),
    );
    WorkItemRepository::create(&state.pool, &work_item)
        .await
        .unwrap();

    // Create comment
    let comment = Comment::new(
        work_item.id,
        "Original content".to_string(),
        Uuid::parse_str(user_id).unwrap(),
    );
    let repo = CommentRepository::new(state.pool.clone());
    repo.create(&comment).await.unwrap();

    let app = build_router(state.clone());

    let request = Request::builder()
        .method("PUT")
        .uri(format!("/api/v1/comments/{}", comment.id))
        .header("Content-Type", "application/json")
        .header("X-User-Id", user_id)
        .body(Body::from(
            json!({
                "content": "Updated content"
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["comment"]["content"], "Updated content");
}

#[tokio::test]
async fn test_update_comment_not_found() {
    let state = create_test_app_state().await;
    let user_id = "00000000-0000-0000-0000-000000000001";
    create_test_user(&state.pool, user_id).await;

    let app = build_router(state.clone());

    let fake_id = Uuid::new_v4();
    let request = Request::builder()
        .method("PUT")
        .uri(format!("/api/v1/comments/{}", fake_id))
        .header("Content-Type", "application/json")
        .header("X-User-Id", user_id)
        .body(Body::from(
            json!({
                "content": "Updated content"
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["error"]["code"], "NOT_FOUND");
}

#[tokio::test]
async fn test_delete_comment_success() {
    let state = create_test_app_state().await;
    let user_id = "00000000-0000-0000-0000-000000000001";
    create_test_user(&state.pool, user_id).await;
    let project_id = create_test_project(&state.pool, user_id).await;

    // Create work item
    let work_item = WorkItem::new(
        pm_core::WorkItemType::Task,
        "Test".to_string(),
        None,
        None,
        project_id,
        Uuid::parse_str(user_id).unwrap(),
    );
    WorkItemRepository::create(&state.pool, &work_item)
        .await
        .unwrap();

    // Create comment
    let comment = Comment::new(
        work_item.id,
        "Test comment".to_string(),
        Uuid::parse_str(user_id).unwrap(),
    );
    let repo = CommentRepository::new(state.pool.clone());
    repo.create(&comment).await.unwrap();

    let app = build_router(state.clone());

    let request = Request::builder()
        .method("DELETE")
        .uri(format!("/api/v1/comments/{}", comment.id))
        .header("X-User-Id", user_id)
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["deleted_id"], comment.id.to_string());

    // Verify comment is soft-deleted (not returned by find)
    let deleted_comment = repo.find_by_id(comment.id).await.unwrap();
    assert!(deleted_comment.is_none());
}
