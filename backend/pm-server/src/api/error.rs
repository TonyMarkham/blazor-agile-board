//! REST API error types
//!
//! These errors are designed to produce consistent JSON responses
//! with appropriate HTTP status codes.

use pm_db::DbError;

use std::panic::Location;

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use error_location::ErrorLocation;
use serde::Serialize;
use thiserror::Error;

/// JSON error response body
#[derive(Debug, Serialize)]
pub struct ApiErrorResponse {
    pub error: ApiErrorBody,
}

/// Inner error body with code, message, and optional field
#[derive(Debug, Serialize)]
pub struct ApiErrorBody {
    /// Machine-readable error code (e.g., "NOT_FOUND", "VALIDATION_ERROR")
    pub code: String,
    /// Human-readable error message
    pub message: String,
    /// Field name if this is a validation error for a specific field
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
}

/// API errors with associated HTTP status codes
#[derive(Debug, Error)]
pub enum ApiError {
    /// Resource not found (404)
    #[error("Resource not found: {message} {location}")]
    NotFound {
        message: String,
        location: ErrorLocation,
    },

    /// Validation error (400)
    #[error("Validation failed: {message} {location}")]
    Validation {
        message: String,
        field: Option<String>,
        location: ErrorLocation,
    },

    /// Version conflict for optimistic locking (409)
    #[error("Conflict: resource was modified (current version: {current_version}) {location}")]
    Conflict {
        message: String,
        current_version: i32,
        location: ErrorLocation,
    },

    /// Internal server error (500)
    #[error("Internal error: {message} {location}")]
    Internal {
        message: String,
        location: ErrorLocation,
    },

    /// Bad request (400)
    #[error("Bad request: {message} {location}")]
    BadRequest {
        message: String,
        location: ErrorLocation,
    },
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        // Log the error with location for debugging
        log::error!("{}", self);

        let (status, body) = match self {
            ApiError::NotFound { message, .. } => (
                StatusCode::NOT_FOUND,
                ApiErrorBody {
                    code: "NOT_FOUND".into(),
                    message,
                    field: None,
                },
            ),
            ApiError::Validation { message, field, .. } => (
                StatusCode::BAD_REQUEST,
                ApiErrorBody {
                    code: "VALIDATION_ERROR".into(),
                    message,
                    field,
                },
            ),
            ApiError::Conflict {
                message,
                current_version,
                ..
            } => (
                StatusCode::CONFLICT,
                ApiErrorBody {
                    code: "CONFLICT".into(),
                    message: format!("{} (current version: {})", message, current_version),
                    field: None,
                },
            ),
            ApiError::Internal { message, .. } => (
                StatusCode::INTERNAL_SERVER_ERROR,
                ApiErrorBody {
                    code: "INTERNAL_ERROR".into(),
                    message,
                    field: None,
                },
            ),
            ApiError::BadRequest { message, .. } => (
                StatusCode::BAD_REQUEST,
                ApiErrorBody {
                    code: "BAD_REQUEST".into(),
                    message,
                    field: None,
                },
            ),
        };

        (status, Json(ApiErrorResponse { error: body })).into_response()
    }
}

/// Convert sqlx errors to API errors
impl From<sqlx::Error> for ApiError {
    #[track_caller]
    fn from(e: sqlx::Error) -> Self {
        // Don't expose internal database details to clients
        log::error!("Database error: {}", e);
        ApiError::Internal {
            message: "Database operation failed".to_string(),
            location: ErrorLocation::from(Location::caller()),
        }
    }
}

/// Convert UUID parse errors to API errors
impl From<uuid::Error> for ApiError {
    #[track_caller]
    fn from(e: uuid::Error) -> Self {
        ApiError::Validation {
            message: format!("Invalid UUID format: {}", e),
            field: None,
            location: ErrorLocation::from(Location::caller()),
        }
    }
}

/// Convert pm-ws errors to API errors
impl From<pm_ws::WsError> for ApiError {
    #[track_caller]
    fn from(e: pm_ws::WsError) -> Self {
        // Map WsError variants to appropriate ApiError variants
        match e {
            pm_ws::WsError::NotFound { message, .. } => ApiError::NotFound {
                message,
                location: ErrorLocation::from(Location::caller()),
            },
            pm_ws::WsError::ValidationError { message, field, .. } => ApiError::Validation {
                message,
                field,
                location: ErrorLocation::from(Location::caller()),
            },
            pm_ws::WsError::ConflictError {
                current_version, ..
            } => ApiError::Conflict {
                message: "Version mismatch".to_string(),
                current_version,
                location: ErrorLocation::from(Location::caller()),
            },
            pm_ws::WsError::Unauthorized { message, .. } => ApiError::BadRequest {
                message: format!("Unauthorized: {}", message),
                location: ErrorLocation::from(Location::caller()),
            },
            _ => ApiError::Internal {
                message: e.to_string(),
                location: ErrorLocation::from(Location::caller()),
            },
        }
    }
}

/// Convert database errors to API errors
impl From<DbError> for ApiError {
    #[track_caller]
    fn from(e: DbError) -> Self {
        // Log the database error for debugging
        log::error!("Database error: {}", e);

        match e {
            DbError::TenantNotFound { tenant_id, .. } => ApiError::NotFound {
                message: format!("Tenant {} not found", tenant_id),
                location: ErrorLocation::from(Location::caller()),
            },
            DbError::Sqlx { source, .. } => {
                // Check if it's a NOT NULL constraint or similar user-facing error
                match source {
                    sqlx::Error::RowNotFound => ApiError::NotFound {
                        message: "Resource not found".to_string(),
                        location: ErrorLocation::from(Location::caller()),
                    },
                    _ => ApiError::Internal {
                        message: "Database operation failed".to_string(),
                        location: ErrorLocation::from(Location::caller()),
                    },
                }
            }
            DbError::Migration { message, .. } => ApiError::Internal {
                message: format!("Database migration error: {}", message),
                location: ErrorLocation::from(Location::caller()),
            },
            DbError::Initialization { message, .. } => ApiError::Internal {
                message: format!("Database initialization error: {}", message),
                location: ErrorLocation::from(Location::caller()),
            },
        }
    }
}

pub type Result<T> = std::result::Result<T, ApiError>;
