use crate::TimeEntryDto;

use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct TimeEntryListResponse {
    pub time_entries: Vec<TimeEntryDto>,
}
