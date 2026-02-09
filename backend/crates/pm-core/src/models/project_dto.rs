use crate::Project;

use serde::{Deserialize, Serialize};

/// Project DTO for JSON serialization
#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectDto {
    pub id: String,
    pub key: String,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub version: i32,
    pub created_at: i64,
    pub updated_at: i64,
    pub created_by: String,
    pub updated_by: String,
    pub next_work_item_number: i32,
}

impl From<Project> for ProjectDto {
    fn from(p: Project) -> Self {
        Self {
            id: p.id.to_string(),
            key: p.key,
            title: p.title,
            description: p.description,
            status: p.status.as_str().to_string(),
            version: p.version,
            created_at: p.created_at.timestamp(),
            updated_at: p.updated_at.timestamp(),
            created_by: p.created_by.to_string(),
            updated_by: p.updated_by.to_string(),
            next_work_item_number: p.next_work_item_number,
        }
    }
}
