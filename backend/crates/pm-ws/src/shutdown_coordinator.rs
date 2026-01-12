use crate::ShutdownGuard;

use tokio::sync::broadcast;

/// Graceful shutdown coordinator
#[derive(Clone)]
pub struct ShutdownCoordinator {
    shutdown_tx: broadcast::Sender<()>,
}

impl ShutdownCoordinator {
    /// Create a new shutdown coordinator
    pub fn new() -> Self {
        let (shutdown_tx, _) = broadcast::channel(1);
        Self { shutdown_tx }
    }

    /// Get a receiver for shutdown notifications
    pub fn subscribe(&self) -> broadcast::Receiver<()> {
        self.shutdown_tx.subscribe()
    }

    /// Trigger shutdown (call this from signal handler)
    pub fn shutdown(&self) {
        log::info!("Shutdown signal received, notifying all subsystems");
        let _ = self.shutdown_tx.send(());
    }

    /// Check if shutdown has been triggered (non-blocking)
    pub fn is_shutdown(&self) -> bool {
        self.shutdown_tx.receiver_count() == 0
    }

    /// Convenience method to create a guard (used in handlers)
    pub fn subscribe_guard(&self) -> ShutdownGuard {
        ShutdownGuard::new(self)
    }
}

impl Default for ShutdownCoordinator {
    fn default() -> Self {
        Self::new()
    }
}