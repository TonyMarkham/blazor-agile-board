use clap::Subcommand;

#[derive(Subcommand)]
pub enum ProjectCommands {
    /// List all projects
    List,

    /// Get a project by ID
    Get {
        /// Project ID (UUID or project key like "PONE")
        id: String,
    },

    /// Create a new project
    Create {
        /// Project title
        #[arg(long)]
        title: String,

        /// Unique project key (e.g., "PROJ")
        #[arg(long)]
        key: String,

        /// Optional description
        #[arg(long)]
        description: Option<String>,
    },

    /// Update a project
    Update {
        /// Project ID (UUID or project key like "PONE")
        id: String,

        #[arg(long)]
        title: Option<String>,

        #[arg(long)]
        description: Option<String>,

        /// Status: active or archived
        #[arg(long, value_parser = ["active", "archived"])]
        status: Option<String>,

        /// Expected version (required for optimistic locking)
        #[arg(long)]
        expected_version: i32,
    },

    /// Delete a project
    Delete {
        /// Project ID (UUID or project key like "PONE")
        id: String,
    },
}
