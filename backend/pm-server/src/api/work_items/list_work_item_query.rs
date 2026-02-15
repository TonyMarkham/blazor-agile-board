use serde::Deserialize;

/// Query parameters for listing work items
#[derive(Debug, Deserialize)]
pub struct ListWorkItemsQuery {
    #[serde(rename = "type")]
    pub item_type: Option<String>,
    pub status: Option<String>,
    pub sprint_id: Option<String>,
    pub parent_id: Option<String>,
    /// When true, return only items with no parent (parent_id IS NULL)
    #[serde(default)]
    pub orphaned: bool,
    /// Return all descendants (children, grandchildren, etc.) of this work item ID
    pub descendants_of: Option<String>,
    /// Return all ancestors (parent, grandparent, etc.) of this work item ID
    pub ancestors_of: Option<String>,
    /// When true, include work items with status 'done' (default: false)
    #[serde(default)]
    pub include_done: bool,
}
