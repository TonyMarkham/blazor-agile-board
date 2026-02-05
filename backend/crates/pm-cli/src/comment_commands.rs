use clap::Subcommand;

#[derive(Subcommand)]
pub enum CommentCommands {
    /// List comments on a work item
    List {
        /// Work item ID (UUID)
        work_item_id: String,
    },

    /// Create a comment on a work item
    Create {
        /// Work item ID (UUID)
        #[arg(long)]
        work_item_id: String,

        /// Comment content
        #[arg(long)]
        content: String,
    },

    /// Update a comment
    Update {
        /// Comment ID (UUID)
        id: String,

        /// New content
        #[arg(long)]
        content: String,
    },

    /// Delete a comment
    Delete {
        /// Comment ID (UUID)
        id: String,
    },
}
