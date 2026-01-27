use crate::SprintStatus;

use chrono::{DateTime, Utc};
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
