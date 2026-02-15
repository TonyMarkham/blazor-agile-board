use clap::Subcommand;

#[derive(Subcommand)]
pub enum TimeEntryCommands {
    /// List time entries for a work item
    List {
        /// Work item ID (UUID or display key like "PONE-123")
        work_item_id: String,
    },

    /// Get a time entry by ID
    Get {
        /// Time entry ID (UUID)
        id: String,
    },

    /// Start a new timer on a work item
    Create {
        /// Work item ID (UUID or display key like "PONE-123")
        #[arg(long)]
        work_item_id: String,
        /// What you're working on
        #[arg(long)]
        description: Option<String>,
    },

    /// Update a time entry (stop timer or edit description)
    Update {
        /// Time entry ID (UUID)
        id: String,
        /// Stop the running timer
        #[arg(long)]
        stop: bool,
        /// Update description
        #[arg(long)]
        description: Option<String>,
    },

    /// Delete a time entry
    Delete {
        /// Time entry ID (UUID)
        id: String,
    },
}
