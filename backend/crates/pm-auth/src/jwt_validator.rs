use crate::{AuthError, Claims, Result as AuthErrorResult};

use pm_core::ErrorLocation;

use std::panic::Location;

use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};

/// Production-grade JWT validator
pub struct JwtValidator {
    decoding_key: DecodingKey,
    validation: Validation,
    algorithm: Algorithm,
}

impl JwtValidator {
    /// Create validator with HS256 (symmetric secret)
    pub fn with_hs256(secret: &[u8]) -> Self {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = true;
        validation.validate_nbf = true;
        validation.leeway = 30; // 30 second clock skew tolerance

        Self {
            decoding_key: DecodingKey::from_secret(secret),
            validation,
            algorithm: Algorithm::HS256,
        }
    }

    /// Create validator with RS256 (asymmetric public key)
    #[track_caller]
    pub fn with_rs256(public_key_pem: &str) -> AuthErrorResult<Self> {
        let decoding_key = DecodingKey::from_rsa_pem(public_key_pem.as_bytes())
            .map_err(|e| AuthError::InvalidToken {
                message: format!("Invalid RSA public key: {}", e),
                location: ErrorLocation::from(Location::caller()),
            })?;

        let mut validation = Validation::new(Algorithm::RS256);
        validation.validate_exp = true;
        validation.validate_nbf = true;
        validation.leeway = 30;

        Ok(Self {
            decoding_key,
            validation,
            algorithm: Algorithm::RS256,
        })
    }

    /// Validate JWT token and return claims
    #[track_caller]
    pub fn validate(&self, token: &str) -> AuthErrorResult<Claims> {
        let token_data = decode::<Claims>(token, &self.decoding_key, &self.validation)
            .map_err(|e| {
                use jsonwebtoken::errors::ErrorKind;
                match e.kind() {
                    ErrorKind::ExpiredSignature => AuthError::TokenExpired {
                        location: ErrorLocation::from(Location::caller()),
                    },
                    _ => AuthError::JwtDecode {
                        source: e,
                        location: ErrorLocation::from(Location::caller()),
                    },
                }
            })?;

        // Additional claim validation
        token_data.claims.validate()?;

        Ok(token_data.claims)
    }

    /// Get the algorithm being used (for logging/debugging)
    pub fn algorithm(&self) -> &str {
        match self.algorithm {
            Algorithm::HS256 => "HS256",
            Algorithm::RS256 => "RS256",
            _ => "unknown",
        }
    }
}