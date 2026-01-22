//! Port allocation and availability checking.

use crate::server::{ServerError, ServerResult};

use std::panic::Location;

use error_location::ErrorLocation;

const PROTOCOL: &str = "http";
const HOST: &str = "127.0.0.1";
const HEALTH_ENDPOINT: &str = "health";
const SERVER_IDENTIFIER: &str = "pm-server";
const HEALTH_CHECK_TIMEOUT_SECS: u64 = 1;

pub struct PortManager;

impl PortManager {
    /// Find an available port, preferring the given port.
    ///
    /// Algorithm:
    /// 1. Try preferred port first
    /// 2. If unavailable, scan range sequentially
    /// 3. Return first available port
    pub fn find_available(preferred: u16, range: (u16, u16)) -> ServerResult<u16> {
        // Try preferred port first
        if Self::is_available(preferred) {
            return Ok(preferred);
        }

        // Search in range
        for port in range.0..=range.1 {
            if port != preferred && Self::is_available(port) {
                return Ok(port);
            }
        }

        Err(ServerError::NoAvailablePort {
            start: range.0,
            end: range.1,
            location: ErrorLocation::from(Location::caller()),
        })
    }

    /// Check if a port is available for binding.
    ///
    /// Attempts to bind to 127.0.0.1:port. If successful,
    /// the port is available. The socket is immediately
    /// released when the listener is dropped.
    pub fn is_available(port: u16) -> bool {
        std::net::TcpListener::bind((HOST, port)).is_ok()
    }

    /// Check if a port has our server running on it.
    ///
    /// Useful for detecting if a previous instance is
    /// still running even if the lock file is stale.
    pub async fn is_our_server(port: u16, expected_version: &str) -> bool {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(HEALTH_CHECK_TIMEOUT_SECS))
            .build()
            .ok();

        if let Some(client) = client {
            let url = format!("{PROTOCOL}://{HOST}:{port}/{HEALTH_ENDPOINT}");
            if let Ok(resp) = client.get(&url).send().await
                && let Ok(body) = resp.text().await
            {
                // Check if response contains our server identifier
                return body.contains(SERVER_IDENTIFIER) || body.contains(expected_version);
            }
        }

        false
    }
}
