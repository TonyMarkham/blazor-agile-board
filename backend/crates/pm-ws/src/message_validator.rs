use crate::{Result as WsErrorResult, WsError};

use std::panic::Location;

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
        if value.len() < min_length {
            return Err(WsError::InvalidMessage {
                message: format!("{} must be at least {} characters", field_name, min_length),
                location: ErrorLocation::from(Location::caller()),
            });
        }

        if value.len() > max_length {
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
    ) -> WsErrorResult<()> {
        // Validate title
        Self::validate_string(title, "title", 1, 200)?;

        // Validate description if present
        if let Some(desc) = description
            && desc.len() > 10000
        {
            return Err(WsError::InvalidMessage {
                message: "description exceeds maximum length (10000)".to_string(),
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
    pub fn validate_comment_create(content: &str) -> WsErrorResult<()> {
        Self::validate_string(content, "content", 1, 5000)
    }

    /// Validate sprint creation request                                                                                                                                         
    #[track_caller]
    pub fn validate_sprint_create(name: &str, start_date: i64, end_date: i64) -> WsErrorResult<()> {
        // Validate name
        Self::validate_string(name, "name", 1, 100)?;

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
}
