use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct CreateTimeEntryRequest {
    pub work_item_id: String,

    /// Optional description of what is being worked on
    #[serde(default)]
    pub description: Option<String>,
}
