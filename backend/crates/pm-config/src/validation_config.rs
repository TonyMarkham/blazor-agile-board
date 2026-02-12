use crate::{ConfigError, ConfigErrorResult};

use serde::Deserialize;

// Validation constraints
pub const MIN_TITLE_LENGTH: usize = 1;
pub const MAX_TITLE_LENGTH: usize = 500;
pub const DEFAULT_MAX_TITLE_LENGTH: usize = 200;

pub const MIN_DESCRIPTION_LENGTH: usize = 0;
pub const MAX_DESCRIPTION_LENGTH: usize = 500000;
pub const DEFAULT_MAX_DESCRIPTION_LENGTH: usize = 10000;

pub const MIN_STORY_POINTS: i32 = 0;
pub const MAX_STORY_POINTS: i32 = 1000;
pub const DEFAULT_MAX_STORY_POINTS: i32 = 100;

pub const MIN_ERROR_MESSAGE_LENGTH: usize = 50;
pub const MAX_ERROR_MESSAGE_LENGTH: usize = 1000;
pub const DEFAULT_MAX_ERROR_MESSAGE_LENGTH: usize = 200;
pub const MIN_CONFIGURABLE_COMMENT_LENGTH: usize = 1;
pub const MAX_CONFIGURABLE_COMMENT_LENGTH: usize = 500000;
pub const DEFAULT_MAX_COMMENT_LENGTH: usize = 5000;
pub const MIN_CONFIGURABLE_SPRINT_NAME_LENGTH: usize = 1;
pub const MAX_CONFIGURABLE_SPRINT_NAME_LENGTH: usize = 500;
pub const DEFAULT_MAX_SPRINT_NAME_LENGTH: usize = 100;
pub const MIN_COMMENT_CONTENT_LENGTH: usize = 1;

// === Time Entry Limits ===
/// Maximum length for time entry description
pub const MAX_TIME_ENTRY_DESCRIPTION_LENGTH: usize = 1000;
/// Maximum duration for a single time entry (24 hours in seconds)
pub const MAX_TIME_ENTRY_DURATION_SECONDS: i64 = 86400;
/// Tolerance for future timestamps (60 seconds for clock drift)
pub const MAX_FUTURE_TIMESTAMP_TOLERANCE_SECONDS: i64 = 60;
/// Default number of time entries to return per page
pub const DEFAULT_TIME_ENTRIES_LIMIT: i32 = 100;
/// Maximum number of time entries to return per page
pub const MAX_TIME_ENTRIES_LIMIT: i32 = 500;

// === Dependency Limits ===
/// Maximum number of dependencies that can block a single item
pub const MAX_BLOCKING_DEPENDENCIES_PER_ITEM: usize = 50;
/// Maximum number of items that a single item can block
pub const MAX_BLOCKED_DEPENDENCIES_PER_ITEM: usize = 50;

/// Validation configuration for field limits.
///
/// These limits are applied during input validation to prevent
/// abuse and ensure reasonable data sizes.
#[derive(Debug, Clone, Copy, Deserialize)]
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
    /// Maximum length for comment content
    pub max_comment_length: usize,
    /// Maximum length for sprint names
    pub max_sprint_name_length: usize,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            max_title_length: DEFAULT_MAX_TITLE_LENGTH,
            max_description_length: DEFAULT_MAX_DESCRIPTION_LENGTH,
            max_story_points: DEFAULT_MAX_STORY_POINTS,
            max_error_message_length: DEFAULT_MAX_ERROR_MESSAGE_LENGTH,
            max_comment_length: DEFAULT_MAX_COMMENT_LENGTH,
            max_sprint_name_length: DEFAULT_MAX_SPRINT_NAME_LENGTH,
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

        if self.max_description_length > MAX_DESCRIPTION_LENGTH {
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

        if self.max_comment_length < MIN_CONFIGURABLE_COMMENT_LENGTH
            || self.max_comment_length > MAX_CONFIGURABLE_COMMENT_LENGTH
        {
            return Err(ConfigError::config(format!(
                "validation.max_comment_length must be {}-{}, got {}",
                MIN_CONFIGURABLE_COMMENT_LENGTH,
                MAX_CONFIGURABLE_COMMENT_LENGTH,
                self.max_comment_length
            )));
        }

        if self.max_sprint_name_length < MIN_CONFIGURABLE_SPRINT_NAME_LENGTH
            || self.max_sprint_name_length > MAX_CONFIGURABLE_SPRINT_NAME_LENGTH
        {
            return Err(ConfigError::config(format!(
                "validation.max_sprint_name_length must be {}-{}, got {}",
                MIN_CONFIGURABLE_SPRINT_NAME_LENGTH,
                MAX_CONFIGURABLE_SPRINT_NAME_LENGTH,
                self.max_sprint_name_length
            )));
        }

        Ok(())
    }
}
