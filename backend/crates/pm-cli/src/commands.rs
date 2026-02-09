use crate::{
    comment_commands::CommentCommands, dependency_commands::DependencyCommands,
    project_commands::ProjectCommands, sprint_commands::SprintCommands,
    work_item_commands::WorkItemCommands,
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

    /// Launch the desktop app for this repository
    Desktop,
}
