use crate::commands::build_server_status;
use crate::server::{HealthInfo, HealthStatus, ServerState};

#[test]
fn test_build_server_status_running_with_pid() {
    let state = ServerState::Running { port: 8080 };
    let health = HealthStatus::Healthy {
        latency_ms: 5,
        version: "0.1.0".into(),
    };

    let status = build_server_status(
        &state,
        Some(8080),
        Some("ws://127.0.0.1:8080/ws".into()),
        Some(&health),
        Some(12345),
    );

    assert_eq!(status.state, "running");
    assert_eq!(status.port, Some(8080));
    assert_eq!(status.websocket_url, Some("ws://127.0.0.1:8080/ws".into()));
    assert_eq!(status.pid, Some(12345));
    assert!(status.is_healthy);
    assert!(status.error.is_none());
    assert!(status.health.is_some());
}

#[test]
fn test_build_server_status_starting_no_pid() {
    let state = ServerState::Starting;

    let status = build_server_status(&state, None, None, None, None);

    assert_eq!(status.state, "starting");
    assert_eq!(status.port, None);
    assert_eq!(status.websocket_url, None);
    assert_eq!(status.pid, None);
    assert!(!status.is_healthy);
    assert!(status.error.is_none());
}

#[test]
fn test_build_server_status_failed_with_error() {
    let state = ServerState::Failed {
        error: "Port 8080 already in use".into(),
    };

    let status = build_server_status(&state, None, None, None, None);

    assert_eq!(status.state, "failed");
    assert_eq!(status.error, Some("Port 8080 already in use".into()));
    assert_eq!(
        status.recovery_hint,
        Some("Please check the logs or restart the application.".into())
    );
    assert!(!status.is_healthy);
}

#[test]
fn test_build_server_status_stopped() {
    let state = ServerState::Stopped;

    let status = build_server_status(&state, None, None, None, None);

    assert_eq!(status.state, "stopped");
    assert_eq!(status.pid, None);
    assert!(!status.is_healthy);
}

#[test]
fn test_build_server_status_restarting_with_attempt() {
    let state = ServerState::Restarting { attempt: 2 };

    let status = build_server_status(&state, Some(8080), None, None, Some(54321));

    assert_eq!(status.state, "restarting (attempt 2)");
    assert_eq!(status.port, Some(8080));
    assert_eq!(status.pid, Some(54321));
    assert!(!status.is_healthy);
}

#[test]
fn test_build_server_status_running_unhealthy() {
    let state = ServerState::Running { port: 8080 };
    let health = HealthStatus::Unhealthy {
        last_error: "Connection refused".into(),
        consecutive_failures: 3,
    };

    let status = build_server_status(
        &state,
        Some(8080),
        Some("ws://127.0.0.1:8080/ws".into()),
        Some(&health),
        Some(12345),
    );

    assert_eq!(status.state, "running");
    assert!(!status.is_healthy); // Unhealthy despite running state
    assert!(status.health.is_some());
}

#[test]
fn test_health_info_conversion() {
    let healthy = HealthStatus::Healthy {
        latency_ms: 10,
        version: "1.0.0".into(),
    };
    let info: HealthInfo = (&healthy).into();
    assert_eq!(info.status, "healthy");
    assert_eq!(info.latency_ms, Some(10));
    assert_eq!(info.version, Some("1.0.0".into()));

    let starting = HealthStatus::Starting;
    let info: HealthInfo = (&starting).into();
    assert_eq!(info.status, "starting");
    assert_eq!(info.latency_ms, None);

    let crashed = HealthStatus::Crashed { exit_code: Some(1) };
    let info: HealthInfo = (&crashed).into();
    assert_eq!(info.status, "crashed (code: Some(1))");
}
