use crate::{CoreError, CoreResult, TimeEntryDto, parse_timestamp, parse_uuid};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeEntry {
    pub id: Uuid,
    pub work_item_id: Uuid,
    pub user_id: Uuid,

    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub duration_seconds: Option<i32>,

    pub description: Option<String>,

    // Audit
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

impl TimeEntry {
    pub fn new(work_item_id: Uuid, user_id: Uuid, description: Option<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            work_item_id,
            user_id,
            started_at: now,
            ended_at: None,
            duration_seconds: None,
            description,
            created_at: now,
            updated_at: now,
            deleted_at: None,
        }
    }

    pub fn stop(&mut self) {
        let now = Utc::now();
        self.ended_at = Some(now);
        self.duration_seconds = Some((now - self.started_at).num_seconds() as i32);
        self.updated_at = now;
    }

    pub fn is_running(&self) -> bool {
        self.ended_at.is_none()
    }
}

impl TryFrom<TimeEntryDto> for TimeEntry {
    type Error = CoreError;

    fn try_from(dto: TimeEntryDto) -> CoreResult<Self> {
        Ok(TimeEntry {
            id: parse_uuid(&dto.id, "time_entry.id")?,
            work_item_id: parse_uuid(&dto.work_item_id, "time_entry.work_item_id")?,
            user_id: parse_uuid(&dto.user_id, "time_entry.user_id")?,
            started_at: parse_timestamp(dto.started_at, "time_entry.started_at")?,
            ended_at: dto
                .ended_at
                .map(|ts| parse_timestamp(ts, "time_entry.ended_at"))
                .transpose()?,
            duration_seconds: dto.duration_seconds,
            description: dto.description,
            created_at: parse_timestamp(dto.created_at, "time_entry.created_at")?,
            updated_at: parse_timestamp(dto.updated_at, "time_entry.updated_at")?,
            deleted_at: None,
        })
    }
}
