use clap::Subcommand;

#[derive(Subcommand)]
pub enum SwimLaneCommands {
    /// List swim lanes for a project (ordered by position)
    List {
        /// Project ID (UUID)
        project_id: String,
    },
}
