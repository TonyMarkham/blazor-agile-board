use crate::WorkItem;

use serde::{Deserialize, Serialize};

/// Work item DTO for JSON serialization
#[derive(Debug, Serialize, Deserialize)]
pub struct WorkItemDto {
    pub id: String,
    pub display_key: String,
    pub item_type: String,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub priority: String,
    pub parent_id: Option<String>,
    pub project_id: String,
    pub assignee_id: Option<String>,
    pub sprint_id: Option<String>,
    pub story_points: Option<i32>,
    pub item_number: i32,
    pub position: i32,
    pub version: i32,
    pub created_at: i64,
    pub updated_at: i64,
    pub created_by: String,
    pub updated_by: String,
}

impl WorkItemDto {
    /// Convert from domain model, fetching project key for display_key
    pub fn from_work_item(w: WorkItem, project_key: &str) -> Self {
        Self {
            id: w.id.to_string(),
            display_key: format!("{}-{}", project_key, w.item_number),
            item_type: w.item_type.as_str().to_string(),
            title: w.title,
            description: w.description,
            status: w.status,
            priority: w.priority,
            parent_id: w.parent_id.map(|id| id.to_string()),
            project_id: w.project_id.to_string(),
            assignee_id: w.assignee_id.map(|id| id.to_string()),
            sprint_id: w.sprint_id.map(|id| id.to_string()),
            story_points: w.story_points,
            item_number: w.item_number,
            position: w.position,
            version: w.version,
            created_at: w.created_at.timestamp(),
            updated_at: w.updated_at.timestamp(),
            created_by: w.created_by.to_string(),
            updated_by: w.updated_by.to_string(),
        }
    }
}
