use error_location::ErrorLocation;
use std::panic::Location;
use thiserror::Error;

/// Errors that can occur during API calls
#[derive(Error, Debug)]
pub enum ClientError {
    #[error("HTTP request error: {message} {location}")]
    Http {
        message: String,
        location: ErrorLocation,
        #[source]
        source: reqwest::Error,
    },

    #[error("API error: {message} (code: {code}) {location}")]
    Api {
        code: String,
        message: String,
        location: ErrorLocation,
    },

    #[error("JSON parse error: {message} {location}")]
    Json {
        message: String,
        location: ErrorLocation,
        #[source]
        source: serde_json::Error,
    },

    #[error("TOML serialization error: {message} {location}")]
    SerializeToml {
        message: String,
        location: ErrorLocation,
        #[source]
        source: toml::ser::Error,
    },

    #[error("TOML parse error: {message} {location}")]
    ParseToml {
        message: String,
        location: ErrorLocation,
        #[source]
        source: toml::de::Error,
    },

    #[error("{message}")]
    Validation { message: String },

    #[error("I/O error: {message} {location}")]
    Io {
        message: String,
        location: ErrorLocation,
        #[source]
        source: std::io::Error,
    },
}

impl ClientError {
    /// Convert reqwest error with context
    #[track_caller]
    pub fn from_reqwest(err: reqwest::Error) -> Self {
        ClientError::Http {
            message: err.to_string(),
            location: ErrorLocation::from(Location::caller()),
            source: err,
        }
    }

    /// Convert JSON error with context
    #[track_caller]
    pub fn from_json(err: serde_json::Error) -> Self {
        ClientError::Json {
            message: err.to_string(),
            location: ErrorLocation::from(Location::caller()),
            source: err,
        }
    }

    /// Convert a TOML serialization error with context
    #[track_caller]
    pub fn from_toml_ser(err: toml::ser::Error) -> Self {
        ClientError::SerializeToml {
            message: err.to_string(),
            location: ErrorLocation::from(Location::caller()),
            source: err,
        }
    }

    /// Convert a TOML deserialization error with context
    #[track_caller]
    pub fn from_toml(err: toml::de::Error) -> Self {
        ClientError::ParseToml {
            message: err.to_string(),
            location: ErrorLocation::from(Location::caller()),
            source: err,
        }
    }

    /// Create an API error with location
    #[allow(dead_code)]
    #[track_caller]
    pub fn api_error(code: String, message: String) -> Self {
        ClientError::Api {
            code,
            message,
            location: ErrorLocation::from(Location::caller()),
        }
    }

    /// Convert I/O error with context
    #[track_caller]
    pub fn from_io(err: std::io::Error) -> Self {
        ClientError::Io {
            message: err.to_string(),
            location: ErrorLocation::from(Location::caller()),
            source: err,
        }
    }
}

impl From<reqwest::Error> for ClientError {
    #[track_caller]
    fn from(err: reqwest::Error) -> Self {
        ClientError::from_reqwest(err)
    }
}

impl From<serde_json::Error> for ClientError {
    #[track_caller]
    fn from(err: serde_json::Error) -> Self {
        ClientError::from_json(err)
    }
}

impl From<toml::ser::Error> for ClientError {
    #[track_caller]
    fn from(err: toml::ser::Error) -> Self {
        ClientError::from_toml_ser(err)
    }
}

impl From<toml::de::Error> for ClientError {
    #[track_caller]
    fn from(err: toml::de::Error) -> Self {
        ClientError::from_toml(err)
    }
}

impl From<std::io::Error> for ClientError {
    #[track_caller]
    fn from(err: std::io::Error) -> Self {
        ClientError::from_io(err)
    }
}

#[allow(dead_code)]
pub type Result<T> = std::result::Result<T, ClientError>;
