//! pm - Blazor Agile Board CLI
//!
//! A command-line interface for managing work items, designed for LLM integration.
//!
//! # Examples
//!
//! ```bash
//! # List all projects
//! pm project list --pretty
//!
//! # Create a work item
//! pm work-item create --project-id <uuid> --type story --title "My task"
//!
//! # Update status
//! pm work-item update <id> --status done --version 1
//! ```

mod cli;
mod client;
mod commands;
mod comment_commands;
mod project_commands;
mod work_item_commands;

use crate::{
    cli::Cli, commands::Commands, comment_commands::CommentCommands,
    project_commands::ProjectCommands, work_item_commands::WorkItemCommands,
};

pub use crate::client::{CliClientResult, error::ClientError};

use pm_cli::Client;
use pm_config::{DEFAULT_HOST, DEFAULT_PORT};

use std::process::ExitCode;

use clap::Parser;

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();

    // Build server URL from config defaults if not provided
    let server_url = cli
        .server
        .unwrap_or_else(|| format!("http://{}:{}", DEFAULT_HOST, DEFAULT_PORT));

    let client = Client::new(&server_url, cli.user_id.as_deref());

    let result = match cli.command {
        // Project commands
        Commands::Project { action } => match action {
            ProjectCommands::List => client.list_projects().await,
            ProjectCommands::Get { id } => client.get_project(&id).await,
        },

        // Work item commands
        Commands::WorkItem { action } => match action {
            WorkItemCommands::Create {
                project_id,
                r#type,
                title,
                description,
                parent_id,
                status,
                priority,
            } => {
                client
                    .create_work_item(
                        &project_id,
                        &r#type,
                        &title,
                        description.as_deref(),
                        parent_id.as_deref(),
                        status.as_deref(),
                        priority.as_deref(),
                    )
                    .await
            }
            WorkItemCommands::Get { id } => client.get_work_item(&id).await,
            WorkItemCommands::List {
                project_id,
                r#type,
                status,
            } => {
                client
                    .list_work_items(&project_id, r#type.as_deref(), status.as_deref())
                    .await
            }
            WorkItemCommands::Update {
                id,
                title,
                description,
                status,
                priority,
                assignee_id,
                sprint_id,
                story_points,
                version,
            } => {
                client
                    .update_work_item(
                        &id,
                        title.as_deref(),
                        description.as_deref(),
                        status.as_deref(),
                        priority.as_deref(),
                        assignee_id.as_deref(),
                        sprint_id.as_deref(),
                        story_points,
                        version,
                    )
                    .await
            }
            WorkItemCommands::Delete { id } => client.delete_work_item(&id).await,
        },

        // Comment commands
        Commands::Comment { action } => match action {
            CommentCommands::List { work_item_id } => client.list_comments(&work_item_id).await,
            CommentCommands::Create {
                work_item_id,
                content,
            } => client.create_comment(&work_item_id, &content).await,
            CommentCommands::Update { id, content } => client.update_comment(&id, &content).await,
            CommentCommands::Delete { id } => client.delete_comment(&id).await,
        },
    };

    // Handle result
    match result {
        Ok(value) => {
            let output = if cli.pretty {
                serde_json::to_string_pretty(&value)
            } else {
                serde_json::to_string(&value)
            };

            match output {
                Ok(json) => {
                    println!("{}", json);
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("Error serializing response: {}", e);
                    ExitCode::FAILURE
                }
            }
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            ExitCode::FAILURE
        }
    }
}
