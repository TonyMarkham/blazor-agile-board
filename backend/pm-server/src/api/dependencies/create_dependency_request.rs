use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct CreateDependencyRequest {
    pub blocking_item_id: String,
    pub blocked_item_id: String,
    /// "blocks" or "relates_to"
    pub dependency_type: String,
}
