//! Axum extractors for REST API authentication

use crate::ApiError;

use pm_ws::AppState;

use std::future::Future;

use axum::{extract::FromRequestParts, http::request::Parts};
use uuid::Uuid;

/// Extracts the user ID from the request
///
/// Checks for `X-User-Id` header first. If not present, falls back to
/// the configured LLM user ID from api_config.
pub struct UserId(pub Uuid);

impl FromRequestParts<AppState> for UserId {
    type Rejection = ApiError;

    #[allow(clippy::manual_async_fn)]
    fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        async move {
            let headers = &parts.headers;

            // Try X-User-Id header first
            #[allow(clippy::collapsible_if)]
            if let Some(header_value) = headers.get("X-User-Id") {
                if let Ok(user_id_str) = header_value.to_str() {
                    if let Ok(uuid) = Uuid::parse_str(user_id_str) {
                        log::debug!("Using user ID from X-User-Id header: {}", uuid);
                        return Ok(UserId(uuid));
                    }
                    log::warn!("Invalid UUID in X-User-Id header: {}", user_id_str);
                }
            }

            // Fall back to configured LLM user ID
            let llm_user_id = state.api_config.llm_user_uuid();
            log::debug!("Using default LLM user ID: {}", llm_user_id);

            Ok(UserId(llm_user_id))
        }
    }
}
