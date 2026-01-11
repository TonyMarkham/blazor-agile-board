use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwimLane {
    pub id: Uuid,
    pub project_id: Uuid,

    pub name: String,
    pub status_value: String,
    pub position: i32,

    pub is_default: bool,

    // Audit
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

impl SwimLane {
    pub fn new(project_id: Uuid, name: String, status_value: String, position: i32) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            project_id,
            name,
            status_value,
            position,
            is_default: false,
            created_at: now,
            updated_at: now,
            deleted_at: None,
        }
    }

    pub fn new_default(
        project_id: Uuid,
        name: String,
        status_value: String,
        position: i32,
    ) -> Self {
        let mut lane = Self::new(project_id, name, status_value, position);
        lane.is_default = true;
        lane
    }
}
