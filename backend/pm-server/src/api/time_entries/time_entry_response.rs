use crate::TimeEntryDto;

use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct TimeEntryResponse {
    pub time_entry: TimeEntryDto,
}
