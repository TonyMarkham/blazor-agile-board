use clap::Subcommand;

#[derive(Subcommand)]
pub enum ProjectCommands {
    /// List all projects
    List,
    /// Get a project by ID
    Get {
        /// Project ID (UUID)
        id: String,
    },
}
