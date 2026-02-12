use serde::Deserialize;

/// Request body for updating a work item
#[derive(Debug, Deserialize)]
pub struct UpdateWorkItemRequest {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub priority: Option<String>,
    #[serde(default)]
    pub assignee_id: Option<String>,
    #[serde(default)]
    pub sprint_id: Option<String>,
    #[serde(default)]
    pub story_points: Option<i32>,
    #[serde(default)]
    pub parent_id: Option<String>,
    /// Set to true to update parent_id (allows clearing parent)
    #[serde(default)]
    pub update_parent: bool,
    #[serde(default)]
    pub position: Option<i32>,
    /// Required: current version for optimistic locking
    pub expected_version: i32,
}
