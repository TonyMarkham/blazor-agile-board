use crate::{
    comment_commands::CommentCommands, dependency_commands::DependencyCommands,
    project_commands::ProjectCommands, sprint_commands::SprintCommands,
    swim_lane_commands::SwimLaneCommands, sync_commands::SyncCommands,
    time_entry_commands::TimeEntryCommands, work_item_commands::WorkItemCommands,
};

use clap::Subcommand;

#[derive(Subcommand)]
pub(crate) enum Commands {
    /// Project operations
    Project {
        #[command(subcommand)]
        action: ProjectCommands,
    },

    /// Sprint operations
    Sprint {
        #[command(subcommand)]
        action: SprintCommands,
    },

    /// Work item operations
    WorkItem {
        #[command(subcommand)]
        action: WorkItemCommands,
    },

    /// Comment operations
    Comment {
        #[command(subcommand)]
        action: CommentCommands,
    },

    /// Dependency operations
    Dependency {
        #[command(subcommand)]
        action: DependencyCommands,
    },

    /// Swim lane operations (read-only)
    SwimLane {
        #[command(subcommand)]
        action: SwimLaneCommands,
    },

    /// Time entry operations (start/stop timers)
    TimeEntry {
        #[command(subcommand)]
        action: TimeEntryCommands,
    },

    /// Launch the desktop app for this repository
    Desktop,

    /// Bulk data sync operations (export/import)
    Sync {
        #[command(subcommand)]
        action: SyncCommands,
    },
}
