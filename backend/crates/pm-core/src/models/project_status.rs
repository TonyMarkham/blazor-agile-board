use crate::{CoreError, Result as CoreErrorResult};

use std::panic::Location;
use std::str::FromStr;

use error_location::ErrorLocation;
use serde::{Deserialize, Serialize};

/// Project lifecycle status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ProjectStatus {
    /// Project is active and accepting work
    #[default]
    Active,
    /// Project is archived (read-only, hidden from default views)
    Archived,
}

impl ProjectStatus {
    /// Convert to database string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Archived => "archived",
        }
    }
}

impl FromStr for ProjectStatus {
    type Err = CoreError;

    #[track_caller]
    fn from_str(s: &str) -> CoreErrorResult<Self> {
        match s {
            "active" => Ok(Self::Active),
            "archived" => Ok(Self::Archived),
            _ => Err(CoreError::InvalidProjectStatus {
                value: s.to_string(),
                location: ErrorLocation::from(Location::caller()),
            }),
        }
    }
}

impl std::fmt::Display for ProjectStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
