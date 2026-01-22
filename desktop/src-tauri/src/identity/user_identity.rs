use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Persistent user identity stored locally.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserIdentity {
    pub id: Uuid,
    pub name: Option<String>,
    pub email: Option<String>,
    pub created_at: String,
    pub schema_version: i32,
}
