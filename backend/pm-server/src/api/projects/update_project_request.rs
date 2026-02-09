use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct UpdateProjectRequest {
    #[serde(default)]
    pub title: Option<String>,

    #[serde(default)]
    pub description: Option<String>,

    /// Status: "active" or "archived"
    #[serde(default)]
    pub status: Option<String>,

    /// Required for optimistic locking
    pub expected_version: i32,
}
