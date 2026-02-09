use clap::Subcommand;

#[derive(Subcommand)]
pub enum SyncCommands {
    /// Export all data to JSON
    Export {
        /// Optional output file (defaults to stdout)
        #[arg(short, long)]
        output: Option<String>,
    },

    /// Import data from JSON file
    Import {
        /// Input JSON file path
        #[arg(short, long)]
        file: String,
    },
}
