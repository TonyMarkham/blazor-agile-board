use crate::{Result as WsErrorResult, WsError};

use pm_config::{
    MAX_FUTURE_TIMESTAMP_TOLERANCE_SECONDS, MAX_TIME_ENTRY_DESCRIPTION_LENGTH,
    MAX_TIME_ENTRY_DURATION_SECONDS, MIN_COMMENT_CONTENT_LENGTH, ValidationConfig,
};
use pm_core::DependencyType;
use pm_proto::DependencyType as ProtoDependencyType;

use std::panic::Location;

use chrono::Utc;
use error_location::ErrorLocation;

/// Validates protobuf messages from clients                                                                                                                                     
pub struct MessageValidator;

impl MessageValidator {
    /// Validate a subscription request                                                                                                                                          
    #[track_caller]
    pub fn validate_subscribe(project_id: &str, resource_type: &str) -> WsErrorResult<()> {
        // Validate project_id
        if project_id.is_empty() {
            return Err(WsError::InvalidMessage {
                message: "project_id cannot be empty".to_string(),
                location: ErrorLocation::from(Location::caller()),
            });
        }

        if project_id.len() > 128 {
            return Err(WsError::InvalidMessage {
                message: "project_id exceeds maximum length (128)".to_string(),
                location: ErrorLocation::from(Location::caller()),
            });
        }

        // Validate resource_type
        match resource_type {
            "project" | "sprint" | "work_item" => Ok(()),
            _ => Err(WsError::InvalidMessage {
                message: format!("invalid resource_type: {}", resource_type),
                location: ErrorLocation::from(Location::caller()),
            }),
        }
    }

    /// Validate a UUID string                                                                                                                                                   
    #[track_caller]
    pub fn validate_uuid(uuid_str: &str, field_name: &str) -> WsErrorResult<()> {
        if uuid_str.is_empty() {
            return Err(WsError::InvalidMessage {
                message: format!("{} cannot be empty", field_name),
                location: ErrorLocation::from(Location::caller()),
            });
        }

        // Basic UUID format check (36 chars with dashes)
        if uuid_str.len() != 36 {
            return Err(WsError::InvalidMessage {
                message: format!("{} must be a valid UUID", field_name),
                location: ErrorLocation::from(Location::caller()),
            });
        }

        // Can add more strict UUID parsing if needed
        Ok(())
    }

    /// Validate a string field                                                                                                                                                  
    #[track_caller]
    pub fn validate_string(
        value: &str,
        field_name: &str,
        min_length: usize,
        max_length: usize,
    ) -> WsErrorResult<()> {
        let char_count = value.chars().count();

        if char_count < min_length {
            return Err(WsError::InvalidMessage {
                message: format!("{} must be at least {} characters", field_name, min_length),
                location: ErrorLocation::from(Location::caller()),
            });
        }

        if char_count > max_length {
            return Err(WsError::InvalidMessage {
                message: format!("{} must not exceed {} characters", field_name, max_length),
                location: ErrorLocation::from(Location::caller()),
            });
        }

        Ok(())
    }

    /// Validate work item creation request                                                                                                                                      
    #[track_caller]
    pub fn validate_work_item_create(
        title: &str,
        description: Option<&str>,
        item_type: &str,
        config: &ValidationConfig,
    ) -> WsErrorResult<()> {
        // Validate title
        Self::validate_string(title, "title", 1, config.max_title_length)?;

        // Validate description if present
        if let Some(desc) = description
            && desc.chars().count() > config.max_description_length
        {
            return Err(WsError::InvalidMessage {
                message: format!(
                    "description exceeds maximum length ({} characters)",
                    config.max_description_length
                ),
                location: ErrorLocation::from(Location::caller()),
            });
        }

        // Validate item_type
        match item_type {
            "project" | "epic" | "story" | "task" => Ok(()),
            _ => Err(WsError::InvalidMessage {
                message: format!("invalid item_type: {}", item_type),
                location: ErrorLocation::from(Location::caller()),
            }),
        }
    }

    /// Validate comment creation request                                                                                                                                        
    #[track_caller]
    pub fn validate_comment_create(content: &str, config: &ValidationConfig) -> WsErrorResult<()> {
        Self::validate_string(
            content,
            "content",
            MIN_COMMENT_CONTENT_LENGTH,
            config.max_comment_length,
        )
    }

    /// Validate sprint creation request                                                                                                                                         
    #[track_caller]
    pub fn validate_sprint_create(
        name: &str,
        start_date: i64,
        end_date: i64,
        config: &ValidationConfig,
    ) -> WsErrorResult<()> {
        // Validate name
        Self::validate_string(name, "name", 1, config.max_sprint_name_length)?;

        // Validate dates
        if start_date >= end_date {
            return Err(WsError::InvalidMessage {
                message: "start_date must be before end_date".to_string(),
                location: ErrorLocation::from(Location::caller()),
            });
        }

        // Validate dates are not in distant past or future (sanity check)
        let now = chrono::Utc::now().timestamp();
        let one_year_ago = now - (365 * 24 * 60 * 60);
        let five_years_future = now + (5 * 365 * 24 * 60 * 60);

        if start_date < one_year_ago || start_date > five_years_future {
            return Err(WsError::InvalidMessage {
                message: "start_date is outside reasonable range".to_string(),
                location: ErrorLocation::from(Location::caller()),
            });
        }

        if end_date < one_year_ago || end_date > five_years_future {
            return Err(WsError::InvalidMessage {
                message: "end_date is outside reasonable range".to_string(),
                location: ErrorLocation::from(Location::caller()),
            });
        }

        Ok(())
    }

    /// Validate project creation request
    #[track_caller]
    pub fn validate_project_create(
        title: &str,
        description: Option<&str>,
        key: &str,
        config: &ValidationConfig,
    ) -> WsErrorResult<()> {
        // Validate title (same limits as work items)
        Self::validate_string(title, "title", 1, config.max_title_length)?;

        // Validate description if present (same limit as work items)
        if let Some(desc) = description
            && desc.chars().count() > config.max_description_length
        {
            return Err(WsError::InvalidMessage {
                message: format!(
                    "description exceeds maximum length ({} characters)",
                    config.max_description_length
                ),
                location: ErrorLocation::from(Location::caller()),
            });
        }

        // Validate key format
        if key.len() < 2 || key.len() > 20 {
            return Err(WsError::InvalidMessage {
                message: "key must be 2-20 characters".to_string(),
                location: ErrorLocation::from(Location::caller()),
            });
        }

        if !key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Err(WsError::InvalidMessage {
                message: "key must contain only letters, numbers, and underscores".to_string(),
                location: ErrorLocation::from(Location::caller()),
            });
        }

        Ok(())
    }

    /// Validate pagination parameters                                                                                                                                           
    #[track_caller]
    pub fn validate_pagination(limit: u32, _offset: u32) -> WsErrorResult<()> {
        if limit == 0 {
            return Err(WsError::InvalidMessage {
                message: "limit must be greater than 0".to_string(),
                location: ErrorLocation::from(Location::caller()),
            });
        }

        if limit > 1000 {
            return Err(WsError::InvalidMessage {
                message: "limit must not exceed 1000".to_string(),
                location: ErrorLocation::from(Location::caller()),
            });
        }

        Ok(())
    }

    /// Validate time entry description (optional, max 1000 chars)
    #[track_caller]
    pub fn validate_time_entry_description(description: Option<&str>) -> WsErrorResult<()> {
        if let Some(desc) = description {
            Self::validate_string(desc, "description", 0, MAX_TIME_ENTRY_DESCRIPTION_LENGTH)?;
        }
        Ok(())
    }

    /// Validate time entry timestamps for manual entry creation.
    /// Ensures:
    /// - Neither timestamp is in the future (with tolerance)
    /// - started_at is before ended_at
    /// - Duration doesn't exceed maximum (24 hours)
    #[track_caller]
    pub fn validate_time_entry_timestamps(started_at: i64, ended_at: i64) -> WsErrorResult<()> {
        let now = Utc::now().timestamp();

        // Cannot be in future (with tolerance for clock drift)
        if started_at > now + MAX_FUTURE_TIMESTAMP_TOLERANCE_SECONDS {
            return Err(WsError::ValidationError {
                message: "started_at cannot be in the future".into(),
                field: Some("started_at".into()),
                location: ErrorLocation::from(Location::caller()),
            });
        }
        if ended_at > now + MAX_FUTURE_TIMESTAMP_TOLERANCE_SECONDS {
            return Err(WsError::ValidationError {
                message: "ended_at cannot be in the future".into(),
                field: Some("ended_at".into()),
                location: ErrorLocation::from(Location::caller()),
            });
        }

        // Start must be before end
        if started_at >= ended_at {
            return Err(WsError::ValidationError {
                message: "started_at must be before ended_at".into(),
                field: Some("started_at".into()),
                location: ErrorLocation::from(Location::caller()),
            });
        }

        // Max duration check
        let duration = ended_at - started_at;
        if duration > MAX_TIME_ENTRY_DURATION_SECONDS {
            return Err(WsError::ValidationError {
                message: format!(
                    "Duration cannot exceed {} hours",
                    MAX_TIME_ENTRY_DURATION_SECONDS / 3600
                ),
                field: Some("ended_at".into()),
                location: ErrorLocation::from(Location::caller()),
            });
        }

        Ok(())
    }

    /// Validate dependency type enum from protobuf i32.
    /// Converts to domain DependencyType on success.
    #[track_caller]
    pub fn validate_dependency_type(value: i32) -> WsErrorResult<DependencyType> {
        match value {
            x if x == ProtoDependencyType::Blocks as i32 => Ok(DependencyType::Blocks),
            x if x == ProtoDependencyType::RelatesTo as i32 => Ok(DependencyType::RelatesTo),
            _ => Err(WsError::ValidationError {
                message: "Invalid dependency_type. Must be BLOCKS or RELATES_TO".into(),
                field: Some("dependency_type".into()),
                location: ErrorLocation::from(Location::caller()),
            }),
        }
    }
}
