pub mod error_location;

// -------------------------------------------------------------------------- //

use crate::ErrorLocation;

use std::result::Result as StdResult;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum CoreError {
    #[error("Validation error: {message} {location}")]
    Validation {
        message: String,
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
}

pub type Result<T> = StdResult<T, CoreError>;
