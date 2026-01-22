use crate::identity::user_identity::UserIdentity;

use serde::Serialize;

/// Result of loading identity - distinguishes "not found" from errors.
#[derive(Debug, Serialize)]
pub struct LoadResult {
    pub user: Option<UserIdentity>,
    /// Present if file exists but is corrupted
    pub corruption_error: Option<String>,
}
