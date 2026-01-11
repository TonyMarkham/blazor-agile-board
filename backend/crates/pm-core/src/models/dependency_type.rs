use std::panic::Location;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DependencyType {
    Blocks,
    RelatesTo,
}

impl DependencyType {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Blocks => "blocks",
            Self::RelatesTo => "relates_to",
        }
    }

    #[track_caller]
    pub fn from_str(s: &str) -> crate::Result<Self> {
        match s {
            "blocks" => Ok(Self::Blocks),
            "relates_to" => Ok(Self::RelatesTo),
            _ => Err(crate::CoreError::InvalidDependencyType {
                value: s.to_string(),
                location: crate::ErrorLocation::from(Location::caller()),
            }),
        }
    }
}
