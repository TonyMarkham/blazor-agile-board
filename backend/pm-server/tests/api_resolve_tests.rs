#![allow(clippy::unwrap_used)]

//! Integration tests for display key resolution
mod common;

use crate::common::{
    create_test_pool, create_test_project, create_test_user, create_test_work_item,
};

use pm_server::{ApiError, resolve_project, resolve_work_item};

use uuid::Uuid;

// =============================================================================
// resolve_project Tests
// =============================================================================

/// WHAT: Resolving a project by valid UUID returns the correct project
/// WHY: Ensures UUID resolution path works for projects that exist in the database
#[tokio::test]
async fn given_existing_project_when_resolving_by_uuid_then_project_returned() {
    // Given: Database with one project
    let pool = create_test_pool().await;
    let user_id = "00000000-0000-0000-0000-000000000001";
    create_test_user(&pool, user_id).await;
    let project_id = create_test_project(&pool, user_id).await;

    // When: Resolving by UUID string
    let result = resolve_project(&pool, &project_id.to_string()).await;

    // Then: Project is found with correct data
    assert!(result.is_ok());
    let project = result.unwrap();
    assert_eq!(project.id, project_id);
    assert_eq!(project.key, "TEST");
    assert_eq!(project.title, "Test Project");
}

/// WHAT: Resolving a project by valid project key returns the correct project
/// WHY: Ensures key-based resolution path works for human-readable lookups
#[tokio::test]
async fn given_existing_project_when_resolving_by_key_then_project_returned() {
    // Given: Database with one project (key="TEST")
    let pool = create_test_pool().await;
    let user_id = "00000000-0000-0000-0000-000000000001";
    create_test_user(&pool, user_id).await;
    let project_id = create_test_project(&pool, user_id).await;

    // When: Resolving by project key
    let result = resolve_project(&pool, "TEST").await;

    // Then: Project is found with correct data
    assert!(result.is_ok());
    let project = result.unwrap();
    assert_eq!(project.id, project_id);
    assert_eq!(project.key, "TEST");
    assert_eq!(project.title, "Test Project");
}

/// WHAT: Resolving a project by nonexistent UUID returns NotFound error
/// WHY: Ensures proper error handling when UUID doesn't match any project
#[tokio::test]
async fn given_empty_database_when_resolving_by_uuid_then_not_found_error() {
    // Given: Empty database with no projects
    let pool = create_test_pool().await;

    // When: Resolving by a random UUID
    let fake_uuid = Uuid::new_v4();
    let result = resolve_project(&pool, &fake_uuid.to_string()).await;

    // Then: NotFound error is returned with UUID in message
    assert!(result.is_err());
    match result.unwrap_err() {
        ApiError::NotFound { message, .. } => {
            assert!(message.contains(&fake_uuid.to_string()));
        }
        other => panic!("Expected NotFound, got {:?}", other),
    }
}

/// WHAT: Resolving a project by nonexistent key returns NotFound error
/// WHY: Ensures proper error handling when project key doesn't exist
#[tokio::test]
async fn given_empty_database_when_resolving_by_key_then_not_found_error() {
    // Given: Empty database with no projects
    let pool = create_test_pool().await;

    // When: Resolving by a nonexistent project key
    let result = resolve_project(&pool, "NONEXISTENT").await;

    // Then: NotFound error is returned with key in message
    assert!(result.is_err());
    match result.unwrap_err() {
        ApiError::NotFound { message, .. } => {
            assert!(message.contains("NONEXISTENT"));
        }
        other => panic!("Expected NotFound, got {:?}", other),
    }
}

// =============================================================================
// resolve_work_item Tests
// =============================================================================

/// WHAT: Resolving a work item by valid UUID returns the correct work item
/// WHY: Ensures UUID resolution path works for work items that exist in the database
#[tokio::test]
async fn given_existing_work_item_when_resolving_by_uuid_then_work_item_returned() {
    // Given: Database with project and work item
    let pool = create_test_pool().await;
    let user_id = "00000000-0000-0000-0000-000000000001";
    create_test_user(&pool, user_id).await;
    let project_id = create_test_project(&pool, user_id).await;
    let work_item_id = create_test_work_item(&pool, project_id, 126, user_id).await;

    // When: Resolving by UUID string
    let result = resolve_work_item(&pool, &work_item_id.to_string()).await;

    // Then: Work item is found with correct data
    assert!(result.is_ok());
    let work_item = result.unwrap();
    assert_eq!(work_item.id, work_item_id);
    assert_eq!(work_item.item_number, 126);
    assert_eq!(work_item.project_id, project_id);
}

/// WHAT: Resolving a work item by valid display key returns the correct work item
/// WHY: Ensures display key resolution path works end-to-end (project lookup + item lookup)
#[tokio::test]
async fn given_existing_work_item_when_resolving_by_display_key_then_work_item_returned() {
    // Given: Database with project (key="TEST") and work item (number=126)
    let pool = create_test_pool().await;
    let user_id = "00000000-0000-0000-0000-000000000001";
    create_test_user(&pool, user_id).await;
    let project_id = create_test_project(&pool, user_id).await;
    let work_item_id = create_test_work_item(&pool, project_id, 126, user_id).await;

    // When: Resolving by display key "TEST-126"
    let result = resolve_work_item(&pool, "TEST-126").await;

    // Then: Work item is found with correct data
    assert!(result.is_ok());
    let work_item = result.unwrap();
    assert_eq!(work_item.id, work_item_id);
    assert_eq!(work_item.item_number, 126);
    assert_eq!(work_item.project_id, project_id);
}

/// WHAT: Resolving a work item with invalid display key format returns BadRequest error
/// WHY: Ensures validation catches malformed display keys before database queries
#[tokio::test]
async fn given_empty_database_when_resolving_invalid_display_key_then_bad_request_error() {
    // Given: Empty database
    let pool = create_test_pool().await;

    // When: Resolving with invalid display key format (no hyphen)
    let result = resolve_work_item(&pool, "TEST126").await;

    // Then: BadRequest error is returned
    assert!(result.is_err());
    match result.unwrap_err() {
        ApiError::BadRequest { message, .. } => {
            assert!(message.contains("Invalid display key format"));
            assert!(message.contains("TEST126"));
        }
        other => panic!("Expected BadRequest, got {:?}", other),
    }
}

/// WHAT: Resolving a work item by nonexistent UUID returns NotFound error
/// WHY: Ensures proper error handling when UUID doesn't match any work item
#[tokio::test]
async fn given_empty_database_when_resolving_work_item_by_uuid_then_not_found_error() {
    // Given: Empty database with no work items
    let pool = create_test_pool().await;

    // When: Resolving by a random UUID
    let fake_uuid = Uuid::new_v4();
    let result = resolve_work_item(&pool, &fake_uuid.to_string()).await;

    // Then: NotFound error is returned with UUID in message
    assert!(result.is_err());
    match result.unwrap_err() {
        ApiError::NotFound { message, .. } => {
            assert!(message.contains(&fake_uuid.to_string()));
        }
        other => panic!("Expected NotFound, got {:?}", other),
    }
}

/// WHAT: Resolving a work item by valid display key but nonexistent item returns NotFound error
/// WHY: Ensures proper error when project exists but work item number doesn't
#[tokio::test]
async fn given_project_without_work_item_when_resolving_by_display_key_then_not_found_error() {
    // Given: Database with project but no work items
    let pool = create_test_pool().await;
    let user_id = "00000000-0000-0000-0000-000000000001";
    create_test_user(&pool, user_id).await;
    let _project_id = create_test_project(&pool, user_id).await;

    // When: Resolving by display key for nonexistent work item
    let result = resolve_work_item(&pool, "TEST-999").await;

    // Then: NotFound error is returned
    assert!(result.is_err());
    match result.unwrap_err() {
        ApiError::NotFound { message, .. } => {
            assert!(message.contains("TEST-999"));
        }
        other => panic!("Expected NotFound, got {:?}", other),
    }
}

// =============================================================================
// End-to-End HTTP Tests
// =============================================================================

/// WHAT: HTTP GET request with display key returns correct work item
/// WHY: Ensures display key resolution works through full HTTP handler stack
#[tokio::test]
async fn given_work_item_when_http_get_with_display_key_then_work_item_returned() {
    use crate::common::create_test_app_state;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use pm_server::routes::build_router;
    use tower::ServiceExt;

    // Given: Database with project (key="TEST") and work item (number=126)
    let state = create_test_app_state().await;
    let user_id = "00000000-0000-0000-0000-000000000001";
    create_test_user(&state.pool, user_id).await;
    let project_id = create_test_project(&state.pool, user_id).await;
    let work_item_id = create_test_work_item(&state.pool, project_id, 126, user_id).await;

    let app = build_router(state.clone());

    // When: Making HTTP GET request with display key "TEST-126"
    let request = Request::builder()
        .method("GET")
        .uri("/api/v1/work-items/TEST-126")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Then: Response is 200 OK with correct work item data
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["work_item"]["id"], work_item_id.to_string());
    assert_eq!(json["work_item"]["item_number"], 126);
    assert_eq!(json["work_item"]["project_id"], project_id.to_string());
}
