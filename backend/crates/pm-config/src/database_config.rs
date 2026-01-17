use crate::DEFAULT_DATABASE_FILENAME;

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct DatabaseConfig {
    pub path: String,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            path: String::from(DEFAULT_DATABASE_FILENAME),
        }
    }
}
