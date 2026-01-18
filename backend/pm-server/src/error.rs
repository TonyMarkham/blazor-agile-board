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

    #[error("Database connection failed: {0}")]
    DatabaseConnection(String),

    #[allow(dead_code)]
    #[error("Database pool exhausted")]
    PoolExhausted,

    #[allow(dead_code)]
    #[error("JWT validation error: {0}")]
    JwtValidation(String),
}

impl From<sqlx::Error> for ServerError {
    fn from(err: sqlx::Error) -> Self {
        Self::DatabaseConnection(err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, ServerError>;
