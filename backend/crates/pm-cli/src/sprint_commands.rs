use clap::Subcommand;

#[derive(Subcommand)]
pub enum SprintCommands {
    /// List sprints in a project
    List {
        /// Project ID (UUID or project key like "PONE")
        project_id: String,
    },

    /// Get a sprint by ID
    Get {
        /// Sprint ID (UUID)
        id: String,
    },

    /// Create a new sprint
    Create {
        /// Project ID (UUID or project key like "PONE")
        #[arg(long)]
        project_id: String,

        /// Sprint name
        #[arg(long)]
        name: String,

        /// Start date (Unix timestamp in seconds)
        #[arg(long)]
        start_date: i64,

        /// End date (Unix timestamp in seconds)
        #[arg(long)]
        end_date: i64,

        /// Sprint goal (optional)
        #[arg(long)]
        goal: Option<String>,
    },

    /// Update a sprint
    Update {
        /// Sprint ID (UUID)
        id: String,

        /// New name
        #[arg(long)]
        name: Option<String>,

        /// New goal
        #[arg(long)]
        goal: Option<String>,

        /// New start date (Unix timestamp in seconds)
        #[arg(long)]
        start_date: Option<i64>,

        /// New end date (Unix timestamp in seconds)
        #[arg(long)]
        end_date: Option<i64>,

        /// Sprint status: planned, active, or completed
        #[arg(long, value_parser = ["planned", "active", "completed"])]
        status: Option<String>,

        /// Expected version (required for optimistic locking)
        #[arg(long)]
        expected_version: i32,
    },

    /// Delete a sprint
    Delete {
        /// Sprint ID (UUID)
        id: String,
    },
}
