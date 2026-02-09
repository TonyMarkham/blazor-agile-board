use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct CreateProjectRequest {
    /// Project title (required)
    pub title: String,

    /// Unique short key, e.g., "PROJ" (required)
    pub key: String,

    /// Optional description
    #[serde(default)]
    pub description: Option<String>,
}
