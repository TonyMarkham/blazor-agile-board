use crate::{CoreError, CoreResult};

use std::panic::Location;
use std::str::FromStr;

use error_location::ErrorLocation;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkItemType {
    Epic = 2,
    Story = 3,
    Task = 4,
}

impl WorkItemType {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Epic => "epic",
            Self::Story => "story",
            Self::Task => "task",
        }
    }
}

impl FromStr for WorkItemType {
    type Err = CoreError;

    #[track_caller]
    fn from_str(s: &str) -> CoreResult<Self> {
        match s {
            "epic" => Ok(Self::Epic),
            "story" => Ok(Self::Story),
            "task" => Ok(Self::Task),
            _ => Err(CoreError::InvalidWorkItemType {
                value: s.to_string(),
                location: ErrorLocation::from(Location::caller()),
            }),
        }
    }
}

impl From<WorkItemType> for i32 {
    fn from(item_type: WorkItemType) -> Self {
        match item_type {
            WorkItemType::Epic => 2,
            WorkItemType::Story => 3,
            WorkItemType::Task => 4,
        }
    }
}

impl From<i32> for WorkItemType {
    fn from(value: i32) -> Self {
        match value {
            2 => WorkItemType::Epic,
            3 => WorkItemType::Story,
            4 => WorkItemType::Task,
            _ => WorkItemType::Epic, // Default to Epic for unknown values
        }
    }
}
