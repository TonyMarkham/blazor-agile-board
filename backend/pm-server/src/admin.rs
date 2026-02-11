//! Administrative endpoints for server management.

use axum::{Json, extract::State, http::StatusCode};
use log::info;
use pm_ws::AppState;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct CheckpointResponse {
    pub status: String,
    pub message: String,
}

/// Checkpoint WAL to main database file.
///
/// This forces SQLite to flush the Write-Ahead Log to the main database file,
/// ensuring durability before shutdown.
pub async fn checkpoint_handler(
    State(state): State<AppState>,
) -> Result<Json<CheckpointResponse>, (StatusCode, String)> {
    info!("Manual checkpoint requested");

    sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)")
        .execute(&state.pool)
        .await
        .map_err(|e| {
            log::error!("Checkpoint failed: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        })?;

    info!("Database checkpoint completed");

    Ok(Json(CheckpointResponse {
        status: "ok".to_string(),
        message: "Database checkpoint completed".to_string(),
    }))
}

/// Graceful shutdown endpoint.
///
/// Triggers immediate graceful shutdown of the server.
pub async fn shutdown_handler(
    State(state): State<AppState>,
) -> Result<StatusCode, (StatusCode, String)> {
    info!("Graceful shutdown requested via HTTP");

    // Trigger shutdown via coordinator
    state.shutdown.shutdown();

    Ok(StatusCode::ACCEPTED)
}
