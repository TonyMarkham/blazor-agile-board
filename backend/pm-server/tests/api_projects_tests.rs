//! Integration tests for project API handlers
mod common;

use crate::common::{create_test_app_state, create_test_project, create_test_user};

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use tower::ServiceExt;
use uuid::Uuid;

use pm_server::routes::build_router;

#[tokio::test]
async fn test_list_projects_empty() {
    let state = create_test_app_state().await;
    let app = build_router(state.clone());

    let request = Request::builder()
        .method("GET")
        .uri("/api/v1/projects")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    let projects = json["projects"].as_array().unwrap();
    assert_eq!(projects.len(), 0);
}

#[tokio::test]
async fn test_list_projects_returns_all() {
    let state = create_test_app_state().await;
    let user_id = "00000000-0000-0000-0000-000000000001";
    create_test_user(&state.pool, user_id).await;
    let _project_id = create_test_project(&state.pool, user_id).await;

    let app = build_router(state.clone());

    let request = Request::builder()
        .method("GET")
        .uri("/api/v1/projects")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    let projects = json["projects"].as_array().unwrap();
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0]["key"], "TEST");
    assert_eq!(projects[0]["title"], "Test Project");
}

#[tokio::test]
async fn test_get_project_success() {
    let state = create_test_app_state().await;
    let user_id = "00000000-0000-0000-0000-000000000001";
    create_test_user(&state.pool, user_id).await;
    let project_id = create_test_project(&state.pool, user_id).await;

    let app = build_router(state.clone());

    let request = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/projects/{}", project_id))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["project"]["id"], project_id.to_string());
    assert_eq!(json["project"]["key"], "TEST");
    assert_eq!(json["project"]["title"], "Test Project");
}

#[tokio::test]
async fn test_get_project_not_found() {
    let state = create_test_app_state().await;
    let app = build_router(state.clone());

    let fake_id = Uuid::new_v4();
    let request = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/projects/{}", fake_id))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["error"]["code"], "NOT_FOUND");
    assert!(
        json["error"]["message"]
            .as_str()
            .unwrap()
            .contains("not found")
    );
}

#[tokio::test]
async fn test_get_project_invalid_uuid() {
    let state = create_test_app_state().await;
    let app = build_router(state.clone());

    let request = Request::builder()
        .method("GET")
        .uri("/api/v1/projects/not-a-uuid")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["error"]["code"], "VALIDATION_ERROR");
}
