//! Port discovery file for multi-instance support.
//!
//! The server writes this file after binding to a port.
//! The CLI reads it to discover the server URL without manual --server flags.
//!
//! File location: `<config_dir>/server.json`
//!
//! ## Stale file detection
//!
//! If the server crashes without cleanup, the file remains. `read_live()`
//! checks whether the PID in the file is still running. If not, it removes
//! the stale file and returns `None`.
//!
//! ## Race condition protection
//!
//! `write()` checks for an existing live server before writing. If another
//! server is already running (same config directory), `write()` returns an
//! error instead of silently overwriting.

use crate::{Config, ConfigError, ConfigErrorResult, port_file::is_process_running};

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

const PORT_FILENAME: &str = "server.json";

/// Information stored in the port discovery file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortFileInfo {
    /// Process ID of the server that wrote this file
    pub pid: u32,
    /// Port the server is listening on
    pub port: u16,
    /// Host the server is bound to
    pub host: String,
    /// ISO 8601 timestamp when the server started
    pub started_at: String,
    /// Server version for diagnostics (useful when sharing server.json for troubleshooting)
    pub version: String,
}

impl PortFileInfo {
    /// Ensure the parent directory of a path exists.
    fn ensure_parent_dir(path: &std::path::Path) -> ConfigErrorResult<()> {
        if let Some(dir) = path.parent()
            && !dir.exists()
        {
            std::fs::create_dir_all(dir).map_err(|e| ConfigError::Io {
                path: dir.to_path_buf(),
                source: e,
            })?;
        }
        Ok(())
    }

    /// Write a port discovery file to the config directory.
    ///
    /// Called by the server after `TcpListener::bind()` succeeds.
    ///
    /// **Safety checks:**
    /// - Creates the config directory if it doesn't exist (safe to call before `Config::load()`)
    /// - Refuses to overwrite a live server's port file (returns error with PID/port info)
    /// - Automatically removes stale files from dead processes before writing
    pub fn write(port: u16, host: &str) -> ConfigErrorResult<PathBuf> {
        let path = Self::path()?;

        // Ensure config directory exists (safe to call before Config::load())
        Self::ensure_parent_dir(&path)?;

        // Guard: refuse to overwrite a live server's port file.
        // Note: there is a small TOCTOU window between this check and the write
        // below, but for a local development tool the risk is negligible.
        // read_live() also auto-removes stale files from dead processes.
        if let Ok(Some(existing)) = Self::read_live() {
            return Err(ConfigError::config(format!(
                "Another pm-server is already running on port {} (PID {}). \
                   Stop it first or use a different config directory.",
                existing.port, existing.pid
            )));
        }

        let info = PortFileInfo {
            pid: std::process::id(),
            port,
            host: host.to_string(),
            started_at: chrono::Utc::now().to_rfc3339(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        };

        let content = serde_json::to_string_pretty(&info)
            .map_err(|e| ConfigError::config(format!("Failed to serialize port file: {e}")))?;

        std::fs::write(&path, content).map_err(|e| ConfigError::Io {
            path: path.clone(),
            source: e,
        })?;

        Ok(path)
    }

    /// Write to a specific config directory (for tests).
    pub fn write_in(
        config_dir: &std::path::Path,
        port: u16,
        host: &str,
    ) -> ConfigErrorResult<PathBuf> {
        let path = config_dir.join(PORT_FILENAME);

        Self::ensure_parent_dir(&path)?;

        if let Ok(Some(existing)) = Self::read_live_in(config_dir) {
            return Err(ConfigError::config(format!(
                "Another pm-server is already running on port {} (PID {}). \
                     Stop it first or use a different config directory.",
                existing.port, existing.pid
            )));
        }

        let info = PortFileInfo {
            pid: std::process::id(),
            port,
            host: host.to_string(),
            started_at: chrono::Utc::now().to_rfc3339(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        };

        let content = serde_json::to_string_pretty(&info)
            .map_err(|e| ConfigError::config(format!("Failed to serialize port file: {e}")))?;

        std::fs::write(&path, content).map_err(|e| ConfigError::Io {
            path: path.clone(),
            source: e,
        })?;

        Ok(path)
    }

    /// Read the port discovery file from the config directory.
    ///
    /// Returns `Ok(None)` if the file does not exist.
    /// Returns `Err` if the file exists but cannot be read or parsed.
    pub fn read() -> ConfigErrorResult<Option<PortFileInfo>> {
        let path = Self::path()?;

        if !path.exists() {
            return Ok(None);
        }

        let content = std::fs::read_to_string(&path).map_err(|e| ConfigError::Io {
            path: path.clone(),
            source: e,
        })?;

        let info: PortFileInfo = serde_json::from_str(&content).map_err(|e| {
            ConfigError::config(format!("Invalid port file {}: {e}", path.display()))
        })?;

        Ok(Some(info))
    }

    /// Read from a specific config directory (for tests).
    pub fn read_in(config_dir: &std::path::Path) -> ConfigErrorResult<Option<PortFileInfo>> {
        let path = config_dir.join(PORT_FILENAME);

        if !path.exists() {
            return Ok(None);
        }

        let content = std::fs::read_to_string(&path).map_err(|e| ConfigError::Io {
            path: path.clone(),
            source: e,
        })?;

        let info: PortFileInfo = serde_json::from_str(&content).map_err(|e| {
            ConfigError::config(format!("Invalid port file {}: {e}", path.display()))
        })?;

        Ok(Some(info))
    }

    /// Read the port discovery file and verify the server process is still alive.
    ///
    /// Returns `Ok(None)` if:
    /// - The file does not exist
    /// - The file exists but the PID is no longer running (stale file removed)
    ///
    /// This is the primary method the CLI should use.
    pub fn read_live() -> ConfigErrorResult<Option<PortFileInfo>> {
        let info = match Self::read()? {
            Some(info) => info,
            None => return Ok(None),
        };

        if is_process_running(info.pid) {
            Ok(Some(info))
        } else {
            // Stale file - server died without cleanup
            log::debug!(
                "Removing stale port file (pid {} no longer running)",
                info.pid
            );
            let path = Self::path()?;
            std::fs::remove_file(&path).ok(); // Best-effort cleanup
            Ok(None)
        }
    }

    /// Read from a specific config directory and verify process liveness (for tests).
    pub fn read_live_in(config_dir: &std::path::Path) -> ConfigErrorResult<Option<PortFileInfo>> {
        let info = match Self::read_in(config_dir)? {
            Some(i) => i,
            None => return Ok(None),
        };

        if !is_process_running(info.pid) {
            let _ = Self::remove_in(config_dir);
            return Ok(None);
        }

        Ok(Some(info))
    }

    /// Delete the port discovery file.
    ///
    /// Called by the server on graceful shutdown.
    /// Silently succeeds if the file does not exist.
    pub fn remove() -> ConfigErrorResult<()> {
        let path = Self::path()?;
        if path.exists() {
            std::fs::remove_file(&path).map_err(|e| ConfigError::Io {
                path: path.clone(),
                source: e,
            })?;
        }
        Ok(())
    }

    /// Remove from a specific config directory (for tests).
    pub fn remove_in(config_dir: &std::path::Path) -> ConfigErrorResult<()> {
        let path = config_dir.join(PORT_FILENAME);
        if path.exists() {
            std::fs::remove_file(&path).map_err(|e| ConfigError::Io {
                path: path.clone(),
                source: e,
            })?;
        }
        Ok(())
    }

    /// Get the path to the port discovery file.
    ///
    /// Returns `<config_dir>/server.json`. Useful for error messages
    /// that need to tell the user where the port file is expected.
    pub fn path() -> ConfigErrorResult<PathBuf> {
        let config_dir = Config::config_dir()?;
        Ok(config_dir.join(PORT_FILENAME))
    }
}
