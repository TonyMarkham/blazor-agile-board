
use std::panic::Location;

use error_location::ErrorLocation;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum WsError {
    #[error("Connection closed: {reason} {location}")]
    ConnectionClosed { reason: String, location: ErrorLocation },

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

    #[error("Connection limit exceeded: tenant {tenant_id} has {current} connections (max: {max}) {location}")]
    ConnectionLimitExceeded {
        tenant_id: String,
        current: usize,
        max: usize,
        location: ErrorLocation,
    },

    #[error("Invalid message: {message} {location}")]
    InvalidMessage { message: String, location: ErrorLocation },

    #[error("Heartbeat timeout after {timeout_secs}s {location}")]
    HeartbeatTimeout {
        timeout_secs: u64,
        location: ErrorLocation,
    },

    #[error("Internal error: {message} {location}")]
    Internal { message: String, location: ErrorLocation },
}

impl WsError {
    /// Convert to protobuf Error for client
    pub fn to_proto_error(&self) -> pm_proto::Error {
        pm_proto::Error {
            code: self.error_code().to_string(),
            message: self.to_string(),
            field: None,
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
        }
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

pub type Result<T> = std::result::Result<T, WsError>;