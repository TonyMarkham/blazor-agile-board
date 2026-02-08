mod api_config;
mod auth;
mod circuit_breaker;
mod config;
mod desktop_id;
mod edge_cases;
mod handler;
mod port_file;
mod retry;
mod server;
mod validation;
mod web_socket;

use std::env;

use tempfile::TempDir;

/// RAII guard for environment variables - automatically restores on drop
pub(crate) struct EnvGuard {
    key: &'static str,
    original: Option<String>,
}

impl EnvGuard {
    pub(crate) fn set(key: &'static str, value: &str) -> Self {
        unsafe {
            let original = env::var(key).ok();
            env::set_var(key, value);
            Self { key, original }
        }
    }

    #[allow(dead_code)]
    pub(crate) fn remove(key: &'static str) -> Self {
        unsafe {
            let original = env::var(key).ok();
            env::remove_var(key);
            Self { key, original }
        }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        unsafe {
            match &self.original {
                Some(val) => env::set_var(self.key, val),
                None => env::remove_var(self.key),
            }
        }
    }
}

/// Create a temp config directory with a git repo for testing.
///
/// Initializes a git repo in the temp dir and creates `.pm/` inside it,
/// so `Config::config_dir_from_git(temp.path())` returns `<temp>/.pm/`.
pub(crate) fn setup_config_dir() -> TempDir {
    let temp = TempDir::new().unwrap();

    // Init a git repo so config_dir_from_git() can find the root
    let output = std::process::Command::new("git")
        .args(["init"])
        .current_dir(temp.path())
        .output()
        .expect("git init failed");
    assert!(output.status.success(), "git init failed in temp dir");

    // Create .pm/ subdirectory
    std::fs::create_dir_all(temp.path().join(".pm")).unwrap();

    temp
}
