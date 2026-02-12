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

        /// Filter by parent work item ID (UUID)
        #[arg(long, conflicts_with = "orphaned")]
        parent_id: Option<String>,

        /// Show only orphaned items (no parent)
        #[arg(long, conflicts_with = "parent_id")]
        orphaned: bool,

        /// Show all descendants (children, grandchildren, etc.) of a work item ID
        #[arg(long, conflicts_with_all = ["parent_id", "orphaned"])]
        descendants_of: Option<String>,

        /// Include work items with status 'done' (excluded by default)
        #[arg(long)]
        include_done: bool,
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

        /// Parent work item ID (UUID, or empty string to clear parent)
        #[arg(long)]
        parent_id: Option<String>,

        /// Set this flag to update the parent (required to distinguish "don't change" from "clear parent")
        #[arg(long)]
        update_parent: bool,

        /// Position for ordering (non-negative integer)
        #[arg(long)]
        position: Option<i32>,

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
