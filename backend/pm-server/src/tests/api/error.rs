use crate::ApiError;

use std::panic::Location;

use axum::response::IntoResponse;
use error_location::ErrorLocation;
use http::StatusCode;
use http_body_util::BodyExt;

#[tokio::test]
async fn test_not_found_returns_404_with_json_body() {
    let error = ApiError::NotFound {
        message: "Item not found".into(),
        location: ErrorLocation::from(Location::caller()),
    };
    let response = error.into_response();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["error"]["code"], "NOT_FOUND");
    assert_eq!(json["error"]["message"], "Item not found");
}

#[tokio::test]
async fn test_validation_error_returns_400_with_field() {
    let error = ApiError::Validation {
        message: "Title too long".into(),
        field: Some("title".into()),
        location: ErrorLocation::from(Location::caller()),
    };
    let response = error.into_response();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["error"]["code"], "VALIDATION_ERROR");
    assert_eq!(json["error"]["field"], "title");
}

#[tokio::test]
async fn test_conflict_error_returns_409_with_version() {
    let error = ApiError::Conflict {
        message: "Version mismatch".into(),
        current_version: 5,
        location: ErrorLocation::from(Location::caller()),
    };
    let response = error.into_response();

    assert_eq!(response.status(), StatusCode::CONFLICT);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["error"]["code"], "CONFLICT");
    assert!(json["error"]["message"].as_str().unwrap().contains("5"));
}

#[tokio::test]
async fn test_internal_error_returns_500() {
    let error = ApiError::Internal {
        message: "Database connection failed".into(),
        location: ErrorLocation::from(Location::caller()),
    };
    let response = error.into_response();

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["error"]["code"], "INTERNAL_ERROR");
}

#[test]
fn test_uuid_error_converts_to_validation() {
    let uuid_err = uuid::Uuid::parse_str("not-a-uuid").unwrap_err();
    let api_err: ApiError = uuid_err.into();

    match api_err {
        ApiError::Validation { message, field, .. } => {
            assert!(message.contains("Invalid UUID"));
            assert!(field.is_none());
        }
        _ => panic!("Expected Validation error"),
    }
}
