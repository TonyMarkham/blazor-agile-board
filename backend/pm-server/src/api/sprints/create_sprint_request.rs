use serde::Deserialize;

/// Request body for creating a sprint
#[derive(Debug, Deserialize)]
pub struct CreateSprintRequest {
    pub project_id: String,
    pub name: String,

    #[serde(default)]
    pub goal: Option<String>,

    /// Unix timestamp (seconds since epoch)
    pub start_date: i64,

    /// Unix timestamp (seconds since epoch)
    pub end_date: i64,
}
