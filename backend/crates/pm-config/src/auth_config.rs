use crate::{
    ConfigError, ConfigErrorResult, DEFAULT_AUTH_ENABLED, DEFAULT_DESKTOP_USER_ID,
    MIN_JWT_SECRET_LENGTH,
};

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct AuthConfig {
    /// Enable JWT authentication (default: false for desktop mode)
    pub enabled: bool,

    /// HS256 JWT secret (min 32 characters when auth enabled)
    #[serde(default, skip_serializing)]
    pub jwt_secret: Option<String>,

    /// Path to RS256 public key PEM file (relative to config dir)
    pub jwt_public_key_path: Option<String>,

    /// User ID when auth is disabled (default: "local-user")
    /// Set to empty string to generate unique session ID per connection
    pub desktop_user_id: Option<String>,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            enabled: DEFAULT_AUTH_ENABLED,
            jwt_secret: None,
            jwt_public_key_path: None,
            desktop_user_id: Some(DEFAULT_DESKTOP_USER_ID.to_string()),
        }
    }
}

impl AuthConfig {
    pub fn validate(&self, config_dir: &std::path::Path) -> ConfigErrorResult<()> {
        if !self.enabled {
            return Ok(());
        }

        // Auth enabled requires JWT config
        match (&self.jwt_secret, &self.jwt_public_key_path) {
            (None, None) => {
                return Err(ConfigError::auth(
                    "auth.enabled=true requires either jwt_secret or jwt_public_key_path. \
                       Set auth.enabled=false for desktop mode.",
                ));
            }
            (Some(secret), _) => {
                if secret.len() < MIN_JWT_SECRET_LENGTH {
                    return Err(ConfigError::auth(format!(
                        "auth.jwt_secret must be at least {} characters for security, got {}",
                        MIN_JWT_SECRET_LENGTH,
                        secret.len()
                    )));
                }
            }
            (None, Some(path)) => {
                // Validate path is relative and within config dir (prevent path traversal)
                let key_path = std::path::Path::new(path);
                // is_absolute() returns false on Windows for Unix-style paths (/etc/passwd),
                // so also reject anything starting with '/' to be cross-platform.
                if key_path.is_absolute() || path.starts_with('/') {
                    return Err(ConfigError::auth(
                        "auth.jwt_public_key_path must be relative to config directory",
                    ));
                }
                if path.contains("..") {
                    return Err(ConfigError::auth(
                        "auth.jwt_public_key_path cannot contain '..' (path traversal protection)",
                    ));
                }

                let full_path = config_dir.join(path);
                if !full_path.exists() {
                    return Err(ConfigError::auth(format!(
                        "auth.jwt_public_key_path '{}' does not exist (looked for {})",
                        path,
                        full_path.display()
                    )));
                }
            }
        }

        Ok(())
    }

    /// Get the desktop user ID, generating a unique session ID if configured to do so.
    pub fn get_desktop_user_id(&self) -> String {
        match &self.desktop_user_id {
            Some(id) if !id.is_empty() => id.clone(),
            _ => format!("session-{}", uuid::Uuid::new_v4()),
        }
    }
}
