use std::result::Result as StdResult;

use error_location::ErrorLocation;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CoreError {
    /// Validation error (400)
    #[error("Validation failed: {message} {location}")]
    Validation {
        message: String,
        field: Option<String>,
        location: ErrorLocation,
    },

    #[error("Invalid work item type: {value} {location}")]
    InvalidWorkItemType {
        value: String,
        location: ErrorLocation,
    },

    #[error("Invalid sprint status: {value} {location}")]
    InvalidSprintStatus {
        value: String,
        location: ErrorLocation,
    },

    #[error("Invalid dependency type: {value} {location}")]
    InvalidDependencyType {
        value: String,
        location: ErrorLocation,
    },

    #[error("UUID parse error: {source} {location}")]
    Uuid {
        source: uuid::Error,
        location: ErrorLocation,
    },

    #[error("Invalid project status: {value} {location}")]
    InvalidProjectStatus {
        value: String,
        location: ErrorLocation,
    },
}

pub type Result<T> = StdResult<T, CoreError>;
