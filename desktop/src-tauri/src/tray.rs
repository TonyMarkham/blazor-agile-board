//! System tray with status indicator and menu.

use crate::server::{ServerManager, ServerState};

use std::sync::Arc;

use tauri::{
    AppHandle, Manager, Runtime,
    menu::{Menu, MenuId, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};

/// Manages the system tray and its state.
pub struct TrayManager {
    status_item_id: MenuId,
}

impl TrayManager {
    /// Create and setup the system tray.
    pub fn setup(app: &tauri::App) -> Result<Arc<Self>, Box<dyn std::error::Error>> {
        // Create menu items
        let show_item = MenuItem::with_id(app, "show", "Show Window", true, None::<&str>)?;
        let status_item =
            MenuItem::with_id(app, "status", "Status: Starting...", false, None::<&str>)?;
        let status_item_id = status_item.id().clone();

        let separator1 = PredefinedMenuItem::separator(app)?;
        let restart_item = MenuItem::with_id(app, "restart", "Restart Server", true, None::<&str>)?;
        let logs_item = MenuItem::with_id(app, "logs", "View Logs...", true, None::<&str>)?;
        let separator2 = PredefinedMenuItem::separator(app)?;
        let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

        // Build menu
        let menu = Menu::with_items(
            app,
            &[
                &show_item,
                &status_item,
                &separator1,
                &restart_item,
                &logs_item,
                &separator2,
                &quit_item,
            ],
        )?;

        // Create tray icon
        let _tray = TrayIconBuilder::new()
            .icon(app.default_window_icon().unwrap().clone())
            .menu(&menu)
            .tooltip("Project Manager")
            .show_menu_on_left_click(false)
            .on_menu_event(move |app, event| match event.id.as_ref() {
                "show" => {
                    if let Some(window) = app.get_webview_window("main") {
                        window.show().ok();
                        window.set_focus().ok();
                    }
                }
                "restart" => {
                    let app_handle = app.clone();
                    tauri::async_runtime::spawn(async move {
                        if let Some(manager) = app_handle.try_state::<Arc<ServerManager>>() {
                            if let Err(e) = manager.stop().await {
                                tracing::error!("Failed to stop server: {}", e);
                            }
                            if let Err(e) = manager.start(&app_handle).await {
                                tracing::error!("Failed to restart server: {}", e);
                            }
                        }
                    });
                }
                "logs" => {
                    if let Ok(data_dir) = app.path().app_data_dir() {
                        let logs_dir = data_dir.join("logs");
                        open_directory(&logs_dir);
                    }
                }
                "quit" => {
                    let app_handle = app.clone();
                    tauri::async_runtime::spawn(async move {
                        if let Some(manager) = app_handle.try_state::<Arc<ServerManager>>() {
                            let _ = manager.stop().await;
                        }
                        app_handle.exit(0);
                    });
                }
                _ => {}
            })
            .on_tray_icon_event(|tray, event| {
                // Show window on left click
                if let TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                } = event
                {
                    if let Some(window) = tray.app_handle().get_webview_window("main") {
                        window.show().ok();
                        window.set_focus().ok();
                    }
                }
            })
            .build(app.app_handle())?;

        Ok(Arc::new(Self { status_item_id }))
    }

    /// Update tray status text based on server state.
    pub fn update_status<R: Runtime>(&self, app: &AppHandle<R>, state: &ServerState) {
        let (status_text, tooltip) = match state {
            ServerState::Stopped => (
                "Status: Stopped".to_string(),
                "Project Manager - Stopped".to_string(),
            ),
            ServerState::Starting => (
                "Status: Starting...".to_string(),
                "Project Manager - Starting...".to_string(),
            ),
            ServerState::Running { port } => (
                format!("Status: Running (port {})", port),
                format!("Project Manager - Running on port {}", port),
            ),
            ServerState::Restarting { attempt } => (
                format!("Status: Restarting (attempt {})", attempt),
                format!("Project Manager - Restarting (attempt {})", attempt),
            ),
            ServerState::ShuttingDown => (
                "Status: Shutting down...".to_string(),
                "Project Manager - Shutting down...".to_string(),
            ),
            ServerState::Failed { error } => (
                "Status: Failed".to_string(),
                format!("Project Manager - Failed: {}", error),
            ),
        };

        // Update menu item text
        if let Some(menu) = app.menu() {
            if let Some(item) = menu.get(&self.status_item_id) {
                if let Some(menu_item) = item.as_menuitem() {
                    let _ = menu_item.set_text(&status_text);
                }
            }
        }

        // Update tray tooltip
        if let Some(tray) = app.tray_by_id("main") {
            let _ = tray.set_tooltip(Some(&tooltip));
        }

        tracing::debug!("Tray status updated: {}", status_text);
    }
}

/// Open a directory in the system file manager.
fn open_directory(path: &std::path::Path) {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open").arg(path).spawn().ok();
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(path)
            .spawn()
            .ok();
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(path)
            .spawn()
            .ok();
    }
}
