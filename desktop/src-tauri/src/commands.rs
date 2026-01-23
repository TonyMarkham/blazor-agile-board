//! Tauri IPC commands for frontend communication.

use crate::identity::{
    backup_corrupted, load, load_result::LoadResult, save, user_identity::UserIdentity,
};
use crate::server::{HealthStatus, ServerManager, ServerState, ServerStatus};

use std::sync::Arc;

use log::error;
use serde::Serialize;
use tauri::{Manager, State};

/// Get current server status.
///
/// Called by frontend to check server state and get WebSocket URL.
#[tauri::command]
pub async fn get_server_status(
    manager: State<'_, Arc<ServerManager>>,
) -> Result<ServerStatus, String> {
    let state = manager.state().await;
    let port = manager.port().await;
    let ws_url = manager.websocket_url().await;
    let health = manager.health().await;
    let pid = manager.server_pid().await;

    Ok(build_server_status(
        &state,
        port,
        ws_url,
        health.as_ref(),
        pid,
    ))
}

/// Get WebSocket URL for frontend connection.
#[tauri::command]
pub async fn get_websocket_url(manager: State<'_, Arc<ServerManager>>) -> Result<String, String> {
    manager
        .websocket_url()
        .await
        .ok_or_else(|| "Server not running".into())
}

/// Called by WASM after it subscribes to events.
/// Returns current server status - enables race-free startup handshake.
///
/// The handshake protocol:
/// 1. WASM subscribes to server-state-changed events
/// 2. WASM calls wasm_ready (this command)
/// 3. Tauri responds with current ServerStatus
/// 4. If server already running, WASM has the port
/// 5. If server still starting, WASM waits for event
#[tauri::command]
pub async fn wasm_ready(manager: State<'_, Arc<ServerManager>>) -> Result<ServerStatus, String> {
    tracing::info!("WASM ready notification received");

    let state = manager.state().await;
    let port = manager.port().await;
    let ws_url = manager.websocket_url().await;
    let health = manager.health().await;
    let pid = manager.server_pid().await;

    Ok(build_server_status(
        &state,
        port,
        ws_url,
        health.as_ref(),
        pid,
    ))
}

/// Manually restart the server.
#[tauri::command]
pub async fn restart_server(
    app: tauri::AppHandle,
    manager: State<'_, Arc<ServerManager>>,
) -> Result<(), String> {
    manager.stop().await.map_err(|e| e.to_string())?;
    manager.start(&app).await.map_err(|e| e.to_string())?;
    Ok(())
}

/// Export diagnostic information as a zip file.
#[tauri::command]
pub async fn export_diagnostics(
    manager: State<'_, Arc<ServerManager>>,
    app: tauri::AppHandle,
) -> Result<String, String> {
    use std::io::Write;

    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;

    let export_path = data_dir.join("diagnostics.zip");

    let file = std::fs::File::create(&export_path).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipWriter::new(file);

    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    // Add system info
    let system_info = format!(
        "OS: {}\nArch: {}\nVersion: {}\nTimestamp: {}",
        std::env::consts::OS,
        std::env::consts::ARCH,
        env!("CARGO_PKG_VERSION"),
        chrono::Utc::now().to_rfc3339(),
    );
    zip.start_file("system_info.txt", options)
        .map_err(|e| e.to_string())?;
    zip.write_all(system_info.as_bytes())
        .map_err(|e| e.to_string())?;

    // Add server status
    let state = manager.state().await;
    let port = manager.port().await;
    let ws_url = manager.websocket_url().await;
    let health = manager.health().await;
    let pid = manager.server_pid().await;
    let status = build_server_status(&state, port, ws_url, health.as_ref(), pid);

    let status_json = serde_json::to_string_pretty(&status).unwrap();
    zip.start_file("server_status.json", options)
        .map_err(|e| e.to_string())?;
    zip.write_all(status_json.as_bytes())
        .map_err(|e| e.to_string())?;

    // Add log files
    let logs_dir = data_dir.join("logs");
    if logs_dir.exists() {
        for entry in (std::fs::read_dir(&logs_dir).map_err(|e| e.to_string())?).flatten() {
            let path = entry.path();
            if path.is_file() {
                let name = format!("logs/{}", path.file_name().unwrap().to_string_lossy());
                zip.start_file(&name, options).map_err(|e| e.to_string())?;
                let content = std::fs::read(&path).map_err(|e| e.to_string())?;
                zip.write_all(&content).map_err(|e| e.to_string())?;
            }
        }
    }

    // Add config (sanitized - remove secrets)
    let config_path = data_dir.join("config.toml");
    if config_path.exists() {
        let config_content = std::fs::read_to_string(&config_path).map_err(|e| e.to_string())?;
        let sanitized = config_content
            .lines()
            .filter(|l| !l.contains("secret") && !l.contains("password") && !l.contains("key"))
            .collect::<Vec<_>>()
            .join("\n");
        zip.start_file("config.toml", options)
            .map_err(|e| e.to_string())?;
        zip.write_all(sanitized.as_bytes())
            .map_err(|e| e.to_string())?;
    }

    zip.finish().map_err(|e| e.to_string())?;

    Ok(export_path.to_string_lossy().into())
}

/// Get recent log lines.
#[tauri::command]
pub async fn get_recent_logs(
    app: tauri::AppHandle,
    lines: Option<usize>,
) -> Result<Vec<String>, String> {
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;

    let log_path = data_dir.join("logs").join("pm-server.log");

    if !log_path.exists() {
        return Ok(vec!["No logs available yet.".into()]);
    }

    let content = std::fs::read_to_string(&log_path).map_err(|e| e.to_string())?;
    let lines_to_return = lines.unwrap_or(100);

    let log_lines: Vec<String> = content
        .lines()
        .rev()
        .take(lines_to_return)
        .map(String::from)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();

    Ok(log_lines)
}

/// Loads user identity from app data directory.
#[tauri::command]
pub async fn load_user_identity(app: tauri::AppHandle) -> Result<LoadResult, String> {
    load(&app).map_err(|e| {
        error!("Failed to load identity: {e}");
        format!("{e}\n\nHint: {}", e.recovery_hint())
    })
}

/// Saves user identity using atomic write pattern.
#[tauri::command]
pub async fn save_user_identity(app: tauri::AppHandle, user: UserIdentity) -> Result<(), String> {
    save(&app, &user).map_err(|e| {
        error!("Failed to save identity: {e}");
        format!("{e}\n\nHint: {}", e.recovery_hint())
    })
}

/// Backs up corrupted user.json for debugging.
#[tauri::command]
pub async fn backup_corrupted_user_identity(app: tauri::AppHandle) -> Result<(), String> {
    backup_corrupted(&app).map(|_| ()).map_err(|e| {
        error!("Failed to backup corrupted identity: {e}");
        format!("{e}\n\nHint: {}", e.recovery_hint())
    })
}

/// Converts internal server state to frontend-facing status.
///
/// Shared by `get_server_status` command and state change events.
/// Health parameter is optional since state events don't include health checks.
pub fn build_server_status(
    state: &ServerState,
    port: Option<u16>,
    ws_url: Option<String>,
    health: Option<&HealthStatus>,
    pid: Option<u32>,
) -> ServerStatus {
    let (state_str, error, recovery_hint) = match state {
        ServerState::Stopped => ("stopped".into(), None, None),
        ServerState::Starting => ("starting".into(), None, None),
        ServerState::Running { .. } => ("running".into(), None, None),
        ServerState::Restarting { attempt } => {
            (format!("restarting (attempt {})", attempt), None, None)
        }
        ServerState::ShuttingDown => ("shutting_down".into(), None, None),
        ServerState::Failed { error } => (
            "failed".into(),
            Some(error.clone()),
            Some("Please check the logs or restart the application.".into()),
        ),
    };

    let is_healthy = state_str == "running"
        && health.map_or(false, |h| matches!(h, HealthStatus::Healthy { .. }));

    ServerStatus {
        state: state_str,
        port,
        websocket_url: ws_url,
        health: health.map(|h| h.into()),
        error,
        recovery_hint,
        is_healthy,
        pid,
    }
}
