#![allow(dead_code)]

mod commands;
mod identity;
mod logging;
mod pm_directory;
mod server;
mod tray;

use logging::setup_logging;
use pm_directory::PmDir;
use server::{ServerConfig, ServerManager, ServerState};
use tray::TrayManager;

#[cfg(test)]
mod tests;

use pm_config::Config;

use std::sync::Arc;

use tauri::{Emitter, Manager};
use tracing::{error, info};

const SERVER_DATA_DIR: &str = ".server";
const TAURI_DATA_DIR: &str = "tauri";
const PM_SERVER_CONFIG_FILENAME: &str = "config.toml";
const PM_DIR_NAME: &str = ".pm";
const BINARY_CONFIG_FILENAME: &str = "config.json";

// Tauri event names (must match frontend TauriService constants)
const EVENT_SERVER_READY: &str = "server-ready";
const EVENT_SERVER_ERROR: &str = "server-error";
const EVENT_SERVER_STATE_CHANGED: &str = "server-state-changed";

/// Find server directory from config.json next to the binary.
///
/// When Tauri is launched by double-clicking (not from a terminal),
/// `git rev-parse` fails because there's no repo context. The installer
/// writes `.pm/bin/config.json` with `{"repo_root": "/path/to/repo"}`.
/// We read that to find the repo root.
fn find_server_dir_from_binary() -> Option<std::path::PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let exe_dir = exe.parent()?;
    let config_path = exe_dir.join(BINARY_CONFIG_FILENAME);
    let content = std::fs::read_to_string(&config_path).ok()?;
    let parsed: serde_json::Value = serde_json::from_str(&content).ok()?;
    let root = parsed.get("repo_root")?.as_str()?;
    let dir = std::path::PathBuf::from(root).join(PM_DIR_NAME);
    info!("Installed mode (config.json): {}", dir.display());
    Some(dir)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Fix WebKitGTK rendering issues on Linux (blank window until resize)
    // WebKitGTK's GPU-accelerated rendering causes display corruption on many systems,
    // especially ARM64/Raspberry Pi. Disabling compositing forces simpler, compatible rendering.
    // See: https://github.com/tauri-apps/tauri/issues/9289
    // See: https://github.com/tauri-apps/tauri/issues/10626
    #[cfg(target_os = "linux")]
    {
        // SAFETY: Called at program startup before any threads spawn, so no race conditions
        unsafe {
            std::env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
            std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        }
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        // Multiple instances allowed — each repo has its own .pm/ directory and server port
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir()?;

            // Find .pm/ directory:
            // 1. macOS .app bundle: .pm/bin/Foo.app/Contents/MacOS/exe
            // 2. Direct binary: .pm/bin/exe (Linux, Windows, macOS)
            // 3. Git repo root (development mode)
            // 4. Global app data dir (standalone fallback)
            let server_dir = if let Ok(exe) = std::env::current_exe()
                && let Some(exe_dir) = exe.parent()
            {
                // macOS .app bundle detection
                if let Some(contents_dir) = exe_dir.parent()
                    && contents_dir.file_name().and_then(|n| n.to_str()) == Some("Contents")
                    && let Some(app_bundle) = contents_dir.parent()
                    && app_bundle.extension().and_then(|e| e.to_str()) == Some("app")
                    && let Some(bin_dir) = app_bundle.parent()
                    && bin_dir.file_name().and_then(|n| n.to_str()) == Some("bin")
                    && let Some(pm_dir) = bin_dir.parent()
                    && pm_dir.file_name().and_then(|n| n.to_str()) == Some(".pm")
                {
                    info!("App bundle mode: {}", pm_dir.display());
                    pm_dir.to_path_buf()
                }
                // Direct binary in .pm/bin/ (all platforms)
                else if exe_dir.file_name().and_then(|n| n.to_str()) == Some("bin")
                    && let Some(pm_dir) = exe_dir.parent()
                    && pm_dir.file_name().and_then(|n| n.to_str()) == Some(".pm")
                {
                    info!("Binary mode: {}", pm_dir.display());
                    pm_dir.to_path_buf()
                }
                // Fallback to config-based detection
                else {
                    match Config::config_dir() {
                        Ok(dir) => {
                            info!("Repo mode: {}", dir.display());
                            dir
                        }
                        Err(_) => {
                            let dir = app_data_dir.join(PM_DIR_NAME);
                            info!("Standalone mode: {}", dir.display());
                            dir
                        }
                    }
                }
            } else {
                // Can't determine exe path, use config-based detection
                match Config::config_dir() {
                    Ok(dir) => {
                        info!("Repo mode: {}", dir.display());
                        dir
                    }
                    Err(_) => {
                        let dir = app_data_dir.join(PM_DIR_NAME);
                        info!("Standalone mode: {}", dir.display());
                        dir
                    }
                }
            };
            std::fs::create_dir_all(&server_dir)?;

            // Tauri's own config/logs directory — inside .pm/
            let tauri_dir = server_dir.join(TAURI_DATA_DIR);
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
                // Resource is bundled under ".server/" destination (from tauri.conf.json)
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

            // Share the resolved .pm/ path with identity module
            app.manage(PmDir(server_dir.clone()));

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

                    // Update tray via TrayManager (skip on macOS due to event handler bug)
                    #[cfg(not(target_os = "macos"))]
                    if let Some(tray_mgr) = app_handle.try_state::<Arc<TrayManager>>() {
                        tray_mgr.update_status(&app_handle, &state);
                    }

                    // Emit to frontend - extract data from state to avoid lock contention
                    let port = match &state {
                        ServerState::Running { port } => Some(*port),
                        _ => None,
                    };
                    let ws_url = port.map(|p| format!("ws://127.0.0.1:{p}/ws"));

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

            // Open devtools in debug builds only
            #[cfg(debug_assertions)]
            if let Some(window) = app.get_webview_window("main") {
                window.open_devtools();
            }

            Ok(())
        })
        .on_window_event(|_window, event| {
            #[cfg(not(target_os = "linux"))]
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                // On macOS/Windows: hide to tray instead of closing
                _window.hide().ok();
                api.prevent_close();
            }
            // On Linux: close = quit (system tray often not visible on Raspberry Pi)
            #[cfg(target_os = "linux")]
            let _ = event; // Suppress unused warning
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
            commands::get_repo_root,
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
