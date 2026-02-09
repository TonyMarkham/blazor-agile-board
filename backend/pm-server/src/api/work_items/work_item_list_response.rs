use pm_core::WorkItemDto;

use serde::Serialize;

/// List of work items response
#[derive(Debug, Serialize)]
pub struct WorkItemListResponse {
    pub work_items: Vec<WorkItemDto>,
}
