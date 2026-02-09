use crate::{CoreError, CoreResult};

use std::panic::Location;
use std::str::FromStr;

use error_location::ErrorLocation;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SprintStatus {
    Planned,
    Active,
    Completed,
    Cancelled,
}

impl SprintStatus {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Planned => "planned",
            Self::Active => "active",
            Self::Completed => "completed",
            Self::Cancelled => "cancelled",
        }
    }
}

impl FromStr for SprintStatus {
    type Err = CoreError;

    #[track_caller]
    fn from_str(s: &str) -> CoreResult<Self> {
        match s {
            "planned" => Ok(Self::Planned),
            "active" => Ok(Self::Active),
            "completed" => Ok(Self::Completed),
            "cancelled" => Ok(Self::Cancelled),
            _ => Err(CoreError::InvalidSprintStatus {
                value: s.to_string(),
                location: ErrorLocation::from(Location::caller()),
            }),
        }
    }
}
