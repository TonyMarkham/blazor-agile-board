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

/// Create a temp config directory and set PM_CONFIG_DIR
pub(crate) fn setup_config_dir() -> (TempDir, EnvGuard) {
    let temp = TempDir::new().unwrap();
    let guard = EnvGuard::set("PM_CONFIG_DIR", temp.path().to_str().unwrap());
    (temp, guard)
}
