pub(crate) mod error;
pub(crate) mod load_result;
pub(crate) mod user_identity;

use crate::identity::{
    error::{IdentityError, Result as IdentityResult},
    load_result::LoadResult,
    user_identity::UserIdentity,
};

use std::fs;
use std::io::Write;
use std::panic::Location;
use std::path::PathBuf;

use log::{info, warn};
use tauri::Manager;

const DATE_FORMAT: &str = "%Y%m%d_%H%M%S";

/// Gets the user identity file path.
fn get_identity_path(app: &tauri::AppHandle) -> IdentityResult<PathBuf> {
    app.path()
        .app_data_dir()
        .map(|p| p.join("user.json"))
        .map_err(|e| IdentityError::app_data_dir(e.to_string()))
}

/// Loads user identity from app data directory.
///
/// Returns:
/// - `Ok(LoadResult { user: Some(...), corruption_error: None })` - loaded successfully
/// - `Ok(LoadResult { user: None, corruption_error: None })` - file doesn't exist (first launch)
/// - `Ok(LoadResult { user: None, corruption_error: Some(...) })` - file exists but corrupted
pub fn load(app: &tauri::AppHandle) -> IdentityResult<LoadResult> {
    let path = get_identity_path(app)?;

    if !path.exists() {
        info!("No identity file at {path:?} (first launch)");
        return Ok(LoadResult {
            user: None,
            corruption_error: None,
        });
    }

    let contents =
        fs::read_to_string(&path).map_err(|e| IdentityError::file_read(path.clone(), e))?;

    match serde_json::from_str::<UserIdentity>(&contents) {
        Ok(user) => {
            info!(
                "Loaded identity: {} (schema v{})",
                user.id, user.schema_version
            );
            Ok(LoadResult {
                user: Some(user),
                corruption_error: None,
            })
        }
        Err(e) => {
            warn!("Identity file corrupted at {path:?}: {e}");
            Ok(LoadResult {
                user: None,
                corruption_error: Some(e.to_string()),
            })
        }
    }
}

/// Saves user identity using atomic write pattern.
///
/// 1. Writes to temp file
/// 2. Syncs to disk (fsync)
/// 3. Atomic rename to final location
///
/// This prevents corruption if the app crashes mid-write.
pub fn save(app: &tauri::AppHandle, user: &UserIdentity) -> IdentityResult<()> {
    let app_data = app
        .path()
        .app_data_dir()
        .map_err(|e| IdentityError::app_data_dir(e.to_string()))?;

    // Ensure directory exists
    fs::create_dir_all(&app_data).map_err(|e| IdentityError::dir_creation(app_data.clone(), e))?;

    let final_path = app_data.join("user.json");
    let temp_path = app_data.join(format!("user.json.tmp.{}", std::process::id()));

    // Serialize with pretty printing for debuggability
    let json = serde_json::to_string_pretty(user)?;

    // Write to temp file with explicit sync
    {
        let mut file = fs::File::create(&temp_path)
            .map_err(|e| IdentityError::file_write(temp_path.clone(), e))?;

        file.write_all(json.as_bytes())
            .map_err(|e| IdentityError::file_write(temp_path.clone(), e))?;

        file.sync_all()
            .map_err(|e| IdentityError::file_write(temp_path.clone(), e))?;
    }

    // Atomic rename
    fs::rename(&temp_path, &final_path).map_err(|e| {
        // Clean up temp file on failure
        let _ = fs::remove_file(&temp_path);
        IdentityError::atomic_rename(temp_path, final_path.clone(), e)
    })?;

    info!("Saved identity: {}", user.id);
    Ok(())
}

/// Backs up corrupted identity file for debugging.
///
/// Renames `user.json` to `user.json.corrupted.{timestamp}`.
pub fn backup_corrupted(app: &tauri::AppHandle) -> IdentityResult<Option<PathBuf>> {
    let path = get_identity_path(app)?;

    if !path.exists() {
        return Ok(None);
    }

    let app_data = path.parent().unwrap();
    let timestamp = chrono::Utc::now().format(DATE_FORMAT);
    let backup_path = app_data.join(format!("user.json.corrupted.{timestamp}"));

    fs::rename(&path, &backup_path).map_err(|e| IdentityError::BackupFailed {
        source: e,
        location: error_location::ErrorLocation::from(Location::caller()),
    })?;

    warn!("Backed up corrupted identity to {backup_path:?}");
    Ok(Some(backup_path))
}
