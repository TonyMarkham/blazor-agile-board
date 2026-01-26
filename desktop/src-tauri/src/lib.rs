#![allow(dead_code)]

mod commands;
mod identity;
mod logging;
mod server;
mod tray;

use logging::setup_logging;
use server::{ServerConfig, ServerManager, ServerState};
use tray::TrayManager;

#[cfg(test)]
mod tests;

use std::sync::Arc;

use tauri::{Emitter, Manager};
use tracing::{error, info};

const SERVER_DATA_DIR: &str = ".server";
const TAURI_DATA_DIR: &str = ".tauri";
const PM_SERVER_CONFIG_FILENAME: &str = "config.toml";

// Tauri event names (must match frontend TauriService constants)
const EVENT_SERVER_READY: &str = "server-ready";
const EVENT_SERVER_ERROR: &str = "server-error";
const EVENT_SERVER_STATE_CHANGED: &str = "server-state-changed";

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
            let app_data_dir = app.path().app_data_dir()?;

            // Server data directory (.server/) - for pm-server
            let server_dir = app_data_dir.join(SERVER_DATA_DIR);
            std::fs::create_dir_all(&server_dir)?;

            // Tauri data directory (.tauri/) - for Tauri config/logs
            let tauri_dir = app_data_dir.join(TAURI_DATA_DIR);
            std::fs::create_dir_all(&tauri_dir)?;

            // Initialize Tauri logging to .tauri/
            setup_logging(&tauri_dir)?;

            info!("Starting Project Manager v{}", env!("CARGO_PKG_VERSION"));
            info!("Server directory: {:?}", server_dir);
            info!("Tauri directory: {:?}", tauri_dir);

            // Setup signal handlers for graceful shutdown on Unix
            #[cfg(unix)]
            {
                let app_handle = app.handle().clone();
                std::thread::spawn(move || {
                    use signal_hook::consts::{SIGINT, SIGTERM};
                    use signal_hook::iterator::Signals;

                    let mut signals = match Signals::new([SIGINT, SIGTERM]) {
                        Ok(s) => s,
                        Err(e) => {
                            error!("Failed to register signal handlers: {e}");
                            return;
                        }
                    };

                    if let Some(sig) = signals.forever().next() {
                        info!("Received signal {sig}, shutting down...");

                        if let Some(manager) = app_handle.try_state::<Arc<ServerManager>>() {
                            tauri::async_runtime::block_on(async {
                                match manager.stop().await {
                                    Ok(()) => {
                                        info!("Server stopped due to signal {sig}")
                                    }
                                    Err(e) => {
                                        error!("Failed to stop server on signal: {e}")
                                    }
                                }
                            });
                        }

                        std::process::exit(0);
                    }
                });
            }

            // Extract bundled pm-server config on first run (to .server/)
            let pm_config_dest = server_dir.join(PM_SERVER_CONFIG_FILENAME);
            if !pm_config_dest.exists()
                && let Ok(resource_dir) = app.path().resource_dir()
            {
                let pm_config_src = resource_dir
                    .join(SERVER_DATA_DIR)
                    .join(PM_SERVER_CONFIG_FILENAME);
                if pm_config_src.exists() {
                    std::fs::copy(&pm_config_src, &pm_config_dest)?;
                    info!("Extracted pm-server config to {}", pm_config_dest.display());
                }
            }

            // Load or create config from .tauri/
            let config = ServerConfig::load_or_create(&tauri_dir)
                .map_err(|e| format!("Config error: {}", e))?;

            // Create server manager with both directories
            let manager = Arc::new(ServerManager::new(
                server_dir.clone(),
                tauri_dir.clone(),
                config,
            ));
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

                        // Build full status for frontend
                        let state = manager_clone.state().await;
                        let port = manager_clone.port().await;
                        let ws_url = manager_clone.websocket_url().await;
                        let health = manager_clone.health().await;
                        let pid = manager_clone.server_pid().await;

                        let status = commands::build_server_status(
                            &state,
                            port,
                            ws_url,
                            health.as_ref(),
                            pid,
                        );

                        info!("Emitting {EVENT_SERVER_READY} event: port={port:?}, pid={pid:?}");
                        app_handle.emit(EVENT_SERVER_READY, status).ok();
                    }
                    Err(e) => {
                        error!("Failed to start server: {e}");
                        app_handle.emit(EVENT_SERVER_ERROR, e.to_string()).ok();
                    }
                }
            });

            // Subscribe to state changes for tray updates
            let app_handle = app.handle().clone();
            let manager_for_events = manager.clone();
            let mut state_rx = manager.subscribe();
            tauri::async_runtime::spawn(async move {
                info!("State subscription task started");
                while state_rx.changed().await.is_ok() {
                    info!("State change detected");
                    let state = state_rx.borrow().clone();
                    info!("New state: {:?}", state);

                    // Update tray via TrayManager
                    if let Some(tray_mgr) = app_handle.try_state::<Arc<TrayManager>>() {
                        tray_mgr.update_status(&app_handle, &state);
                    }

                    // Emit to frontend - extract data from state to avoid lock contention
                    let port = match &state {
                        ServerState::Running { port } => Some(*port),
                        _ => None,
                    };
                    let ws_url = port.map(|p| format!("ws://127.0.0.1:{}/ws", p));

                    // Get PID for state events
                    let pid = manager_for_events.server_pid().await;

                    let status = commands::build_server_status(
                        &state, port, ws_url,
                        None, // Health check happens separately, not in state events
                        pid,
                    );

                    info!(
                        "Emitting {}: state={}",
                        EVENT_SERVER_STATE_CHANGED, status.state
                    );
                    app_handle.emit(EVENT_SERVER_STATE_CHANGED, status).ok();
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
            commands::wasm_ready,
            commands::load_user_identity,
            commands::save_user_identity,
            commands::backup_corrupted_user_identity,
            commands::restart_server,
            commands::export_diagnostics,
            commands::get_recent_logs,
            commands::quit_app,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            use tauri::RunEvent;

            if let RunEvent::ExitRequested { api, code, .. } = event {
                info!("Exit requested (code: {:?})", code);
                api.prevent_exit();

                let app_handle_clone = app_handle.clone();
                tauri::async_runtime::block_on(async move {
                    if let Some(manager) = app_handle_clone.try_state::<Arc<ServerManager>>() {
                        info!("Stopping server before exit...");
                        match manager.stop().await {
                            Ok(()) => info!("Server stopped successfully"),
                            Err(e) => error!("Failed to stop server: {}", e),
                        }
                    }
                });

                std::process::exit(code.unwrap_or(0));
            }
        });
}
