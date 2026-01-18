use crate::circuit_breaker::CircuitBreakerError;

use std::panic::Location;

use error_location::ErrorLocation;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum WsError {
    #[error("Connection closed: {reason} {location}")]
    ConnectionClosed {
        reason: String,
        location: ErrorLocation,
    },

    #[error("Protobuf decode failed: {source} {location}")]
    ProtoDecode {
        #[source]
        source: prost::DecodeError,
        location: ErrorLocation,
    },

    #[error("Protobuf encode failed: {source} {location}")]
    ProtoEncode {
        #[source]
        source: prost::EncodeError,
        location: ErrorLocation,
    },

    #[error("Send buffer full, client too slow {location}")]
    SendBufferFull { location: ErrorLocation },

    #[error("Broadcast channel lagged, missed {missed_count} messages {location}")]
    BroadcastLagged {
        missed_count: u64,
        location: ErrorLocation,
    },

    #[error("Connection limit exceeded: {current} connections (max: {max}) {location}")]
    ConnectionLimitExceeded {
        current: usize,
        max: usize,
        location: ErrorLocation,
    },

    #[error("Invalid message: {message} {location}")]
    InvalidMessage {
        message: String,
        location: ErrorLocation,
    },

    #[error("Heartbeat timeout after {timeout_secs}s {location}")]
    HeartbeatTimeout {
        timeout_secs: u64,
        location: ErrorLocation,
    },

    #[error("Internal error: {message} {location}")]
    Internal {
        message: String,
        location: ErrorLocation,
    },

    #[error("Validation failed: {message}")]
    ValidationError {
        message: String,
        field: Option<String>,
        location: ErrorLocation,
    },

    #[error("Resource not found: {message}")]
    NotFound {
        message: String,
        location: ErrorLocation,
    },

    #[error("Conflict: resource was modified (current version: {current_version})")]
    ConflictError {
        current_version: i32,
        location: ErrorLocation,
    },

    #[error("Cannot delete: {message}")]
    DeleteBlocked {
        message: String,
        location: ErrorLocation,
    },

    #[error("Unauthorized: {message}")]
    Unauthorized {
        message: String,
        location: ErrorLocation,
    },

    // Database error with details
    #[error("Database error: {message}")]
    Database {
        message: String,
        location: ErrorLocation,
    },

    // Service unavailable (circuit breaker open)
    #[error("Service temporarily unavailable. Retry after {retry_after_secs} seconds")]
    ServiceUnavailable {
        retry_after_secs: u64,
        location: ErrorLocation,
    },

    // Request timeout
    #[error("Request timed out after {timeout_secs} seconds")]
    Timeout {
        timeout_secs: u64,
        location: ErrorLocation,
    },
}

impl WsError {
    /// Convert to protobuf Error for client
    pub fn to_proto_error(&self) -> pm_proto::Error {
        pm_proto::Error {
            code: self.error_code().to_string(),
            message: self.to_string(),
            field: match self {
                Self::ValidationError { field, .. } => field.clone(),
                _ => None,
            },
        }
    }

    fn error_code(&self) -> &'static str {
        match self {
            Self::ConnectionClosed { .. } => "CONNECTION_CLOSED",
            Self::ProtoDecode { .. } => "DECODE_ERROR",
            Self::ProtoEncode { .. } => "ENCODE_ERROR",
            Self::SendBufferFull { .. } => "SLOW_CLIENT",
            Self::BroadcastLagged { .. } => "BROADCAST_LAGGED",
            Self::ConnectionLimitExceeded { .. } => "CONNECTION_LIMIT",
            Self::InvalidMessage { .. } => "INVALID_MESSAGE",
            Self::HeartbeatTimeout { .. } => "HEARTBEAT_TIMEOUT",
            Self::Internal { .. } => "INTERNAL_ERROR",
            Self::ValidationError { .. } => "VALIDATION_ERROR",
            Self::NotFound { .. } => "NOT_FOUND",
            Self::ConflictError { .. } => "CONFLICT",
            Self::DeleteBlocked { .. } => "DELETE_BLOCKED",
            Self::Unauthorized { .. } => "UNAUTHORIZED",
            Self::Database { .. } => "DATABASE_ERROR",
            Self::ServiceUnavailable { .. } => "SERVICE_UNAVAILABLE",
            Self::Timeout { .. } => "TIMEOUT",
        }
    }

    /// Check if this error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::Database { .. } | Self::ServiceUnavailable { .. } | Self::Timeout { .. }
        )
    }
}

impl From<prost::DecodeError> for WsError {
    #[track_caller]
    fn from(source: prost::DecodeError) -> Self {
        Self::ProtoDecode {
            source,
            location: ErrorLocation::from(Location::caller()),
        }
    }
}

impl From<prost::EncodeError> for WsError {
    #[track_caller]
    fn from(source: prost::EncodeError) -> Self {
        Self::ProtoEncode {
            source,
            location: ErrorLocation::from(Location::caller()),
        }
    }
}

impl From<pm_db::DbError> for WsError {
    #[track_caller]
    fn from(err: pm_db::DbError) -> Self {
        Self::Database {
            message: err.to_string(),
            location: ErrorLocation::from(Location::caller()),
        }
    }
}

impl From<sqlx::Error> for WsError {
    #[track_caller]
    fn from(err: sqlx::Error) -> Self {
        Self::Database {
            message: err.to_string(),
            location: ErrorLocation::from(Location::caller()),
        }
    }
}

impl From<CircuitBreakerError> for WsError {
    #[track_caller]
    fn from(err: CircuitBreakerError) -> Self {
        match err {
            CircuitBreakerError::CircuitOpen { retry_after_secs } => Self::ServiceUnavailable {
                retry_after_secs,
                location: ErrorLocation::from(Location::caller()),
            },
        }
    }
}

pub type Result<T> = std::result::Result<T, WsError>;
