use crate::{ConfigError, ConfigErrorResult};

use serde::Deserialize;

// Validation constraints
pub const MIN_TITLE_LENGTH: usize = 1;
pub const MAX_TITLE_LENGTH: usize = 500;
pub const DEFAULT_MAX_TITLE_LENGTH: usize = 200;

pub const MIN_DESCRIPTION_LENGTH: usize = 0;
pub const MAX_DESCRIPTION_LENGTH: usize = 100000;
pub const DEFAULT_MAX_DESCRIPTION_LENGTH: usize = 10000;

pub const MIN_STORY_POINTS: i32 = 0;
pub const MAX_STORY_POINTS: i32 = 1000;
pub const DEFAULT_MAX_STORY_POINTS: i32 = 100;

pub const MIN_ERROR_MESSAGE_LENGTH: usize = 50;
pub const MAX_ERROR_MESSAGE_LENGTH: usize = 1000;
pub const DEFAULT_MAX_ERROR_MESSAGE_LENGTH: usize = 200;

/// Validation configuration for field limits.
///
/// These limits are applied during input validation to prevent
/// abuse and ensure reasonable data sizes.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ValidationConfig {
    /// Maximum length for work item titles
    pub max_title_length: usize,
    /// Maximum length for work item descriptions
    pub max_description_length: usize,
    /// Maximum story points allowed
    pub max_story_points: i32,
    /// Maximum length for error messages returned to clients
    pub max_error_message_length: usize,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            max_title_length: DEFAULT_MAX_TITLE_LENGTH,
            max_description_length: DEFAULT_MAX_DESCRIPTION_LENGTH,
            max_story_points: DEFAULT_MAX_STORY_POINTS,
            max_error_message_length: DEFAULT_MAX_ERROR_MESSAGE_LENGTH,
        }
    }
}

impl ValidationConfig {
    pub fn validate(&self) -> ConfigErrorResult<()> {
        if self.max_title_length < MIN_TITLE_LENGTH || self.max_title_length > MAX_TITLE_LENGTH {
            return Err(ConfigError::config(format!(
                "validation.max_title_length must be {}-{}, got {}",
                MIN_TITLE_LENGTH, MAX_TITLE_LENGTH, self.max_title_length
            )));
        }

        if self.max_description_length < MIN_DESCRIPTION_LENGTH
            || self.max_description_length > MAX_DESCRIPTION_LENGTH
        {
            return Err(ConfigError::config(format!(
                "validation.max_description_length must be {}-{}, got {}",
                MIN_DESCRIPTION_LENGTH, MAX_DESCRIPTION_LENGTH, self.max_description_length
            )));
        }

        if self.max_story_points < MIN_STORY_POINTS || self.max_story_points > MAX_STORY_POINTS {
            return Err(ConfigError::config(format!(
                "validation.max_story_points must be {}-{}, got {}",
                MIN_STORY_POINTS, MAX_STORY_POINTS, self.max_story_points
            )));
        }

        if self.max_error_message_length < MIN_ERROR_MESSAGE_LENGTH
            || self.max_error_message_length > MAX_ERROR_MESSAGE_LENGTH
        {
            return Err(ConfigError::config(format!(
                "validation.max_error_message_length must be {}-{}, got {}",
                MIN_ERROR_MESSAGE_LENGTH, MAX_ERROR_MESSAGE_LENGTH, self.max_error_message_length
            )));
        }

        Ok(())
    }
}
