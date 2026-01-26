use crate::server::config::{default_db_filename, default_true};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseSettings {
    /// Database filename (relative to data directory)
    #[serde(default = "default_db_filename")]
    pub filename: String,

    /// Enable WAL checkpoint on shutdown
    #[serde(default = "default_true")]
    pub checkpoint_on_shutdown: bool,

    /// Backup before migrations
    #[serde(default = "default_true")]
    pub backup_before_migration: bool,
}

impl Default for DatabaseSettings {
    fn default() -> Self {
        Self {
            filename: default_db_filename(),
            checkpoint_on_shutdown: default_true(),
            backup_before_migration: default_true(),
        }
    }
}
