use crate::SprintDto;

use serde::Serialize;

/// Response wrapper for a list of sprints
#[derive(Debug, Serialize)]
pub struct SprintListResponse {
    pub sprints: Vec<SprintDto>,
}
