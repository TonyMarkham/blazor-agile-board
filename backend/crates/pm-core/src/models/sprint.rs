use crate::{CoreError, CoreResult, SprintDto, SprintStatus, parse_timestamp, parse_uuid};

use std::panic::Location;
use std::str::FromStr;

use chrono::{DateTime, Utc};
use error_location::ErrorLocation;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sprint {
    pub id: Uuid,
    pub project_id: Uuid,

    pub name: String,
    pub goal: Option<String>,

    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,

    pub status: SprintStatus,
    pub version: i32,

    // Audit
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Uuid,
    pub updated_by: Uuid,
    pub deleted_at: Option<DateTime<Utc>>,
}

impl Sprint {
    pub fn new(
        project_id: Uuid,
        name: String,
        goal: Option<String>,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
        created_by: Uuid,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            project_id,
            name,
            goal,
            start_date,
            end_date,
            status: SprintStatus::Planned,
            version: 1,
            created_at: now,
            updated_at: now,
            created_by,
            updated_by: created_by,
            deleted_at: None,
        }
    }
}

impl TryFrom<SprintDto> for Sprint {
    type Error = CoreError;

    fn try_from(dto: SprintDto) -> CoreResult<Self> {
        Ok(Sprint {
            id: parse_uuid(&dto.id, "sprint.id")?,
            project_id: parse_uuid(&dto.project_id, "sprint.project_id")?,
            name: dto.name,
            goal: dto.goal,
            start_date: parse_timestamp(dto.start_date, "sprint.start_date")?,
            end_date: parse_timestamp(dto.end_date, "sprint.end_date")?,
            status: SprintStatus::from_str(&dto.status).map_err(|_| CoreError::Validation {
                message: format!("Invalid sprint status: {}", dto.status),
                field: Some("status".into()),
                location: ErrorLocation::from(Location::caller()),
            })?,
            version: dto.version,
            created_at: parse_timestamp(dto.created_at, "sprint.created_at")?,
            updated_at: parse_timestamp(dto.updated_at, "sprint.updated_at")?,
            created_by: parse_uuid(&dto.created_by, "sprint.created_by")?,
            updated_by: parse_uuid(&dto.updated_by, "sprint.updated_by")?,
            deleted_at: None,
        })
    }
}
