use crate::commands::Commands;

use clap::Parser;

#[derive(Parser)]
#[command(name = "pm")]
#[command(about = "Blazor Agile Board CLI for LLM integration")]
#[command(version)]
#[command(propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub(crate) command: Commands,

    /// Server URL (auto-discovered from server.json, or specify manually)
    #[arg(long, global = true)]
    pub(crate) server: Option<String>,

    /// User ID to use for operations (optional, uses LLM user by default)
    #[arg(long, global = true)]
    pub(crate) user_id: Option<String>,

    /// Pretty-print JSON output
    #[arg(long, global = true)]
    pub(crate) pretty: bool,
}
