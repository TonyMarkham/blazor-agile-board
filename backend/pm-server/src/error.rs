use thiserror::Error;

#[derive(Error, Debug)]
pub enum ServerError {
    #[error("Config error: {0}")]
    Config(#[from] pm_config::ConfigError),

    #[error("Failed to read JWT key file {path}: {source}")]
    JwtKeyFile {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("Environment variable error: {message}")]
    EnvVar { message: String },
}

pub type Result<T> = std::result::Result<T, ServerError>;
