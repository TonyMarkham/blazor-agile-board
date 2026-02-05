//! Integration tests for the CLI client using wiremock mock server

use pm_cli::Client;

use serde_json::json;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{body_string_contains, header, method, path, query_param},
};

#[tokio::test]
async fn test_list_projects_success() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/projects"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "projects": [
                {
                    "id": "00000000-0000-0000-0000-000000000001",
                    "key": "TEST",
                    "title": "Test Project",
                    "description": "A test project",
                    "created_at": 1704067200,
                    "updated_at": 1704067200
                }
            ]
        })))
        .mount(&mock_server)
        .await;

    let client = Client::new(&mock_server.uri(), None);
    let result = client.list_projects().await.unwrap();

    assert!(result["projects"].is_array());
    let projects = result["projects"].as_array().unwrap();
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0]["key"], "TEST");
    assert_eq!(projects[0]["title"], "Test Project");
}

#[tokio::test]
async fn test_get_project_not_found() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(
            "/api/v1/projects/00000000-0000-0000-0000-000000000001",
        ))
        .respond_with(ResponseTemplate::new(404).set_body_json(json!({
            "error": {
                "code": "NOT_FOUND",
                "message": "Project not found"
            }
        })))
        .mount(&mock_server)
        .await;

    let client = Client::new(&mock_server.uri(), None);
    let result = client
        .get_project("00000000-0000-0000-0000-000000000001")
        .await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("NOT_FOUND"));
}

#[tokio::test]
async fn test_create_work_item_success() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/api/v1/work-items"))
        .and(body_string_contains("Test Task"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "work_item": {
                "id": "00000000-0000-0000-0000-000000000002",
                "display_key": "TEST-1",
                "item_type": "task",
                "title": "Test Task",
                "description": null,
                "status": "backlog",
                "priority": "medium",
                "parent_id": null,
                "project_id": "00000000-0000-0000-0000-000000000001",
                "assignee_id": null,
                "sprint_id": null,
                "story_points": null,
                "item_number": 1,
                "position": 1,
                "version": 1,
                "created_at": 1704067200,
                "updated_at": 1704067200,
                "created_by": "00000000-0000-0000-0000-000000000001",
                "updated_by": "00000000-0000-0000-0000-000000000001"
            }
        })))
        .mount(&mock_server)
        .await;

    let client = Client::new(&mock_server.uri(), None);
    let result = client
        .create_work_item(
            "00000000-0000-0000-0000-000000000001",
            "task",
            "Test Task",
            None,
            None,
            None,
            None,
        )
        .await
        .unwrap();

    assert_eq!(result["work_item"]["title"], "Test Task");
    assert_eq!(result["work_item"]["item_type"], "task");
    assert_eq!(result["work_item"]["version"], 1);
}

#[tokio::test]
async fn test_update_work_item_conflict() {
    let mock_server = MockServer::start().await;

    Mock::given(method("PUT"))
        .and(path(
            "/api/v1/work-items/00000000-0000-0000-0000-000000000001",
        ))
        .respond_with(ResponseTemplate::new(409).set_body_json(json!({
            "error": {
                "code": "CONFLICT",
                "message": "Version mismatch (current version: 5)"
            }
        })))
        .mount(&mock_server)
        .await;

    let client = Client::new(&mock_server.uri(), None);
    let result = client
        .update_work_item(
            "00000000-0000-0000-0000-000000000001",
            Some("Updated"),
            None,
            None,
            None,
            None,
            None,
            None,
            999,
        )
        .await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("CONFLICT"));
}

#[tokio::test]
async fn test_list_work_items_with_filters() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(
            "/api/v1/projects/00000000-0000-0000-0000-000000000001/work-items",
        ))
        .and(query_param("type", "task"))
        .and(query_param("status", "in_progress"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "work_items": []
        })))
        .mount(&mock_server)
        .await;

    let client = Client::new(&mock_server.uri(), None);
    let result = client
        .list_work_items(
            "00000000-0000-0000-0000-000000000001",
            Some("task"),
            Some("in_progress"),
        )
        .await
        .unwrap();

    assert!(result["work_items"].is_array());
}

#[tokio::test]
async fn test_user_id_header_sent() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/v1/projects"))
        .and(header("X-User-Id", "user-123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"projects": []})))
        .mount(&mock_server)
        .await;

    let client = Client::new(&mock_server.uri(), Some("user-123"));
    let result = client.list_projects().await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_delete_work_item() {
    let mock_server = MockServer::start().await;

    Mock::given(method("DELETE"))
        .and(path(
            "/api/v1/work-items/00000000-0000-0000-0000-000000000001",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "deleted_id": "00000000-0000-0000-0000-000000000001"
        })))
        .mount(&mock_server)
        .await;

    let client = Client::new(&mock_server.uri(), None);
    let result = client
        .delete_work_item("00000000-0000-0000-0000-000000000001")
        .await
        .unwrap();

    assert_eq!(result["deleted_id"], "00000000-0000-0000-0000-000000000001");
}

#[tokio::test]
async fn test_create_comment() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path(
            "/api/v1/work-items/00000000-0000-0000-0000-000000000001/comments",
        ))
        .and(body_string_contains("Test comment"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "comment": {
                "id": "00000000-0000-0000-0000-000000000002",
                "work_item_id": "00000000-0000-0000-0000-000000000001",
                "content": "Test comment",
                "created_at": 1704067200,
                "updated_at": 1704067200,
                "created_by": "00000000-0000-0000-0000-000000000003",
                "updated_by": "00000000-0000-0000-0000-000000000003"
            }
        })))
        .mount(&mock_server)
        .await;

    let client = Client::new(&mock_server.uri(), None);
    let result = client
        .create_comment("00000000-0000-0000-0000-000000000001", "Test comment")
        .await
        .unwrap();

    assert_eq!(result["comment"]["content"], "Test comment");
}
