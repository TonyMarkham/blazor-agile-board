use crate::{CoreError, CoreResult};

use std::panic::Location;

use chrono::{DateTime, Utc};
use error_location::ErrorLocation;
use uuid::Uuid;

pub fn parse_uuid(s: &str, field: &str) -> CoreResult<Uuid> {
    Uuid::parse_str(s).map_err(|_| CoreError::Validation {
        message: format!("Invalid UUID for {}: {}", field, s),
        field: Some(field.into()),
        location: ErrorLocation::from(Location::caller()),
    })
}

pub fn parse_timestamp(ts: i64, field: &str) -> CoreResult<DateTime<Utc>> {
    DateTime::from_timestamp(ts, 0).ok_or_else(|| CoreError::Validation {
        message: format!("Invalid timestamp for {}: {}", field, ts),
        field: Some(field.into()),
        location: ErrorLocation::from(Location::caller()),
    })
}
