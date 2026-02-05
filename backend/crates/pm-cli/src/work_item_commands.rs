use clap::Subcommand;

#[derive(Subcommand)]
pub enum WorkItemCommands {
    /// Create a new work item
    Create {
        /// Project ID (UUID)
        #[arg(long)]
        project_id: String,

        /// Item type: epic, story, or task
        #[arg(long, value_parser = ["epic", "story", "task"])]
        r#type: String,

        /// Work item title
        #[arg(long)]
        title: String,

        /// Work item description
        #[arg(long)]
        description: Option<String>,

        /// Parent work item ID (UUID)
        #[arg(long)]
        parent_id: Option<String>,

        /// Initial status (default: backlog)
        #[arg(long)]
        status: Option<String>,

        /// Priority: low, medium, high, critical (default: medium)
        #[arg(long)]
        priority: Option<String>,
    },

    /// Get a work item by ID
    Get {
        /// Work item ID (UUID)
        id: String,
    },

    /// List work items in a project
    List {
        /// Project ID (UUID)
        project_id: String,

        /// Filter by type: epic, story, or task
        #[arg(long, value_parser = ["epic", "story", "task"])]
        r#type: Option<String>,

        /// Filter by status
        #[arg(long)]
        status: Option<String>,
    },

    /// Update a work item
    Update {
        /// Work item ID (UUID)
        id: String,

        /// New title
        #[arg(long)]
        title: Option<String>,

        /// New description
        #[arg(long)]
        description: Option<String>,

        /// New status: backlog, todo, in_progress, review, done, blocked
        #[arg(long)]
        status: Option<String>,

        /// New priority: low, medium, high, critical
        #[arg(long)]
        priority: Option<String>,

        /// Assignee user ID (UUID, or empty to unassign)
        #[arg(long)]
        assignee_id: Option<String>,

        /// Sprint ID (UUID, or empty to remove from sprint)
        #[arg(long)]
        sprint_id: Option<String>,

        /// Story points (0-100)
        #[arg(long)]
        story_points: Option<i32>,

        /// Expected version (required for optimistic locking)
        #[arg(long)]
        version: i32,
    },

    /// Delete a work item
    Delete {
        /// Work item ID (UUID)
        id: String,
    },
}
