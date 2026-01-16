use std::net::AddrParseError;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ServerError {
    #[error("Invalid bind address: {source}")]
    InvalidBindAddr {
        #[source]
        source: AddrParseError,
    },

    #[error("Missing JWT configuration: must provide either JWT_SECRET or JWT_PUBLIC_KEY")]
    MissingJwtConfig,

    #[error("Environment variable error: {message}")]
    EnvVar { message: String },
}

pub type Result<T> = std::result::Result<T, ServerError>;
