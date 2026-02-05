use serde::Deserialize;

/// Query parameters for listing work items
#[derive(Debug, Deserialize)]
pub struct ListWorkItemsQuery {
    #[serde(rename = "type")]
    pub item_type: Option<String>,
    pub status: Option<String>,
    pub sprint_id: Option<String>,
}
