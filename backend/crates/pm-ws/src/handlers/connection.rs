use crate::{Result as WsResult, WsError};

use pm_auth::Claims;

use std::collections::HashMap;
use std::env;
use std::panic::Location;

use error_location::ErrorLocation;
use tracing::{error, warn};
use uuid::Uuid;

/// Extracts user identity from WebSocket connection.
///
/// # Security Model
/// - When auth disabled (desktop mode): Accepts user_id from query params
/// - When auth enabled (web mode): REJECTS user_id param, requires JWT
///
/// This prevents impersonation attacks in multi-user environments.
pub fn extract_user_id(
    query_params: &HashMap<String, String>,
    jwt_claims: Option<&Claims>,
) -> WsResult<Uuid> {
    let auth_enabled = env::var("PM_AUTH_ENABLED")
        .map(|v| v.to_lowercase() == "true")
        .unwrap_or(false);

    // SECURITY: Reject user_id param when auth is enabled
    if auth_enabled && query_params.contains_key("user_id") {
        error!("Attempted user_id bypass with auth enabled");
        return Err(WsError::Unauthorized {
            message: "user_id parameter not allowed when authentication is enabled".into(),
            location: ErrorLocation::from(Location::caller()),
        });
    }

    // Auth enabled: require valid JWT
    if auth_enabled {
        return jwt_claims
            .and_then(|c| Uuid::parse_str(&c.sub).ok())
            .ok_or_else(|| WsError::Unauthorized {
                message: "Valid JWT required".into(),
                location: ErrorLocation::from(Location::caller()),
            });
    }

    // Auth disabled (desktop mode): use user_id from query params
    if let Some(id_str) = query_params.get("user_id") {
        Uuid::parse_str(id_str).map_err(|_| WsError::InvalidMessage {
            message: format!("Invalid user_id format: {id_str}"),
            location: ErrorLocation::from(Location::caller()),
        })
    } else {
        // Fallback for legacy clients - generate session ID
        warn!("No user_id provided in desktop mode, generating session ID");
        Ok(Uuid::new_v4())
    }
}
