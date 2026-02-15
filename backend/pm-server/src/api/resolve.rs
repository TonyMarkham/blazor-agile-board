//! Display key and project key resolution helpers
//!
//! This module provides utilities to resolve human-readable identifiers
//! (display keys like "PONE-126", project keys like "PONE") into database entities.

use crate::ApiError;

use pm_core::Project;
use pm_db::{ProjectRepository, WorkItemRepository};

use std::panic::Location;

use error_location::ErrorLocation;
use sqlx::SqlitePool;
use uuid::Uuid;

// =============================================================================
// Display Key Parsing
// =============================================================================

/// Parse a display key like "PONE-126" into (project_key, item_number).
///
/// # Format
/// Display keys must match the pattern: `{PROJECT_KEY}-{NUMBER}`
/// - Project key: 1-10 uppercase ASCII letters (A-Z)
/// - Separator: single hyphen (-)
/// - Item number: positive integer (1 or more digits)
///
/// # Errors
/// Returns `ApiError::BadRequest` if:
/// - Input doesn't contain exactly one hyphen
/// - Project key is empty or contains non-uppercase-ASCII characters
/// - Item number is missing, non-numeric, or zero
pub fn parse_display_key(s: &str) -> Result<(&str, i64), ApiError> {
    // Split on hyphen - must have exactly 2 parts
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 2 {
        return Err(ApiError::BadRequest {
            message: format!(
                "Invalid display key format '{}': expected PROJECT-NUMBER (e.g., PONE-126)",
                s
            ),
            location: ErrorLocation::from(Location::caller()),
        });
    }

    let project_key = parts[0];
    let number_str = parts[1];

    // Validate project key: 1-10 uppercase ASCII letters
    if project_key.is_empty() || project_key.len() > 10 {
        return Err(ApiError::BadRequest {
            message: format!(
                "Invalid project key '{}': must be 1-10 uppercase letters",
                project_key
            ),
            location: ErrorLocation::from(Location::caller()),
        });
    }

    if !project_key.chars().all(|c| c.is_ascii_uppercase()) {
        return Err(ApiError::BadRequest {
            message: format!(
                "Invalid project key '{}': must contain only uppercase ASCII letters (A-Z)",
                project_key
            ),
            location: ErrorLocation::from(Location::caller()),
        });
    }

    // Parse item number - must be positive integer
    let item_number: i64 = number_str.parse().map_err(|_| ApiError::BadRequest {
        message: format!(
            "Invalid item number '{}': must be a positive integer",
            number_str
        ),
        location: ErrorLocation::from(Location::caller()),
    })?;

    if item_number <= 0 {
        return Err(ApiError::BadRequest {
            message: format!(
                "Invalid item number {}: must be greater than 0",
                item_number
            ),
            location: ErrorLocation::from(Location::caller()),
        });
    }

    Ok((project_key, item_number))
}

// =============================================================================
// Project Resolution
// =============================================================================

/// Resolve a project identifier (UUID or project key) to a Project entity.
///
/// # Input Formats
/// - **UUID**: `"8d96310e-1e69-4dc5-9529-5c173674ab90"` (36 chars with hyphens)
/// - **Project key**: `"PONE"` (1-10 uppercase ASCII letters)
///
/// # Resolution Strategy
/// 1. Try parsing as UUID → if success, look up by ID
/// 2. If UUID parsing fails, treat as project key → look up by key
/// 3. Return NotFound if no match in database
///
/// # Errors
/// - `ApiError::NotFound` if project doesn't exist or is soft-deleted
/// - Database errors propagated from repository layer
pub async fn resolve_project(pool: &SqlitePool, identifier: &str) -> Result<Project, ApiError> {
    let repo = ProjectRepository::new(pool.clone());

    // Try UUID first (fast, no database query)
    if let Ok(uuid) = Uuid::parse_str(identifier) {
        return repo
            .find_by_id(uuid)
            .await?
            .ok_or_else(|| ApiError::NotFound {
                message: format!("Project with ID {} not found", identifier),
                location: ErrorLocation::from(Location::caller()),
            });
    }

    // Not a UUID, treat as project key
    repo.find_by_key(identifier)
        .await?
        .ok_or_else(|| ApiError::NotFound {
            message: format!("Project with key '{}' not found", identifier),
            location: ErrorLocation::from(Location::caller()),
        })
}

// =============================================================================
// Work Item Resolution
// =============================================================================

/// Resolve a work item identifier (UUID or display key) to a WorkItem entity.
///
/// # Input Formats
/// - **UUID**: `"a18c7c4c-a7af-427f-86fb-200d9a6c777b"` (36 chars with hyphens)
/// - **Display key**: `"PONE-126"` (project key + hyphen + item number)
///
/// # Resolution Strategy
/// 1. Try parsing as UUID → if success, look up by ID
/// 2. If UUID parsing fails, try parsing as display key:
///    - Extract project key and item number using `parse_display_key`
///    - Resolve project key to project UUID using `resolve_project`
///    - Look up work item by project ID + item number
/// 3. Return NotFound if no match in database
///
/// # Errors
/// - `ApiError::BadRequest` if display key format is invalid (from `parse_display_key`)
/// - `ApiError::NotFound` if project or work item doesn't exist or is soft-deleted
/// - Database errors propagated from repository layer
pub async fn resolve_work_item(
    pool: &SqlitePool,
    identifier: &str,
) -> Result<pm_core::WorkItem, ApiError> {
    // Try UUID first (fast, no database query)
    if let Ok(uuid) = Uuid::parse_str(identifier) {
        return WorkItemRepository::find_by_id(pool, uuid)
            .await?
            .ok_or_else(|| ApiError::NotFound {
                message: format!("Work item with ID {} not found", identifier),
                location: ErrorLocation::from(Location::caller()),
            });
    }

    // Not a UUID, try parsing as display key (PROJ-123)
    let (project_key, item_number) = parse_display_key(identifier)?;

    // Resolve project key to project entity
    let project = resolve_project(pool, project_key).await?;

    // Look up work item by project ID + item number
    WorkItemRepository::find_by_project_and_number(pool, project.id, item_number as i32)
        .await?
        .ok_or_else(|| ApiError::NotFound {
            message: format!(
                "Work item {} not found in project {}",
                identifier, project_key
            ),
            location: ErrorLocation::from(Location::caller()),
        })
}
