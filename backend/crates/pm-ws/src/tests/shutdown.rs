use crate::{ShutdownCoordinator, ShutdownGuard};

use tokio::time::{timeout, Duration};

#[tokio::test]
async fn given_coordinator_when_shutdown_triggered_then_subscribers_notified() {
    let coordinator = ShutdownCoordinator::new();
    let mut guard = ShutdownGuard::new(&coordinator);

    // Spawn task to trigger shutdown
    let coord_clone = coordinator.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(10)).await;
        coord_clone.shutdown();
    });

    // Wait for shutdown with timeout
    let result = timeout(Duration::from_millis(100), guard.wait()).await;
    assert!(result.is_ok(), "Shutdown signal should be received");
}

#[tokio::test]
async fn given_multiple_subscribers_when_shutdown_then_all_notified() {
    let coordinator = ShutdownCoordinator::new();
    let mut guard1 = ShutdownGuard::new(&coordinator);
    let mut guard2 = ShutdownGuard::new(&coordinator);

    coordinator.shutdown();

    let result1 = timeout(Duration::from_millis(10), guard1.wait()).await;
    let result2 = timeout(Duration::from_millis(10), guard2.wait()).await;

    assert!(result1.is_ok());
    assert!(result2.is_ok());
}

#[test]
fn given_new_coordinator_when_checked_then_not_shutdown() {
    let coordinator = ShutdownCoordinator::new();
    let mut guard = ShutdownGuard::new(&coordinator);

    assert!(!guard.poll_shutdown());
}