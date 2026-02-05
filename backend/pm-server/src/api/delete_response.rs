use serde::Serialize;

/// Delete response
#[derive(Debug, Serialize)]
pub struct DeleteResponse {
    pub deleted_id: String,
}
