use pm_proto::FieldChange;

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

    pub fn created(entity_type: &str, entity_id: Uuid, user_id: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            entity_type: entity_type.to_string(),
            entity_id,
            action: "created".to_string(),
            field_name: None,
            old_value: None,
            new_value: None,
            user_id,
            timestamp: Utc::now(),
            comment: None,
        }
    }

    pub fn updated(
        entity_type: &str,
        entity_id: Uuid,
        user_id: Uuid,
        changes: &[FieldChange],
    ) -> Self {
        let comment = if changes.is_empty() {
            None
        } else {
            Some(format!("{} fields changed", changes.len()))
        };

        Self {
            id: Uuid::new_v4(),
            entity_type: entity_type.to_string(),
            entity_id,
            action: "updated".to_string(),
            field_name: None,
            old_value: None,
            new_value: None,
            user_id,
            timestamp: Utc::now(),
            comment,
        }
    }

    pub fn deleted(entity_type: &str, entity_id: Uuid, user_id: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            entity_type: entity_type.to_string(),
            entity_id,
            action: "deleted".to_string(),
            field_name: None,
            old_value: None,
            new_value: None,
            user_id,
            timestamp: Utc::now(),
            comment: None,
        }
    }
}
