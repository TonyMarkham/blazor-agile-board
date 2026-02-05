use crate::WorkItemDto;
use serde::Serialize;

/// Single work item response
#[derive(Debug, Serialize)]
pub struct WorkItemResponse {
    pub work_item: WorkItemDto,
}
