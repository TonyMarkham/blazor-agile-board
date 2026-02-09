use crate::SwimLaneDto;

use serde::Serialize;

/// Response wrapper for list of swim lanes
#[derive(Debug, Serialize)]
pub struct SwimLaneListResponse {
    pub swim_lanes: Vec<SwimLaneDto>,
}
