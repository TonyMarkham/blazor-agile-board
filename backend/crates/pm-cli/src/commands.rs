use crate::{
    comment_commands::CommentCommands, project_commands::ProjectCommands,
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

    /// Launch the desktop app for this repository
    Desktop,
}
