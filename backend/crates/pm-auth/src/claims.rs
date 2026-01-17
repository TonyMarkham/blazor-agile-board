use crate::{AuthError, Result as AuthErrorResult};

use std::panic::Location;

use error_location::ErrorLocation;
use serde::{Deserialize, Serialize};

/// JWT Claims structure - matches platform JWT format                                                                                                                           
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject (user_id)                                                                                                                                                        
    pub sub: String,
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
