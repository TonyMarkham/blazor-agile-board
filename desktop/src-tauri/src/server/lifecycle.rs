//! Server process lifecycle with crash recovery.

use crate::server::{
    HealthChecker, HealthStatus, LockFile, PortManager, ServerCommand, ServerConfig, ServerError,
    ServerResult, ServerState,
};

use std::panic::Location;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::{Duration, Instant};

use tauri::async_runtime::Mutex;
use tauri_plugin_shell::ShellExt;
use tauri_plugin_shell::process::CommandChild;
use tokio::sync::{mpsc, watch};
use tracing::{debug, error, info, warn};

/// Manages the pm-server process lifecycle.
///
/// Responsibilities:
/// - Start server process as Tauri sidecar
/// - Monitor health and trigger restarts
/// - Handle graceful shutdown
/// - Maintain lock file
pub struct ServerManager {
    config: ServerConfig,
    data_dir: PathBuf,
    process: Arc<Mutex<Option<CommandChild>>>,
    health_checker: Arc<Mutex<Option<HealthChecker>>>,
    lock_file: Arc<Mutex<Option<LockFile>>>,
    state_tx: watch::Sender<ServerState>,
    state_rx: watch::Receiver<ServerState>,
    restart_count: Arc<AtomicU32>,
    restart_window_start: Arc<Mutex<Option<Instant>>>,
    shutdown_requested: Arc<AtomicBool>,
    actual_port: Arc<Mutex<Option<u16>>>,
    command_tx: mpsc::Sender<ServerCommand>,
    command_rx: Arc<Mutex<mpsc::Receiver<ServerCommand>>>,
}

impl ServerManager {
    /// Create a new server manager.
    pub fn new(data_dir: PathBuf, config: ServerConfig) -> Self {
        let (state_tx, state_rx) = watch::channel(ServerState::Stopped);
        let (command_tx, command_rx) = mpsc::channel(16);

        Self {
            config,
            data_dir,
            process: Arc::new(Mutex::new(None)),
            health_checker: Arc::new(Mutex::new(None)),
            lock_file: Arc::new(Mutex::new(None)),
            state_tx,
            state_rx,
            restart_count: Arc::new(AtomicU32::new(0)),
            restart_window_start: Arc::new(Mutex::new(None)),
            shutdown_requested: Arc::new(AtomicBool::new(false)),
            actual_port: Arc::new(Mutex::new(None)),
            command_tx,
            command_rx: Arc::new(Mutex::new(command_rx)),
        }
    }

    /// Start the server and wait for it to be ready.
    pub async fn start(&self, app: &tauri::AppHandle) -> ServerResult<()> {
        self.shutdown_requested.store(false, Ordering::SeqCst);

        // Update state
        self.set_state(ServerState::Starting);

        // Ensure data directory exists
        self.ensure_data_dir()?;

        // Find available port
        let port =
            PortManager::find_available(self.config.server.port, self.config.server.port_range)?;

        info!("Using port {}", port);

        // Acquire lock file
        let lock = LockFile::acquire(&self.data_dir, port)?;
        *self.lock_file.lock().await = Some(lock);

        // Spawn the server process
        self.spawn_process(app, port).await?;

        // Store actual port
        *self.actual_port.lock().await = Some(port);

        // Create health checker
        let health_checker = HealthChecker::new(port, 3);
        *self.health_checker.lock().await = Some(health_checker);

        // Wait for server to be ready
        let timeout = Duration::from_secs(self.config.resilience.startup_timeout_secs);
        {
            let checker = self.health_checker.lock().await;
            if let Some(ref hc) = *checker {
                hc.wait_ready(timeout).await?;
            }
        }

        // Update state
        self.set_state(ServerState::Running { port });

        info!("Server started successfully on port {port}");

        // Start background health monitoring
        self.start_health_monitor();

        // Start command handler
        self.start_command_handler(app.clone());

        Ok(())
    }

    /// Spawn the pm-server sidecar process.
    async fn spawn_process(&self, app: &tauri::AppHandle, port: u16) -> ServerResult<()> {
        let sidecar = app
            .shell()
            .sidecar("pm-server")
            .map_err(|e| ServerError::ProcessSpawn {
                source: e,
                location: error_location::ErrorLocation::from(Location::caller()),
            })?
            .env("PM_CONFIG_DIR", self.data_dir.to_str().unwrap())
            .env("PM_SERVER_PORT", port.to_string())
            .env("PM_SERVER_HOST", &self.config.server.host)
            .env("PM_LOG_LEVEL", &self.config.logging.level)
            .env("PM_AUTH_ENABLED", "false"); // Desktop mode = no auth

        let (mut rx, child) = sidecar.spawn().map_err(|e| ServerError::ProcessSpawn {
            source: e,
            location: error_location::ErrorLocation::from(Location::caller()),
        })?;

        // Handle process output in background task
        let _data_dir = self.data_dir.clone();
        tauri::async_runtime::spawn(async move {
            use tauri_plugin_shell::process::CommandEvent;

            while let Some(event) = rx.recv().await {
                match event {
                    CommandEvent::Stdout(line) => {
                        debug!("pm-server: {}", String::from_utf8_lossy(&line));
                    }
                    CommandEvent::Stderr(line) => {
                        let msg = String::from_utf8_lossy(&line);
                        if msg.contains("ERROR") || msg.contains("WARN") {
                            warn!("pm-server: {}", msg);
                        } else {
                            debug!("pm-server: {}", msg);
                        }
                    }
                    CommandEvent::Error(e) => {
                        error!("pm-server error: {}", e);
                    }
                    CommandEvent::Terminated(payload) => {
                        info!(
                            "pm-server terminated with code {:?}, signal {:?}",
                            payload.code, payload.signal
                        );
                    }
                    _ => {}
                }
            }
        });

        *self.process.lock().await = Some(child);

        Ok(())
    }

    /// Start background health monitoring task.
    fn start_health_monitor(&self) {
        let health_checker = self.health_checker.clone();
        let shutdown_requested = self.shutdown_requested.clone();
        let interval = Duration::from_secs(self.config.resilience.health_check_interval_secs);
        let restart_count = self.restart_count.clone();
        let max_restarts = self.config.resilience.max_restarts;
        let command_tx = self.command_tx.clone();

        tauri::async_runtime::spawn(async move {
            loop {
                tokio::time::sleep(interval).await;

                if shutdown_requested.load(Ordering::SeqCst) {
                    break;
                }

                let checker = health_checker.lock().await;
                if let Some(ref hc) = *checker {
                    let status = hc.check().await;
                    drop(checker); // Release lock before potential long operations

                    match status {
                        HealthStatus::Healthy { .. } => {
                            // Reset restart count on healthy status
                            restart_count.store(0, Ordering::SeqCst);
                        }
                        HealthStatus::Unhealthy {
                            consecutive_failures,
                            ..
                        } if consecutive_failures >= 3 => {
                            let count = restart_count.fetch_add(1, Ordering::SeqCst) + 1;

                            if count > max_restarts {
                                let _ = command_tx
                                    .send(ServerCommand::MaxRestartsExceeded { count })
                                    .await;
                                break;
                            }

                            warn!(
                                "Server unhealthy, requesting restart {}/{}",
                                count, max_restarts
                            );

                            let _ = command_tx
                                .send(ServerCommand::Restart { attempt: count })
                                .await;
                        }
                        _ => {}
                    }
                }
            }
        });
    }

    /// Start command handler that processes restart requests.
    fn start_command_handler(&self, app: tauri::AppHandle) {
        let command_rx = self.command_rx.clone();
        let process = self.process.clone();
        let health_checker = self.health_checker.clone();
        let state_tx = self.state_tx.clone();
        let config = self.config.clone();
        let data_dir = self.data_dir.clone();
        let actual_port = self.actual_port.clone();
        let shutdown_requested = self.shutdown_requested.clone();

        tauri::async_runtime::spawn(async move {
            let mut rx = command_rx.lock().await;

            while let Some(cmd) = rx.recv().await {
                if shutdown_requested.load(Ordering::SeqCst) {
                    break;
                }

                match cmd {
                    ServerCommand::Restart { attempt } => {
                        info!("Processing restart request, attempt {}", attempt);
                        let _ = state_tx.send(ServerState::Restarting { attempt });

                        // Kill existing process
                        {
                            let mut proc_guard = process.lock().await;
                            if let Some(child) = proc_guard.take() {
                                child.kill().ok();
                            }
                        }

                        // Exponential backoff with cap
                        let backoff = std::cmp::min(
                            config.resilience.initial_backoff_ms * 2u64.pow(attempt - 1),
                            config.resilience.max_backoff_ms,
                        );
                        tokio::time::sleep(Duration::from_millis(backoff)).await;

                        // Find new port (previous might be stuck)
                        let port = match PortManager::find_available(
                            config.server.port,
                            config.server.port_range,
                        ) {
                            Ok(p) => p,
                            Err(e) => {
                                error!("Failed to find available port: {}", e);
                                let _ = state_tx.send(ServerState::Failed {
                                    error: e.to_string(),
                                });
                                continue;
                            }
                        };

                        // Spawn new process
                        let sidecar = match app.shell().sidecar("pm-server").map(|s| {
                            s.env("PM_CONFIG_DIR", data_dir.to_str().unwrap())
                                .env("PM_SERVER_PORT", port.to_string())
                                .env("PM_SERVER_HOST", &config.server.host)
                                .env("PM_LOG_LEVEL", &config.logging.level)
                                .env("PM_AUTH_ENABLED", "false")
                        }) {
                            Ok(s) => s,
                            Err(e) => {
                                error!("Failed to create sidecar: {}", e);
                                let _ = state_tx.send(ServerState::Failed {
                                    error: e.to_string(),
                                });
                                continue;
                            }
                        };

                        match sidecar.spawn() {
                            Ok((_rx, child)) => {
                                *process.lock().await = Some(child);
                                *actual_port.lock().await = Some(port);

                                // Update health checker port
                                *health_checker.lock().await = Some(HealthChecker::new(port, 3));

                                // Wait for ready
                                let hc = health_checker.lock().await;
                                if let Some(ref checker) = *hc {
                                    let timeout =
                                        Duration::from_secs(config.resilience.startup_timeout_secs);
                                    match checker.wait_ready(timeout).await {
                                        Ok(()) => {
                                            info!("Server restarted successfully on port {}", port);
                                            let _ = state_tx.send(ServerState::Running { port });
                                        }
                                        Err(e) => {
                                            warn!(
                                                "Server failed to become ready after restart: {}",
                                                e
                                            );
                                            // Health monitor will detect and request another restart
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Failed to spawn process: {}", e);
                                let _ = state_tx.send(ServerState::Failed {
                                    error: e.to_string(),
                                });
                            }
                        }
                    }
                    ServerCommand::MaxRestartsExceeded { count } => {
                        error!("Max restarts exceeded: {} attempts", count);
                        let _ = state_tx.send(ServerState::Failed {
                            error: format!("Server crashed {} times", count),
                        });
                        break;
                    }
                }
            }
        });
    }

    /// Ensure data directory structure exists.
    fn ensure_data_dir(&self) -> ServerResult<()> {
        std::fs::create_dir_all(&self.data_dir).map_err(|e| ServerError::DataDirCreation {
            path: self.data_dir.clone(),
            source: e,
            location: error_location::ErrorLocation::from(Location::caller()),
        })?;

        let logs_dir = self.data_dir.join(&self.config.logging.directory);
        std::fs::create_dir_all(&logs_dir).map_err(|e| ServerError::DataDirCreation {
            path: logs_dir,
            source: e,
            location: error_location::ErrorLocation::from(Location::caller()),
        })?;

        Ok(())
    }

    fn set_state(&self, state: ServerState) {
        let _ = self.state_tx.send(state);
    }

    /// Subscribe to state changes.
    pub fn subscribe(&self) -> watch::Receiver<ServerState> {
        self.state_rx.clone()
    }

    /// Get current state.
    pub async fn state(&self) -> ServerState {
        self.state_rx.borrow().clone()
    }

    /// Get the WebSocket URL for frontend connection.
    pub async fn websocket_url(&self) -> Option<String> {
        self.actual_port
            .lock()
            .await
            .map(|p| format!("ws://127.0.0.1:{}/ws", p))
    }

    /// Get current port (if running).
    pub async fn port(&self) -> Option<u16> {
        *self.actual_port.lock().await
    }

    /// Get health status.
    pub async fn health(&self) -> Option<HealthStatus> {
        if let Some(ref hc) = *self.health_checker.lock().await {
            Some(hc.status().await)
        } else {
            None
        }
    }

    /// Stop the server gracefully.
    pub async fn stop(&self) -> ServerResult<()> {
        self.shutdown_requested.store(true, Ordering::SeqCst);
        self.set_state(ServerState::ShuttingDown);

        // Update health status
        if let Some(ref hc) = *self.health_checker.lock().await {
            hc.set_status(HealthStatus::ShuttingDown).await;
        }

        // Checkpoint database before shutdown
        if self.config.database.checkpoint_on_shutdown {
            if let Err(e) = self.checkpoint_database().await {
                warn!("Failed to checkpoint database: {}", e);
            }
        }

        // Graceful shutdown with timeout
        let mut process_guard = self.process.lock().await;
        if let Some(child) = process_guard.take() {
            let timeout = Duration::from_secs(self.config.resilience.shutdown_timeout_secs);
            let port = self
                .actual_port
                .lock()
                .await
                .unwrap_or(self.config.server.port);

            // First, try HTTP shutdown endpoint
            let shutdown_success = self.request_graceful_shutdown(port).await;

            if !shutdown_success {
                // Fallback to OS-level signals
                #[cfg(unix)]
                {
                    use nix::sys::signal::{Signal, kill};
                    use nix::unistd::Pid;

                    let pid = child.pid();
                    info!("Sending SIGTERM to pid {}", pid);
                    kill(Pid::from_raw(pid as i32), Signal::SIGTERM).ok();
                }

                #[cfg(windows)]
                {
                    use windows_sys::Win32::System::Console::{
                        CTRL_BREAK_EVENT, GenerateConsoleCtrlEvent,
                    };

                    let pid = child.pid();
                    info!("Sending CTRL_BREAK to pid {}", pid);
                    unsafe {
                        GenerateConsoleCtrlEvent(CTRL_BREAK_EVENT, pid);
                    }
                }
            }

            // Wait for process to exit with timeout
            let start = Instant::now();
            let poll_interval = Duration::from_millis(100);

            while start.elapsed() < timeout {
                // Check if process has exited via health endpoint
                let client = reqwest::Client::builder()
                    .timeout(Duration::from_millis(500))
                    .build()
                    .ok();

                if let Some(client) = client {
                    let url = format!("http://127.0.0.1:{}/health", port);
                    if client.get(&url).send().await.is_err() {
                        info!("Server stopped responding, shutdown complete");
                        break;
                    }
                }

                tokio::time::sleep(poll_interval).await;
            }

            // Force kill if still running after timeout
            info!("Force killing server process");
            child.kill().ok();
        }
        drop(process_guard);

        // Release lock file
        if let Some(mut lock) = self.lock_file.lock().await.take() {
            lock.release();
        }

        self.set_state(ServerState::Stopped);
        info!("Server stopped");

        Ok(())
    }

    /// Checkpoint database WAL via HTTP endpoint.
    async fn checkpoint_database(&self) -> ServerResult<()> {
        let port = self
            .actual_port
            .lock()
            .await
            .unwrap_or(self.config.server.port);
        let url = format!("http://127.0.0.1:{}/admin/checkpoint", port);

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| ServerError::CheckpointFailed {
                message: e.to_string(),
                location: error_location::ErrorLocation::from(Location::caller()),
            })?;

        let resp = client
            .post(&url)
            .send()
            .await
            .map_err(|e| ServerError::CheckpointFailed {
                message: e.to_string(),
                location: error_location::ErrorLocation::from(Location::caller()),
            })?;

        if !resp.status().is_success() {
            return Err(ServerError::CheckpointFailed {
                message: format!("HTTP {}", resp.status()),
                location: error_location::ErrorLocation::from(Location::caller()),
            });
        }

        info!("Database checkpoint completed");
        Ok(())
    }

    /// Request graceful shutdown via HTTP endpoint.
    async fn request_graceful_shutdown(&self, port: u16) -> bool {
        let url = format!("http://127.0.0.1:{}/admin/shutdown", port);

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .ok();

        if let Some(client) = client {
            match client.post(&url).send().await {
                Ok(resp) if resp.status().is_success() => {
                    info!("Graceful shutdown request acknowledged");
                    return true;
                }
                Ok(resp) => {
                    warn!("Shutdown request returned HTTP {}", resp.status());
                }
                Err(e) => {
                    warn!("Failed to send shutdown request: {}", e);
                }
            }
        }

        false
    }
}
