use crate::{AuthError, Result as AuthErrorResult};

use std::panic::Location;

use error_location::ErrorLocation;
use serde::{Deserialize, Serialize};

/// JWT Claims structure - matches platform JWT format                                                                                                                           
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject (user_id)                                                                                                                                                        
    pub sub: String,
    /// Tenant identifier                                                                                                                                                        
    pub tenant_id: String,
    /// Expiration timestamp (Unix)                                                                                                                                              
    pub exp: i64,
    /// Issued at timestamp (Unix)                                                                                                                                               
    pub iat: i64,
    /// Optional: User roles for authorization                                                                                                                                   
    #[serde(default)]
    pub roles: Vec<String>,
}

impl Claims {
    /// Validate claims after JWT signature verification                                                                                                                         
    #[track_caller]
    pub fn validate(&self) -> AuthErrorResult<()> {
        // Validate tenant_id format (non-empty, reasonable length)
        if self.tenant_id.is_empty() {
            return Err(AuthError::InvalidClaim {
                claim: "tenant_id".to_string(),
                message: "tenant_id cannot be empty".to_string(),
                location: ErrorLocation::from(Location::caller()),
            });
        }
        if self.tenant_id.len() > 128 {
            return Err(AuthError::InvalidClaim {
                claim: "tenant_id".to_string(),
                message: "tenant_id exceeds maximum length".to_string(),
                location: ErrorLocation::from(Location::caller()),
            });
        }

        // Validate sub (user_id)
        if self.sub.is_empty() {
            return Err(AuthError::InvalidClaim {
                claim: "sub".to_string(),
                message: "sub (user_id) cannot be empty".to_string(),
                location: ErrorLocation::from(Location::caller()),
            });
        }

        Ok(())
    }
}
