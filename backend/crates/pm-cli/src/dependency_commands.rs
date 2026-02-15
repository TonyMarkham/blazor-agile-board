use clap::Subcommand;

#[derive(Subcommand)]
pub enum DependencyCommands {
    /// List dependencies for a work item
    List {
        /// Work item ID (UUID or display key like "PONE-123")
        work_item_id: String,
    },
    /// Create a dependency link between two work items
    Create {
        /// ID of the work item that blocks (UUID or display key like "PONE-123")
        #[arg(long)]
        blocking: String,
        /// ID of the work item that is blocked (UUID or display key like "PONE-124")
        #[arg(long)]
        blocked: String,
        /// Dependency type: blocks or relates_to
        #[arg(long, value_parser = ["blocks", "relates_to"])]
        r#type: String,
    },
    /// Delete a dependency
    Delete {
        /// Dependency ID (UUID)
        id: String,
    },
}
