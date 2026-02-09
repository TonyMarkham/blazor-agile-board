use pm_core::SprintDto;

use serde::Serialize;

/// Response wrapper for a single sprint
#[derive(Debug, Serialize)]
pub struct SprintResponse {
    pub sprint: SprintDto,
}
