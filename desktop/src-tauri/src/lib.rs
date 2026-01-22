mod commands;
mod identity;
mod logging;
mod server;
mod tray;

use logging::setup_logging;
use server::{ServerConfig, ServerManager};
use tray::TrayManager;

#[cfg(test)]
mod tests;

use std::sync::Arc;

use tauri::{Emitter, Manager};
use tracing::{error, info};

const PM_SERVER_CONFIG_DIRECTORY_NAME: &str = ".pm";
const PM_SERVER_CONFIG_FILENAME: &str = "config.toml";

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            // Focus existing window on second instance attempt
            if let Some(window) = app.get_webview_window("main") {
                window.show().ok();
                window.set_focus().ok();
            }
        }))
        .setup(|app| {
            // Get data directory early for logging setup
            let data_dir = app
                .path()
                .app_data_dir()?
                .join(PM_SERVER_CONFIG_DIRECTORY_NAME);
            std::fs::create_dir_all(&data_dir)?;

            // Initialize logging with rotation
            setup_logging(&data_dir)?;

            info!("Starting Project Manager v{}", env!("CARGO_PKG_VERSION"));
            info!("Data directory: {:?}", data_dir);

            // Extract bundled pm-server config on first run
            let pm_config_dest = data_dir.join(PM_SERVER_CONFIG_FILENAME);
            if !pm_config_dest.exists() {
                if let Ok(resource_dir) = app.path().resource_dir() {
                    let pm_config_src = resource_dir
                        .join(PM_SERVER_CONFIG_DIRECTORY_NAME)
                        .join(PM_SERVER_CONFIG_FILENAME);
                    if pm_config_src.exists() {
                        std::fs::copy(&pm_config_src, &pm_config_dest)?;
                        info!("Extracted pm-server config to {}", pm_config_dest.display());
                    }
                }
            }

            // Load or create config
            let config = ServerConfig::load_or_create(&data_dir)
                .map_err(|e| format!("Config error: {}", e))?;

            // Create server manager
            let manager = Arc::new(ServerManager::new(data_dir.clone(), config));
            app.manage(manager.clone());

            // Setup system tray with TrayManager
            let tray_manager = TrayManager::setup(app)?;
            app.manage(tray_manager.clone());

            // Start server in background
            let app_handle = app.handle().clone();
            let manager_clone = manager.clone();
            tauri::async_runtime::spawn(async move {
                match manager_clone.start(&app_handle).await {
                    Ok(()) => {
                        info!("Server started successfully");
                        app_handle.emit("server-ready", ()).ok();
                    }
                    Err(e) => {
                        error!("Failed to start server: {}", e);
                        app_handle.emit("server-error", e.to_string()).ok();
                    }
                }
            });

            // Subscribe to state changes for tray updates
            let app_handle = app.handle().clone();
            let mut state_rx = manager.subscribe();
            tauri::async_runtime::spawn(async move {
                while state_rx.changed().await.is_ok() {
                    let state = state_rx.borrow().clone();

                    // Update tray via TrayManager
                    if let Some(tray_mgr) = app_handle.try_state::<Arc<TrayManager>>() {
                        tray_mgr.update_status(&app_handle, &state);
                    }

                    // Emit to frontend
                    app_handle
                        .emit("server-state-changed", format!("{:?}", state))
                        .ok();
                }
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                // Hide to tray instead of closing
                window.hide().ok();
                api.prevent_close();
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_server_status,
            commands::get_websocket_url,
            commands::load_user_identity,
            commands::save_user_identity,
            commands::backup_corrupted_user_identity,
            commands::restart_server,
            commands::export_diagnostics,
            commands::get_recent_logs,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
