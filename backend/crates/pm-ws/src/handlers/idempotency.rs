use crate::{Result as WsErrorResult, WsError};

use pm_db::IdempotencyRepository;

use std::panic::Location;

use error_location::ErrorLocation;
use sqlx::SqlitePool;

/// Check if a message has already been processed.                                                     
/// Returns Some(cached_result) if replay, None if new request.                                        
pub async fn check_idempotency(
    pool: &SqlitePool,
    message_id: &str,
) -> WsErrorResult<Option<String>> {
    let repo = IdempotencyRepository::new(pool.clone());
    repo.find_by_message_id(message_id)
        .await
        .map_err(|e| WsError::Internal {
            message: format!("Failed to check idempotency: {e}"),
            location: ErrorLocation::from(Location::caller()),
        })
}

/// Store the result of a successful operation for idempotency.                                        
pub async fn store_idempotency(
    pool: &SqlitePool,
    message_id: &str,
    operation: &str,
    result_json: &str,
) -> WsErrorResult<()> {
    let repo = IdempotencyRepository::new(pool.clone());
    repo.create(message_id, operation, result_json)
        .await
        .map_err(|e| WsError::Internal {
            message: format!("Failed to store idempotency: {e}"),
            location: ErrorLocation::from(Location::caller()),
        })
}
