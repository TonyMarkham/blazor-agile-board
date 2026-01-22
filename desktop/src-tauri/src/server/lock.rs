//! Lock file for single-instance enforcement.

use crate::server::{ServerError, ServerResult};

use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::panic::Location;
use std::path::{Path, PathBuf};

use error_location::ErrorLocation;

#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;

const LOCK_FILENAME: &str = "server.lock";
const LOCK_FILE_MODE: u32 = 0o600; // Owner read/write only

/// Manages a lock file that prevents multiple instances.
///
/// The lock file contains JSON with the PID, port, and start time.
/// This allows detecting stale locks from crashed processes.
pub struct LockFile {
    path: PathBuf,
    file: Option<File>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct LockInfo {
    pid: u32,
    port: u16,
    started_at: String,
}

impl LockFile {
    /// Try to acquire the lock file.
    ///
    /// Returns Ok if acquired, Err if another instance is running.
    /// If a lock file exists but the process is dead, the stale
    /// lock is removed and acquisition succeeds.
    pub fn acquire(data_dir: &Path, port: u16) -> ServerResult<Self> {
        let path = data_dir.join(LOCK_FILENAME);

        // Check if existing lock is stale
        if path.exists()
            && let Ok(existing) = Self::read_lock_info(&path)
        {
            if Self::is_process_running(existing.pid) {
                return Err(ServerError::AlreadyRunning {
                    path: path.clone(),
                    location: ErrorLocation::from(Location::caller()),
                });
            }
            // Stale lock from crashed process, remove it
            tracing::info!(
                "Removing stale lock file (PID {} not running)",
                existing.pid
            );
            std::fs::remove_file(&path).ok();
        }

        // Create lock file with exclusive access
        #[cfg(unix)]
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(LOCK_FILE_MODE)
            .open(&path)
            .map_err(|e| ServerError::LockAcquisition {
                path: path.clone(),
                source: e,
                location: ErrorLocation::from(Location::caller()),
            })?;

        #[cfg(windows)]
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)
            .map_err(|e| ServerError::LockAcquisition {
                path: path.clone(),
                source: e,
                location: ErrorLocation::from(Location::caller()),
            })?;

        let mut lock = Self {
            path,
            file: Some(file),
        };

        lock.write_info(port)?;

        Ok(lock)
    }

    /// Write current process info to lock file.
    fn write_info(&mut self, port: u16) -> ServerResult<()> {
        let info = LockInfo {
            pid: std::process::id(),
            port,
            started_at: chrono::Utc::now().to_rfc3339(),
        };

        let content = serde_json::to_string_pretty(&info).unwrap();

        if let Some(ref mut file) = self.file {
            file.write_all(content.as_bytes())?;
            file.sync_all()?;
        }

        Ok(())
    }

    /// Read lock info from existing file.
    fn read_lock_info(path: &Path) -> Result<LockInfo, std::io::Error> {
        let mut file = File::open(path)?;
        let mut content = String::new();
        file.read_to_string(&mut content)?;
        serde_json::from_str(&content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    /// Check if a process with given PID is running.
    #[cfg(unix)]
    fn is_process_running(pid: u32) -> bool {
        // kill(pid, 0) returns 0 if process exists, -1 otherwise
        unsafe { libc::kill(pid as i32, 0) == 0 }
    }

    /// Check if a process with given PID is running (Windows).
    #[cfg(windows)]
    fn is_process_running(pid: u32) -> bool {
        use windows_sys::Win32::Foundation::{CloseHandle, STILL_ACTIVE};
        use windows_sys::Win32::System::Threading::{
            GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
        };

        unsafe {
            let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if handle == 0 {
                return false;
            }

            let mut exit_code: u32 = 0;
            let result = GetExitCodeProcess(handle, &mut exit_code);
            CloseHandle(handle);

            result != 0 && exit_code == STILL_ACTIVE
        }
    }

    /// Release the lock file.
    ///
    /// Called automatically on drop, but can be called
    /// explicitly for graceful shutdown.
    pub fn release(&mut self) {
        self.file.take();
        std::fs::remove_file(&self.path).ok();
    }
}

impl Drop for LockFile {
    fn drop(&mut self) {
        self.release();
    }
}
