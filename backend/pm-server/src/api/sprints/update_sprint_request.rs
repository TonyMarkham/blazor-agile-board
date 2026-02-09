use serde::Deserialize;

/// Request body for updating a sprint
#[derive(Debug, Deserialize)]
pub struct UpdateSprintRequest {
    #[serde(default)]
    pub name: Option<String>,

    #[serde(default)]
    pub goal: Option<String>,

    #[serde(default)]
    pub start_date: Option<i64>,

    #[serde(default)]
    pub end_date: Option<i64>,

    /// Sprint status: "planned", "active", or "completed"
    #[serde(default)]
    pub status: Option<String>,

    /// Required for optimistic locking
    pub expected_version: i32,
}
