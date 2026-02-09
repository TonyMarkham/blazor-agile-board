use crate::{CoreError, CoreResult, DependencyDto, DependencyType, parse_timestamp, parse_uuid};

use std::panic::Location;
use std::str::FromStr;

use chrono::{DateTime, Utc};
use error_location::ErrorLocation;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    pub id: Uuid,

    pub blocking_item_id: Uuid,
    pub blocked_item_id: Uuid,

    pub dependency_type: DependencyType,

    // Audit
    pub created_at: DateTime<Utc>,
    pub created_by: Uuid,
    pub deleted_at: Option<DateTime<Utc>>,
}

impl Dependency {
    pub fn new(
        blocking_item_id: Uuid,
        blocked_item_id: Uuid,
        dependency_type: DependencyType,
        created_by: Uuid,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            blocking_item_id,
            blocked_item_id,
            dependency_type,
            created_at: Utc::now(),
            created_by,
            deleted_at: None,
        }
    }
}

impl TryFrom<DependencyDto> for Dependency {
    type Error = CoreError;

    fn try_from(dto: DependencyDto) -> CoreResult<Self> {
        Ok(Dependency {
            id: parse_uuid(&dto.id, "dependency.id")?,
            blocking_item_id: parse_uuid(&dto.blocking_item_id, "dependency.blocking_item_id")?,
            blocked_item_id: parse_uuid(&dto.blocked_item_id, "dependency.blocked_item_id")?,
            dependency_type: DependencyType::from_str(&dto.dependency_type).map_err(|_| {
                CoreError::Validation {
                    message: format!("Invalid dependency type: {}", dto.dependency_type),
                    field: Some("dependency_type".into()),
                    location: ErrorLocation::from(Location::caller()),
                }
            })?,
            created_at: parse_timestamp(dto.created_at, "dependency.created_at")?,
            created_by: parse_uuid(&dto.created_by, "dependency.created_by")?,
            deleted_at: None,
        })
    }
}
