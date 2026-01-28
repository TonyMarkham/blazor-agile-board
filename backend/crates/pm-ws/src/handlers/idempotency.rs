use crate::{Result as WsErrorResult, WsError};

use pm_db::IdempotencyRepository;
use pm_proto::WebSocketMessage;

use std::panic::Location;

use base64::Engine;
use error_location::ErrorLocation;
use log::warn;
use prost::Message;
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

/// Decode a cached protobuf response from base64-encoded string.
/// Returns the decoded WebSocketMessage.
pub fn decode_cached_response(cached_b64: &str) -> WsErrorResult<WebSocketMessage> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(cached_b64)
        .map_err(|e| WsError::Internal {
            message: format!("Failed to decode cached response: {e}"),
            location: ErrorLocation::from(Location::caller()),
        })?;

    WebSocketMessage::decode(&*bytes).map_err(|e| WsError::Internal {
        message: format!("Failed to decode cached protobuf: {e}"),
        location: ErrorLocation::from(Location::caller()),
    })
}

/// Store idempotency with non-fatal error handling.
/// Logs a warning if storage fails but doesn't propagate the error.
pub async fn store_idempotency_non_fatal(
    pool: &SqlitePool,
    message_id: &str,
    operation: &str,
    response: &WebSocketMessage,
) {
    let response_bytes = response.encode_to_vec();
    let response_b64 = base64::engine::general_purpose::STANDARD.encode(&response_bytes);

    if let Err(e) = store_idempotency(pool, message_id, operation, &response_b64).await {
        warn!("Failed to store idempotency for {operation} (non-fatal): {e}");
    }
}
