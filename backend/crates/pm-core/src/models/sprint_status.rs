use std::panic::Location;

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

    #[track_caller]
    pub fn from_str(s: &str) -> crate::Result<Self> {
        match s {
            "planned" => Ok(Self::Planned),
            "active" => Ok(Self::Active),
            "completed" => Ok(Self::Completed),
            "cancelled" => Ok(Self::Cancelled),
            _ => Err(crate::CoreError::InvalidSprintStatus {
                value: s.to_string(),
                location: crate::ErrorLocation::from(Location::caller()),
            }),
        }
    }
}
