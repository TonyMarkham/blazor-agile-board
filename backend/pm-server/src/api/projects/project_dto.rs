use pm_core::Project;

use serde::Serialize;

/// Project DTO for JSON serialization
#[derive(Debug, Serialize)]
pub struct ProjectDto {
    pub id: String,
    pub key: String,
    pub title: String,
    pub description: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl From<Project> for ProjectDto {
    fn from(p: Project) -> Self {
        Self {
            id: p.id.to_string(),
            key: p.key,
            title: p.title,
            description: p.description,
            created_at: p.created_at.timestamp(),
            updated_at: p.updated_at.timestamp(),
        }
    }
}
