use clap::Subcommand;

#[derive(Subcommand)]
pub enum SyncCommands {
    /// Export all data to JSON
    Export {
        /// Optional output file (defaults to stdout)
        #[arg(short, long)]
        output: Option<String>,

        /// Export scope (default: full database)
        #[command(subcommand)]
        scope: Option<ExportScope>,
    },

    /// Import data from JSON file
    Import {
        /// Input JSON file path
        #[arg(short, long)]
        file: String,
    },
}

#[derive(Subcommand)]
pub enum ExportScope {
    /// Export a specific work item and optionally its related data
    WorkItem {
        /// Work item ID (UUID)
        id: String,

        /// Include N levels of descendants (0=just item, 1=children, 2=grandchildren, etc.)
        #[arg(long, default_value = "0", value_parser = clap::value_parser!(u32).range(0..=2))]
        descendant_levels: u32,

        /// Include comments for matched work items
        #[arg(long)]
        comments: bool,

        /// Include sprint data for matched work items
        #[arg(long)]
        sprints: bool,

        /// Include dependency links for matched work items
        #[arg(long)]
        dependencies: bool,

        /// Include time entries for matched work items
        #[arg(long)]
        time_entries: bool,
    },
}
