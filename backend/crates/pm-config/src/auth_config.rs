use crate::DEFAULT_AUTH_ENABLED;

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct AuthConfig {
    pub enabled: bool,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            enabled: DEFAULT_AUTH_ENABLED,
        }
    }
}
