use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMember {
    pub id: Uuid,
    pub project_id: Uuid,
    pub user_id: Uuid,
    pub role: String,
    pub created_at: DateTime<Utc>,
}

impl ProjectMember {
    pub fn new(project_id: Uuid, user_id: Uuid, role: &str) -> Self {
        Self {
            id: Uuid::new_v4(),
            project_id,
            user_id,
            role: role.to_string(),
            created_at: Utc::now(),
        }
    }

    pub fn has_permission(&self, required: Permission) -> bool {
        matches!(
            (self.role.as_str(), required),
            ("admin", _)
                | ("editor", Permission::View | Permission::Edit)
                | ("viewer", Permission::View)
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Permission {
    View,
    Edit,
    Admin,
}
