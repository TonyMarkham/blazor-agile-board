use serde::Serialize;

#[derive(Debug, Default, Serialize)]
pub struct EntityImportCounts {
    pub created: usize,
    pub updated: usize,
    pub skipped: usize,
}
