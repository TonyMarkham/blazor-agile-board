use clap::Subcommand;

#[derive(Subcommand)]
pub enum SwimLaneCommands {
    /// List swim lanes for a project (ordered by position)
    List {
        /// Project ID (UUID or project key like "PONE")
        project_id: String,
    },
}
