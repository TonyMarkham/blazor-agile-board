use pm_core::ErrorLocation;

use std::panic::Location;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum DbError {
    #[error("SQLx error: {source} {location}")]
    Sqlx {
        source: sqlx::Error,
        location: ErrorLocation,
    },

    #[error("Migration error: {message} {location}")]
    Migration {
        message: String,
        location: ErrorLocation,
    },

    #[error("Database initialization failed: {message} {location}")]
    Initialization {
        message: String,
        location: ErrorLocation,
    },

    #[error("Tenant database not found: {tenant_id} {location}")]
    TenantNotFound {
        tenant_id: String,
        location: ErrorLocation,
    },
}

impl From<sqlx::Error> for DbError {
    #[track_caller]
    fn from(source: sqlx::Error) -> Self {
        Self::Sqlx {
            source,
            location: ErrorLocation::from(Location::caller()),
        }
    }
}

pub type Result<T> = std::result::Result<T, DbError>;
