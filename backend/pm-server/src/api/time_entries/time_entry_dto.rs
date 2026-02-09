use pm_core::TimeEntry;

use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct TimeEntryDto {
    pub id: String,
    pub work_item_id: String,
    pub user_id: String,
    pub started_at: i64,
    pub ended_at: Option<i64>,
    pub duration_seconds: Option<i32>,
    pub description: Option<String>,
    pub is_running: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

impl From<TimeEntry> for TimeEntryDto {
    fn from(te: TimeEntry) -> Self {
        let is_running = te.is_running();
        Self {
            id: te.id.to_string(),
            work_item_id: te.work_item_id.to_string(),
            user_id: te.user_id.to_string(),
            started_at: te.started_at.timestamp(),
            ended_at: te.ended_at.map(|dt| dt.timestamp()),
            duration_seconds: te.duration_seconds,
            description: te.description,
            is_running,
            created_at: te.created_at.timestamp(),
            updated_at: te.updated_at.timestamp(),
        }
    }
}
