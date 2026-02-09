use pm_core::Sprint;

use serde::Serialize;

/// Sprint DTO for JSON serialization
#[derive(Debug, Serialize)]
pub struct SprintDto {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub goal: Option<String>,
    pub start_date: i64,
    pub end_date: i64,
    pub status: String,
    pub version: i32,
    pub created_at: i64,
    pub updated_at: i64,
    pub created_by: String,
    pub updated_by: String,
}

impl From<Sprint> for SprintDto {
    fn from(s: Sprint) -> Self {
        Self {
            id: s.id.to_string(),
            project_id: s.project_id.to_string(),
            name: s.name,
            goal: s.goal,
            start_date: s.start_date.timestamp(),
            end_date: s.end_date.timestamp(),
            status: s.status.as_str().to_string(),
            version: s.version,
            created_at: s.created_at.timestamp(),
            updated_at: s.updated_at.timestamp(),
            created_by: s.created_by.to_string(),
            updated_by: s.updated_by.to_string(),
        }
    }
}
