use crate::sync::entity_import_counts::EntityImportCounts;

use serde::Serialize;

#[derive(Debug, Default, Serialize)]
pub struct ImportResult {
    pub projects: EntityImportCounts,
    pub sprints: EntityImportCounts,
    pub swim_lanes: EntityImportCounts,
    pub work_items: EntityImportCounts,
    pub comments: EntityImportCounts,
    pub dependencies: EntityImportCounts,
    pub time_entries: EntityImportCounts,
}
