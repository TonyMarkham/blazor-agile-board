use error_location::ErrorLocation;

use uuid::Uuid;

/// Unique connection identifier                                                                                                                                                 
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ConnectionId(Uuid);

impl ConnectionId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn parse(value: &str) -> Result<Self, crate::WsError> {
        let uuid = Uuid::parse_str(value).map_err(|_| crate::WsError::ValidationError {
            message: format!("Invalid connection_id: {}", value),
            field: Some("connection_id".to_string()),
            location: ErrorLocation::from(std::panic::Location::caller()),
        })?;
        Ok(Self(uuid))
    }
}

impl Default for ConnectionId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ConnectionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
