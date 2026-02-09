use pm_core::Dependency;

use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct DependencyDto {
    pub id: String,
    pub blocking_item_id: String,
    pub blocked_item_id: String,
    pub dependency_type: String,
    pub created_at: i64,
    pub created_by: String,
}

impl From<Dependency> for DependencyDto {
    fn from(d: Dependency) -> Self {
        Self {
            id: d.id.to_string(),
            blocking_item_id: d.blocking_item_id.to_string(),
            blocked_item_id: d.blocked_item_id.to_string(),
            dependency_type: d.dependency_type.as_str().to_string(),
            created_at: d.created_at.timestamp(),
            created_by: d.created_by.to_string(),
        }
    }
}
