use crate::{
    CommentDto, DependencyDto, ProjectDto, SprintDto, SwimLaneDto, TimeEntryDto, WorkItemDto,
};
use serde::{Deserialize, Serialize};

/// Complete export/import payload containing all entity types
#[derive(Debug, Serialize, Deserialize)]
pub struct ExportData {
    /// Schema version for compatibility checks (current: 1)
    pub schema_version: u32,

    /// RFC3339 timestamp when data was exported
    pub exported_at: String,

    /// Source identifier (e.g., "pm-server")
    pub exported_by: String,

    /// All projects
    pub projects: Vec<ProjectDto>,

    /// All work items (epics, stories, tasks)
    pub work_items: Vec<WorkItemDto>,

    /// All sprints
    pub sprints: Vec<SprintDto>,

    /// All comments
    pub comments: Vec<CommentDto>,

    /// All swim lanes (fixed configuration)
    pub swim_lanes: Vec<SwimLaneDto>,

    /// All dependencies
    pub dependencies: Vec<DependencyDto>,

    /// All time entries
    pub time_entries: Vec<TimeEntryDto>,
}
