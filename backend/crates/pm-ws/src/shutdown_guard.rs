use crate::ShutdownCoordinator;

use tokio::sync::broadcast;

/// Helper for gracefully handling shutdown in async tasks
pub struct ShutdownGuard {
    shutdown_rx: broadcast::Receiver<()>,
}

impl ShutdownGuard {
    pub fn new(coordinator: &ShutdownCoordinator) -> Self {
        Self {
            shutdown_rx: coordinator.subscribe(),
        }
    }

    /// Wait for shutdown signal
    pub async fn wait(&mut self) {
        let _ = self.shutdown_rx.recv().await;
    }

    /// Poll for shutdown signal (non-blocking, consumes signal if present)
    pub fn poll_shutdown(&mut self) -> bool {
        matches!(self.shutdown_rx.try_recv(), Ok(_))
    }
}