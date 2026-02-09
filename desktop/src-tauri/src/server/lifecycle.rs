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

use error_location::ErrorLocation;
use tauri::async_runtime::Mutex;
use tokio::sync::{mpsc, watch};
use tracing::{error, info, warn};

/// Manages the pm-server process lifecycle.
///
/// Responsibilities:
/// - Start server process as standalone detached process
/// - Monitor health and trigger restarts
/// - Handle graceful shutdown
/// - Maintain lock file
pub struct ServerManager {
    config: ServerConfig,
    server_dir: PathBuf,
    tauri_dir: PathBuf,
    server_pid: Arc<Mutex<Option<u32>>>,
    health_checker: Arc<Mutex<Option<HealthChecker>>>,
    lock_file: Arc<Mutex<Option<LockFile>>>,
    state_tx: watch::Sender<ServerState>,
    state_rx: watch::Receiver<ServerState>,
    restart_count: Arc<AtomicU32>,
    shutdown_requested: Arc<AtomicBool>,
    actual_port: Arc<Mutex<Option<u16>>>,
    command_tx: mpsc::Sender<ServerCommand>,
    command_rx: Arc<Mutex<mpsc::Receiver<ServerCommand>>>,
}

impl ServerManager {
    /// Create a new server manager.
    pub fn new(server_dir: PathBuf, tauri_dir: PathBuf, config: ServerConfig) -> Self {
        let (state_tx, state_rx) = watch::channel(ServerState::Stopped);
        let (command_tx, command_rx) = mpsc::channel(16);

        Self {
            config,
            server_dir,
            tauri_dir,
            server_pid: Arc::new(Mutex::new(None)),
            health_checker: Arc::new(Mutex::new(None)),
            lock_file: Arc::new(Mutex::new(None)),
            state_tx,
            state_rx,
            restart_count: Arc::new(AtomicU32::new(0)),
            shutdown_requested: Arc::new(AtomicBool::new(false)),
            actual_port: Arc::new(Mutex::new(None)),
            command_tx,
            command_rx: Arc::new(Mutex::new(command_rx)),
        }
    }

    /// Find the pm-server binary in development or bundled locations.
    /// Find the pm-server binary.
    ///
    /// Search order:
    /// 1. Sibling to current exe (bundled production + dev builds)
    /// 2. Installed at <repo>/.pm/bin/pm-server
    /// 3. System PATH
    fn find_server_binary(&self) -> ServerResult<PathBuf> {
        // 1. Sibling to current executable
        if let Ok(exe) = std::env::current_exe()
            && let Some(exe_dir) = exe.parent()
        {
            let sibling = exe_dir.join("pm-server");
            if sibling.exists() {
                info!("Using pm-server (sibling): {}", sibling.display());
                return Ok(sibling);
            }
        }

        // 2. Installed location: <repo>/.pm/bin/pm-server
        let installed = self.server_dir.join("bin").join("pm-server");
        if installed.exists() {
            info!("Using pm-server (installed): {}", installed.display());
            return Ok(installed);
        }

        // 3. System PATH
        if let Ok(output) = std::process::Command::new("which")
            .arg("pm-server")
            .output()
            && output.status.success()
        {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                info!("Using pm-server (PATH): {}", path);
                return Ok(PathBuf::from(path));
            }
        }

        Err(ServerError::ProcessSpawn {
            source: std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "pm-server binary not found. Build it with `just build-rs-server` or install via install.sh",
            )
                .into(),
            location: ErrorLocation::from(Location::caller()),
        })
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

        info!("Using port {port}");

        // Acquire lock file
        let lock = LockFile::acquire(&self.server_dir, port)?;
        *self.lock_file.lock().await = Some(lock);

        // Spawn the server process and get ready signal receiver
        let ready_rx = self.spawn_process(app, port).await?;

        // Store actual port
        *self.actual_port.lock().await = Some(port);

        // Create health checker
        let health_checker = HealthChecker::new(port, 3);
        *self.health_checker.lock().await = Some(health_checker);

        // Wait for server to signal it's ready (with timeout)
        let timeout_secs = self.config.resilience.startup_timeout_secs;
        let ready_result = tokio::time::timeout(Duration::from_secs(timeout_secs), ready_rx).await;

        match ready_result {
            Ok(Ok(())) => {
                info!("Server signaled ready");
            }
            Ok(Err(_)) => {
                return Err(ServerError::StartupFailed {
                    message: "Ready signal channel closed unexpectedly".into(),
                    location: ErrorLocation::from(Location::caller()),
                });
            }
            Err(_) => {
                return Err(ServerError::StartupTimeout {
                    timeout_secs,
                    location: ErrorLocation::from(Location::caller()),
                });
            }
        }

        // Verify with a single health check
        {
            let checker = self.health_checker.lock().await;
            if let Some(ref hc) = *checker {
                match hc.check().await {
                    HealthStatus::Healthy { .. } => {
                        info!("Health check confirmed server is ready");
                    }
                    status => {
                        warn!("Server signaled ready but health check returned: {status:?}");
                    }
                }
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

    /// Spawn pm-server as a standalone detached process.
    async fn spawn_process(
        &self,
        _app: &tauri::AppHandle,
        port: u16,
    ) -> ServerResult<tokio::sync::oneshot::Receiver<()>> {
        info!(
            "Spawning standalone pm-server from {}",
            self.server_dir.display()
        );

        // Find pm-server binary
        let server_binary = self.find_server_binary()?;
        info!("Using pm-server at: {}", server_binary.display());

        // Prepare log file path (in server_dir/logs/)
        let log_file = self
            .server_dir
            .join(&self.config.logging.directory)
            .join("pm-server.log");

        // Ensure logs directory exists
        if let Some(log_dir) = log_file.parent() {
            std::fs::create_dir_all(log_dir).map_err(|e| ServerError::ProcessSpawn {
                source: std::io::Error::other(format!("Failed to create logs directory: {e}"))
                    .into(),
                location: ErrorLocation::from(Location::caller()),
            })?;
        }

        // Spawn as detached process
        let mut cmd = std::process::Command::new(&server_binary);
        // Set cwd to repo root so pm-server's git-based config_dir() works.
        // self.server_dir is <repo>/.pm/, so parent is the repo root.
        cmd.current_dir(self.server_dir.parent().unwrap_or(&self.server_dir))
            .env("PM_SERVER_PORT", port.to_string())
            .env("PM_SERVER_HOST", &self.config.server.host)
            .env("PM_LOG_LEVEL", &self.config.logging.level)
            .env("PM_LOG_FILE", log_file.to_str().unwrap())
            .env(
                "PM_IDLE_SHUTDOWN_SECS",
                self.config.connection.idle_shutdown_secs.to_string(),
            )
            .env("PM_AUTH_ENABLED", "false"); // Desktop mode = no auth

        // Detach on Unix
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            unsafe {
                cmd.pre_exec(|| {
                    libc::setsid();
                    Ok(())
                });
            }
        }

        // Close stdio - server logs to file via PM_LOG_FILE
        cmd.stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());

        let child = cmd.spawn().map_err(|e| ServerError::ProcessSpawn {
            source: std::io::Error::other(format!("Failed to spawn pm-server: {}", e)).into(),
            location: ErrorLocation::from(Location::caller()),
        })?;

        let pid = child.id();
        info!("Spawned standalone pm-server with PID: {}", pid);

        // Store PID for tracking
        *self.server_pid.lock().await = Some(pid);

        // Don't store child handle - it's detached
        drop(child);

        // Create channel and poll /ready endpoint
        let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
        let ready_port = port;

        tokio::spawn(async move {
            let timeout = Duration::from_secs(30);
            let start = Instant::now();

            while start.elapsed() < timeout {
                tokio::time::sleep(Duration::from_millis(200)).await;

                if let Ok(client) = reqwest::Client::builder()
                    .timeout(Duration::from_millis(1000))
                    .build()
                {
                    let url = format!("http://127.0.0.1:{}/ready", ready_port);
                    if let Ok(response) = client.get(&url).send().await
                        && response.status().is_success()
                    {
                        info!("Server readiness check passed");
                        let _ = ready_tx.send(());
                        return;
                    }
                }
            }
            warn!("Server readiness check timed out after 30s");
        });

        Ok(ready_rx)
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

                            warn!("Server unhealthy, requesting restart {count}/{max_restarts}");

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
    fn start_command_handler(&self, _app: tauri::AppHandle) {
        let command_rx = self.command_rx.clone();
        let server_pid = self.server_pid.clone();
        let health_checker = self.health_checker.clone();
        let state_tx = self.state_tx.clone();
        let config = self.config.clone();
        let server_dir = self.server_dir.clone();
        let actual_port = self.actual_port.clone();
        let shutdown_requested = self.shutdown_requested.clone();
        let manager_find_binary = self.find_server_binary();

        tauri::async_runtime::spawn(async move {
            let mut rx = command_rx.lock().await;

            // Get binary path once at start
            let server_binary = match manager_find_binary {
                Ok(path) => path,
                Err(e) => {
                    error!("Failed to find server binary for restarts: {e}");
                    return;
                }
            };

            while let Some(cmd) = rx.recv().await {
                if shutdown_requested.load(Ordering::SeqCst) {
                    break;
                }

                match cmd {
                    ServerCommand::Restart { attempt } => {
                        info!("Processing restart request, attempt {attempt}");
                        let _ = state_tx.send(ServerState::Restarting { attempt });

                        // Kill existing process by PID
                        {
                            let pid_guard = server_pid.lock().await;
                            if let Some(pid) = *pid_guard {
                                #[cfg(unix)]
                                {
                                    use nix::sys::signal::{Signal, kill};
                                    use nix::unistd::Pid;
                                    kill(Pid::from_raw(pid as i32), Signal::SIGKILL).ok();
                                }

                                #[cfg(windows)]
                                {
                                    std::process::Command::new("taskkill")
                                        .args(["/F", "/PID", &pid.to_string()])
                                        .output()
                                        .ok();
                                }
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
                                error!("Failed to find available port: {e}");
                                let _ = state_tx.send(ServerState::Failed {
                                    error: e.to_string(),
                                });
                                continue;
                            }
                        };

                        // Prepare log file path
                        let log_file = server_dir
                            .join(&config.logging.directory)
                            .join("pm-server.log");

                        // Spawn new process
                        let mut cmd = std::process::Command::new(&server_binary);
                        cmd.current_dir(server_dir.parent().unwrap_or(&server_dir))
                            .env("PM_SERVER_PORT", port.to_string())
                            .env("PM_SERVER_HOST", &config.server.host)
                            .env("PM_LOG_LEVEL", &config.logging.level)
                            .env("PM_LOG_FILE", log_file.to_str().unwrap())
                            .env(
                                "PM_IDLE_SHUTDOWN_SECS",
                                config.connection.idle_shutdown_secs.to_string(),
                            )
                            .env("PM_AUTH_ENABLED", "false");

                        #[cfg(unix)]
                        {
                            use std::os::unix::process::CommandExt;
                            unsafe {
                                cmd.pre_exec(|| {
                                    libc::setsid();
                                    Ok(())
                                });
                            }
                        }

                        cmd.stdin(std::process::Stdio::null())
                            .stdout(std::process::Stdio::null())
                            .stderr(std::process::Stdio::null());

                        match cmd.spawn() {
                            Ok(child) => {
                                let new_pid = child.id();
                                *server_pid.lock().await = Some(new_pid);
                                *actual_port.lock().await = Some(port);
                                drop(child); // Detached

                                // Update health checker port
                                *health_checker.lock().await = Some(HealthChecker::new(port, 3));

                                // Wait for ready
                                let hc = health_checker.lock().await;
                                if let Some(ref checker) = *hc {
                                    let timeout =
                                        Duration::from_secs(config.resilience.startup_timeout_secs);
                                    match checker.wait_ready(timeout).await {
                                        Ok(()) => {
                                            info!("Server restarted successfully on port {port}");
                                            let _ = state_tx.send(ServerState::Running { port });
                                        }
                                        Err(e) => {
                                            warn!(
                                                "Server failed to become ready after restart: {e}"
                                            );
                                            // Health monitor will detect and request another restart
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Failed to spawn process: {e}");
                                let _ = state_tx.send(ServerState::Failed {
                                    error: e.to_string(),
                                });
                            }
                        }
                    }
                    ServerCommand::MaxRestartsExceeded { count } => {
                        error!("Max restarts exceeded: {count} attempts");
                        let _ = state_tx.send(ServerState::Failed {
                            error: format!("Server crashed {count} times"),
                        });
                        break;
                    }
                }
            }
        });
    }

    /// Ensure data directory structure exists.
    fn ensure_data_dir(&self) -> ServerResult<()> {
        std::fs::create_dir_all(&self.server_dir).map_err(|e| ServerError::DataDirCreation {
            path: self.server_dir.clone(),
            source: e,
            location: ErrorLocation::from(Location::caller()),
        })?;

        let logs_dir = self.server_dir.join(&self.config.logging.directory);
        std::fs::create_dir_all(&logs_dir).map_err(|e| ServerError::DataDirCreation {
            path: logs_dir,
            source: e,
            location: ErrorLocation::from(Location::caller()),
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
            .map(|p| format!("ws://127.0.0.1:{p}/ws"))
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

    /// Get server process PID (if running).
    pub async fn server_pid(&self) -> Option<u32> {
        *self.server_pid.lock().await
    }

    /// Get tauri data directory.
    #[allow(dead_code)]
    pub fn tauri_dir(&self) -> &PathBuf {
        &self.tauri_dir
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
        if self.config.database.checkpoint_on_shutdown
            && let Err(e) = self.checkpoint_database().await
        {
            warn!("Failed to checkpoint database: {e}");
        }

        // Kill server process if we have a PID
        let pid_guard = self.server_pid.lock().await;
        if let Some(pid) = *pid_guard {
            drop(pid_guard); // Release lock before async operations

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

                    info!("Sending SIGTERM to pid {pid}");
                    kill(Pid::from_raw(pid as i32), Signal::SIGTERM).ok();
                }

                #[cfg(windows)]
                {
                    use windows_sys::Win32::System::Console::{
                        CTRL_BREAK_EVENT, GenerateConsoleCtrlEvent,
                    };

                    info!("Sending CTRL_BREAK to pid {pid}");
                    unsafe {
                        GenerateConsoleCtrlEvent(CTRL_BREAK_EVENT, pid);
                    }
                }
            }

            // Wait for process to exit with timeout
            let start = Instant::now();
            let poll_interval = Duration::from_millis(100);

            while start.elapsed() < timeout {
                if let Ok(client) = reqwest::Client::builder()
                    .timeout(Duration::from_millis(500))
                    .build()
                {
                    let url = format!("http://127.0.0.1:{}/health", port);
                    if client.get(&url).send().await.is_err() {
                        info!("Server stopped responding, shutdown complete");
                        break;
                    }
                }
                tokio::time::sleep(poll_interval).await;
            }

            // Force kill if still running after timeout
            info!("Force killing server process (PID: {})", pid);

            #[cfg(unix)]
            {
                use nix::sys::signal::{Signal, kill};
                use nix::unistd::Pid;

                kill(Pid::from_raw(pid as i32), Signal::SIGKILL).ok();
            }

            #[cfg(windows)]
            {
                std::process::Command::new("taskkill")
                    .args(["/F", "/PID", &pid.to_string()])
                    .output()
                    .ok();
            }

            // Clear stored PID
            *self.server_pid.lock().await = None;
        }

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
        let url = format!("http://127.0.0.1:{port}/admin/checkpoint");

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| ServerError::CheckpointFailed {
                message: e.to_string(),
                location: ErrorLocation::from(Location::caller()),
            })?;

        let resp = client
            .post(&url)
            .send()
            .await
            .map_err(|e| ServerError::CheckpointFailed {
                message: e.to_string(),
                location: ErrorLocation::from(Location::caller()),
            })?;

        if !resp.status().is_success() {
            return Err(ServerError::CheckpointFailed {
                message: format!("HTTP {}", resp.status()),
                location: ErrorLocation::from(Location::caller()),
            });
        }

        info!("Database checkpoint completed");
        Ok(())
    }

    /// Request graceful shutdown via HTTP endpoint.
    async fn request_graceful_shutdown(&self, port: u16) -> bool {
        let url = format!("http://127.0.0.1:{port}/admin/shutdown");

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
                    warn!("Failed to send shutdown request: {e}");
                }
            }
        }

        false
    }
}
