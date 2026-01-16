use error_location::ErrorLocation;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AuthError {
    #[error("Invalid token: {message} {location}")]
    InvalidToken {
        message: String,
        location: ErrorLocation,
    },

    #[error("Token expired {location}")]
    TokenExpired { location: ErrorLocation },

    #[error("Missing authorization header {location}")]
    MissingHeader { location: ErrorLocation },

    #[error("Invalid authorization scheme: expected 'Bearer' {location}")]
    InvalidScheme { location: ErrorLocation },

    #[error("JWT decode failed: {source} {location}")]
    JwtDecode {
        #[source]
        source: jsonwebtoken::errors::Error,
        location: ErrorLocation,
    },

    #[error("Rate limit exceeded: {limit} requests per {window_secs}s {location}")]
    RateLimitExceeded {
        limit: u32,
        window_secs: u64,
        location: ErrorLocation,
    },

    #[error("Invalid claim '{claim}': {message} {location}")]
    InvalidClaim {
        claim: String,
        message: String,
        location: ErrorLocation,
    },
}

impl AuthError {
    /// Convert to protobuf Error message for client response
    pub fn to_proto_error(&self) -> pm_proto::Error {
        pm_proto::Error {
            code: self.error_code().to_string(),
            message: self.to_string(),
            field: self.field(),
        }
    }

    fn error_code(&self) -> &'static str {
        match self {
            Self::InvalidToken { .. } => "INVALID_TOKEN",
            Self::TokenExpired { .. } => "TOKEN_EXPIRED",
            Self::MissingHeader { .. } => "MISSING_AUTH_HEADER",
            Self::InvalidScheme { .. } => "INVALID_AUTH_SCHEME",
            Self::JwtDecode { .. } => "JWT_DECODE_FAILED",
            Self::RateLimitExceeded { .. } => "RATE_LIMIT_EXCEEDED",
            Self::InvalidClaim { .. } => "INVALID_CLAIM",
        }
    }

    fn field(&self) -> Option<String> {
        match self {
            Self::InvalidClaim { claim, .. } => Some(claim.clone()),
            _ => None,
        }
    }
}

pub type Result<T> = std::result::Result<T, AuthError>;
