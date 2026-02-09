use pm_core::SwimLane;

use serde::Serialize;

/// Swim Lane DTO for JSON serialization
#[derive(Debug, Serialize)]
pub struct SwimLaneDto {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub status_value: String,
    pub position: i32,
    pub is_default: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

impl From<SwimLane> for SwimLaneDto {
    fn from(sl: SwimLane) -> Self {
        Self {
            id: sl.id.to_string(),
            project_id: sl.project_id.to_string(),
            name: sl.name,
            status_value: sl.status_value,
            position: sl.position,
            is_default: sl.is_default,
            created_at: sl.created_at.timestamp(),
            updated_at: sl.updated_at.timestamp(),
        }
    }
}
