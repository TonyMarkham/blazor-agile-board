use serde::Deserialize;
use uuid::Uuid;

/// Default LLM user UUID - a well-known value for the system LLM user
pub const DEFAULT_LLM_USER_ID: &str = "00000000-0000-0000-0000-000000000001";
/// Default display name for the LLM user
pub const DEFAULT_LLM_USER_NAME: &str = "LLM Assistant";

/// Configuration for the REST API layer
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ApiConfig {
    /// Whether the REST API is enabled
    pub enabled: bool,
    /// UUID for the LLM user (used when no X-User-Id header provided)
    pub llm_user_id: String,
    /// Display name for the LLM user
    pub llm_user_name: String,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            llm_user_id: DEFAULT_LLM_USER_ID.to_string(),
            llm_user_name: DEFAULT_LLM_USER_NAME.to_string(),
        }
    }
}

impl ApiConfig {
    /// Parse the LLM user ID as a UUID
    /// Falls back to the default if parsing fails
    pub fn llm_user_uuid(&self) -> Uuid {
        Uuid::parse_str(&self.llm_user_id).unwrap_or_else(|_| {
            Uuid::parse_str(DEFAULT_LLM_USER_ID).expect("Default LLM user ID is valid")
        })
    }
}
