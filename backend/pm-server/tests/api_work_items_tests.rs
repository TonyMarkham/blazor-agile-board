mod common;

use crate::common::{create_test_app_state, create_test_project, create_test_user};

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
async fn test_create_work_item_success() {
    let state = create_test_app_state().await;
    let user_id = "00000000-0000-0000-0000-000000000001";
    create_test_user(&state.pool, user_id).await;
    let project_id = create_test_project(&state.pool, user_id).await;

    let app = build_router(state.clone());

    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/work-items")
        .header("Content-Type", "application/json")
        .header("X-User-Id", user_id)
        .body(Body::from(
            json!({
                "project_id": project_id.to_string(),
                "item_type": "task",
                "title": "Test Task",
                "description": "A test task",
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["work_item"]["title"], "Test Task");
    assert_eq!(json["work_item"]["item_type"], "task");
    assert_eq!(json["work_item"]["version"], 1);
    assert_eq!(json["work_item"]["display_key"], "TEST-1");
}

#[tokio::test]
async fn test_get_work_item_not_found() {
    let state = create_test_app_state().await;
    let app = build_router(state.clone());

    let fake_id = Uuid::new_v4();
    let request = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/work-items/{}", fake_id))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["error"]["code"], "NOT_FOUND");
}
