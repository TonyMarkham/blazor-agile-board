use crate::DependencyType;

use chrono::{DateTime, Utc};
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
