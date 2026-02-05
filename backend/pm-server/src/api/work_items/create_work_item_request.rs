use serde::Deserialize;

/// Request body for creating a work item
#[derive(Debug, Deserialize)]
pub struct CreateWorkItemRequest {
    pub project_id: String,
    pub item_type: String,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub parent_id: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub priority: Option<String>,
}
