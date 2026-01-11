use std::panic::Location;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkItemType {
    Project,
    Epic,
    Story,
    Task,
}

impl WorkItemType {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Project => "project",
            Self::Epic => "epic",
            Self::Story => "story",
            Self::Task => "task",
        }
    }

    #[track_caller]
    pub fn from_str(s: &str) -> crate::Result<Self> {
        match s {
            "project" => Ok(Self::Project),
            "epic" => Ok(Self::Epic),
            "story" => Ok(Self::Story),
            "task" => Ok(Self::Task),
            _ => Err(crate::CoreError::InvalidWorkItemType {
                value: s.to_string(),
                location: crate::ErrorLocation::from(Location::caller()),
            }),
        }
    }
}
