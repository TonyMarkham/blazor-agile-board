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
mod dependency_commands;
mod project_commands;
mod sprint_commands;
mod swim_lane_commands;
mod sync_commands;
mod time_entry_commands;
mod work_item_commands;

use crate::{
    cli::Cli,
    client::{CliClientResult, error::ClientError},
    commands::Commands,
    comment_commands::CommentCommands,
    dependency_commands::DependencyCommands,
    project_commands::ProjectCommands,
    sprint_commands::SprintCommands,
    swim_lane_commands::SwimLaneCommands,
    sync_commands::SyncCommands,
    time_entry_commands::TimeEntryCommands,
    work_item_commands::WorkItemCommands,
};

use pm_cli::Client;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();

    // Desktop launches Tauri — handle before server discovery
    if matches!(cli.command, Commands::Desktop) {
        return launch_desktop();
    }

    // Discover server URL: explicit flag > port file > error
    let server_url = match cli.server {
        Some(url) => url,
        None => discover_server_url(),
    };

    let client = Client::new(&server_url, cli.user_id.as_deref());

    let result = match cli.command {
        // Project commands
        Commands::Project { action } => match action {
            ProjectCommands::List => client.list_projects().await,
            ProjectCommands::Get { id } => client.get_project(&id).await,
            ProjectCommands::Create {
                title,
                key,
                description,
            } => {
                client
                    .create_project(&title, &key, description.as_deref())
                    .await
            }
            ProjectCommands::Update {
                id,
                title,
                description,
                status,
                expected_version,
            } => {
                client
                    .update_project(
                        &id,
                        title.as_deref(),
                        description.as_deref(),
                        status.as_deref(),
                        expected_version,
                    )
                    .await
            }
            ProjectCommands::Delete { id } => client.delete_project(&id).await,
        },

        // Sprint commands
        Commands::Sprint { action } => match action {
            SprintCommands::List { project_id } => client.list_sprints(&project_id).await,
            SprintCommands::Get { id } => client.get_sprint(&id).await,
            SprintCommands::Create {
                project_id,
                name,
                start_date,
                end_date,
                goal,
            } => {
                client
                    .create_sprint(&project_id, &name, start_date, end_date, goal.as_deref())
                    .await
            }
            SprintCommands::Update {
                id,
                name,
                goal,
                start_date,
                end_date,
                status,
                expected_version,
            } => {
                client
                    .update_sprint(
                        &id,
                        name.as_deref(),
                        goal.as_deref(),
                        start_date,
                        end_date,
                        status.as_deref(),
                        expected_version,
                    )
                    .await
            }
            SprintCommands::Delete { id } => client.delete_sprint(&id).await,
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
                parent_id,
                orphaned,
                descendants_of,
                include_done,
            } => {
                client
                    .list_work_items(
                        &project_id,
                        r#type.as_deref(),
                        status.as_deref(),
                        parent_id.as_deref(),
                        orphaned,
                        descendants_of.as_deref(),
                        include_done,
                    )
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
                parent_id,
                update_parent,
                position,
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
                        parent_id.as_deref(),
                        update_parent,
                        position,
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

        Commands::Dependency { action } => match action {
            DependencyCommands::List { work_item_id } => {
                client.list_dependencies(&work_item_id).await
            }
            DependencyCommands::Create {
                blocking,
                blocked,
                r#type,
            } => client.create_dependency(&blocking, &blocked, &r#type).await,
            DependencyCommands::Delete { id } => client.delete_dependency(&id).await,
        },

        // Swim lane commands (read-only)
        Commands::SwimLane { action } => match action {
            SwimLaneCommands::List { project_id } => client.list_swim_lanes(&project_id).await,
        },

        // Time entry commands
        Commands::TimeEntry { action } => match action {
            TimeEntryCommands::List { work_item_id } => {
                client.list_time_entries(&work_item_id).await
            }
            TimeEntryCommands::Get { id } => client.get_time_entry(&id).await,
            TimeEntryCommands::Create {
                work_item_id,
                description,
            } => {
                client
                    .create_time_entry(&work_item_id, description.as_deref())
                    .await
            }
            TimeEntryCommands::Update {
                id,
                stop,
                description,
            } => {
                let stop_flag = if stop { Some(true) } else { None };
                client
                    .update_time_entry(&id, stop_flag, description.as_deref())
                    .await
            }
            TimeEntryCommands::Delete { id } => client.delete_time_entry(&id).await,
        },

        // Desktop is handled above before server discovery
        Commands::Desktop => unreachable!(),

        // Sync commands (bulk export/import)
        Commands::Sync { action } => match action {
            SyncCommands::Export { output, scope } => match scope {
                Some(sync_commands::ExportScope::WorkItem {
                    id,
                    descendant_levels,
                    comments,
                    sprints,
                    dependencies,
                    time_entries,
                }) => {
                    client
                        .export_data(
                            output.as_deref(),
                            Some(&id),
                            descendant_levels,
                            comments,
                            sprints,
                            dependencies,
                            time_entries,
                        )
                        .await
                }
                None => {
                    client
                        .export_data(output.as_deref(), None, 0, false, false, false, false)
                        .await
                }
            },
            SyncCommands::Import { file } => client.import_data(&file).await,
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

/// Discover the server URL from the port discovery file.
///
/// The pm-server writes a `server.json` file after binding, containing
/// the PID, port, and host. This function reads that file and verifies
/// the server process is still alive.
///
/// Falls back to a clear error message if no server is found.
fn discover_server_url() -> String {
    match pm_config::PortFileInfo::read_live() {
        Ok(Some(info)) => {
            format!("http://{}:{}", info.host, info.port)
        }
        Ok(None) => {
            let port_path = pm_config::PortFileInfo::path()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| ".pm/server.json".to_string());

            eprintln!("Error: No running pm-server found.");
            eprintln!();
            eprintln!("Checked: {}", port_path);
            eprintln!();
            eprintln!("Start the server first:");
            eprintln!("  cargo run -p pm-server");
            eprintln!("  pm desktop                # Desktop mode");
            eprintln!();
            eprintln!("Or specify a server URL explicitly:");
            eprintln!("  pm --server http://127.0.0.1:8000 <command>");
            std::process::exit(1);
        }
        Err(e) => {
            let port_path = pm_config::PortFileInfo::path()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| ".pm/server.json".to_string());

            eprintln!("Error reading port file ({}): {}", port_path, e);
            eprintln!();
            eprintln!("Specify a server URL explicitly:");
            eprintln!("  pm --server http://127.0.0.1:8000 <command>");
            std::process::exit(1);
        }
    }
}

/// Launch the Tauri desktop app for the current repository.
fn launch_desktop() -> ExitCode {
    let repo_root = match pm_config::Config::config_dir() {
        Ok(pm_dir) => match pm_dir.parent() {
            Some(root) => root.to_path_buf(),
            None => {
                eprintln!(
                    "Error: cannot determine repo root from {}",
                    pm_dir.display()
                );
                return ExitCode::FAILURE;
            }
        },
        Err(e) => {
            eprintln!("Error: {}", e);
            eprintln!();
            eprintln!("pm desktop must be run from inside a git repository.");
            return ExitCode::FAILURE;
        }
    };

    let pm_dir = repo_root.join(".pm");

    // Ensure .pm/ directory exists
    if let Err(e) = std::fs::create_dir_all(&pm_dir) {
        eprintln!("Error: cannot create {}: {}", pm_dir.display(), e);
        return ExitCode::FAILURE;
    }

    // Find Tauri binary
    let binary = match find_tauri_binary(&pm_dir) {
        Some(path) => path,
        None => {
            eprintln!("Error: Tauri desktop app not found.");
            eprintln!();
            eprintln!("Searched locations:");
            eprintln!("  1. {}/bin/", pm_dir.display());
            eprintln!("  2. Next to the pm binary");
            eprintln!();
            eprintln!("Install the desktop app or build from source:");
            eprintln!("  just build");
            return ExitCode::FAILURE;
        }
    };

    eprintln!("Launching desktop app: {}", binary.display());
    eprintln!("Repository: {}", repo_root.display());

    // On macOS, use `open` for .app bundles to properly detach from terminal
    #[cfg(target_os = "macos")]
    {
        if let Some(app_bundle) = binary
            .ancestors()
            .find(|p| p.extension().and_then(|e| e.to_str()) == Some("app"))
        {
            match std::process::Command::new("open")
                .arg(app_bundle)
                .arg("--args") // Separator for app arguments (none currently)
                .current_dir(&repo_root)
                .spawn()
            {
                Ok(_) => return ExitCode::SUCCESS,
                Err(e) => {
                    eprintln!("Error: failed to launch via open: {}", e);
                    return ExitCode::FAILURE;
                }
            }
        }
    }

    // Direct spawn for non-.app bundles or non-macOS
    match std::process::Command::new(&binary)
        .current_dir(&repo_root)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
    {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("Error: failed to launch {}: {}", binary.display(), e);
            ExitCode::FAILURE
        }
    }
}

/// Search for the Tauri binary.
///
/// Search order:
/// 1. <repo>/.pm/bin/ — installed location
/// 2. Next to the current executable — co-located dev/release builds
fn find_tauri_binary(pm_dir: &std::path::Path) -> Option<PathBuf> {
    let bin_dir = pm_dir.join("bin");

    // macOS .app bundle (installed builds)
    #[cfg(target_os = "macos")]
    {
        let macos_app = bin_dir
            .join("Project Manager.app")
            .join("Contents")
            .join("MacOS")
            .join("project-manager");
        if macos_app.exists() {
            return Some(macos_app);
        }
    }

    // Unix binary (macOS fallback + Linux)
    #[cfg(unix)]
    {
        let unix_bin = bin_dir.join("project-manager");
        if unix_bin.exists() {
            return Some(unix_bin);
        }
    }

    // Windows binary
    #[cfg(windows)]
    {
        let win_bin = bin_dir.join("project-manager.exe");
        if win_bin.exists() {
            return Some(win_bin);
        }
    }

    // Sibling to current executable
    if let Ok(exe) = std::env::current_exe()
        && let Some(exe_dir) = exe.parent()
    {
        let sibling = exe_dir.join("project-manager");
        if sibling.exists() {
            return Some(sibling);
        }
    }

    None
}
