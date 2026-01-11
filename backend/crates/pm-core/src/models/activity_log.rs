use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityLog {
    pub id: Uuid,

    pub entity_type: String,
    pub entity_id: Uuid,

    pub action: String,

    pub field_name: Option<String>,
    pub old_value: Option<String>,
    pub new_value: Option<String>,

    pub user_id: Uuid,
    pub timestamp: DateTime<Utc>,

    pub comment: Option<String>,
}

impl ActivityLog {
    pub fn new(entity_type: String, entity_id: Uuid, action: String, user_id: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            entity_type,
            entity_id,
            action,
            field_name: None,
            old_value: None,
            new_value: None,
            user_id,
            timestamp: Utc::now(),
            comment: None,
        }
    }
}
