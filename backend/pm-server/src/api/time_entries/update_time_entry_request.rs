use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct UpdateTimeEntryRequest {
    /// Set to true to stop the running timer
    #[serde(default)]
    pub stop: Option<bool>,

    /// Update description
    #[serde(default)]
    pub description: Option<String>,
}
