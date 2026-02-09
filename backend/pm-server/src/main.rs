pub mod admin;
pub mod api;
pub mod error;
pub mod health;
pub mod logger;
pub mod routes;

#[cfg(test)]
mod tests;

pub use api::{
    comments::{
        comment_dto::CommentDto,
        comment_list_response::CommentListResponse,
        comment_response::CommentResponse,
        comments::{create_comment, delete_comment, list_comments, update_comment},
        create_comment_request::CreateCommentRequest,
        update_comment_request::UpdateCommentRequest,
    },
    delete_response::DeleteResponse,
    dependencies::{
        create_dependency_request::CreateDependencyRequest,
        dependencies::{create_dependency, delete_dependency, list_dependencies},
        dependency_dto::DependencyDto,
        dependency_list_response::DependencyListResponse,
    },
    error::ApiError,
    error::Result as ApiResult,
    extractors::user_id::UserId,
    projects::{
        create_project_request::CreateProjectRequest,
        project_dto::ProjectDto,
        project_list_response::ProjectListResponse,
        project_response::ProjectResponse,
        projects::{create_project, delete_project, get_project, list_projects, update_project},
        update_project_request::UpdateProjectRequest,
    },
    sprints::{
        create_sprint_request::CreateSprintRequest,
        sprint_dto::SprintDto,
        sprint_list_response::SprintListResponse,
        sprint_response::SprintResponse,
        sprints::{create_sprint, delete_sprint, get_sprint, list_sprints, update_sprint},
        update_sprint_request::UpdateSprintRequest,
    },
    work_items::{
        create_work_item_request::CreateWorkItemRequest,
        list_work_item_query::ListWorkItemsQuery,
        update_work_item_request::UpdateWorkItemRequest,
        work_item_dto::WorkItemDto,
        work_item_list_response::WorkItemListResponse,
        work_item_response::WorkItemResponse,
        work_items::{
            create_work_item, delete_work_item, get_work_item, list_work_items, update_work_item,
        },
    },
};

pub use crate::routes::build_router;

use pm_auth::{JwtValidator, RateLimiterFactory};
use pm_ws::{
    AppState, CircuitBreaker, CircuitBreakerConfig, ConnectionConfig, ConnectionLimits,
    ConnectionRegistry, Metrics, ShutdownCoordinator,
};

use std::error::Error;
use std::sync::Arc;

use log::{error, info, warn};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Load and validate configuration
    let config = pm_config::Config::load()?;
    config.validate()?;

    // Construct log file path if configured
    let log_file_path: Option<std::path::PathBuf> = if let Some(ref filename) = config.logging.file
    {
        let config_dir = pm_config::Config::config_dir()?;
        let log_dir = config_dir.join(&config.logging.dir);

        // Ensure log directory exists
        std::fs::create_dir_all(&log_dir)?;

        Some(log_dir.join(filename))
    } else {
        None
    };

    // Initialize logger (before any other logging)
    logger::initialize(config.logging.level, log_file_path, config.logging.colored)?;

    info!("Starting pm-server v{}", env!("CARGO_PKG_VERSION"));
    config.log_summary();

    // Initialize database pool
    let database_path = config.database_path()?;
    info!("Connecting to database: {}", database_path.display());

    let pool = SqlitePoolOptions::new()
        .max_connections(10)
        .connect_with(
            SqliteConnectOptions::new()
                .filename(database_path)
                .create_if_missing(true)
                .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
                .synchronous(sqlx::sqlite::SqliteSynchronous::Normal)
                .busy_timeout(std::time::Duration::from_secs(5)),
        )
        .await?;

    info!("Database connection established");

    // Run migrations
    info!("Running database migrations...");
    sqlx::migrate!("../crates/pm-db/migrations")
        .run(&pool)
        .await?;
    info!("Migrations complete");

    ensure_llm_user(&pool, &config).await;

    // Create circuit breaker
    let circuit_breaker = Arc::new(CircuitBreaker::new(CircuitBreakerConfig::default()));
    info!("Circuit breaker initialized");

    // Create JWT validator (optional based on auth.enabled)
    let jwt_validator: Option<Arc<JwtValidator>> = if config.auth.enabled {
        let validator = if let Some(ref secret) = config.auth.jwt_secret {
            info!("JWT: HS256 authentication enabled");
            JwtValidator::with_hs256(secret.as_bytes())
        } else if let Some(ref key_path) = config.auth.jwt_public_key_path {
            let config_dir = pm_config::Config::config_dir()?;
            let full_path = config_dir.join(key_path);
            let public_key = std::fs::read_to_string(&full_path).map_err(|e| {
                error::ServerError::JwtKeyFile {
                    path: full_path.display().to_string(),
                    source: e,
                }
            })?;
            info!("JWT: RS256 authentication enabled");
            JwtValidator::with_rs256(&public_key)?
        } else {
            unreachable!("validate() ensures JWT config when auth.enabled")
        };
        Some(Arc::new(validator))
    } else {
        warn!("Authentication DISABLED - running in desktop/development mode");
        None
    };

    // Get desktop user ID for anonymous mode
    let desktop_user_id = config.auth.get_desktop_user_id();

    // Convert config types for pm-auth
    let rate_limiter_factory = RateLimiterFactory::new(pm_auth::RateLimitConfig {
        max_requests: config.rate_limit.max_requests,
        window_secs: config.rate_limit.window_secs,
    });

    // Create connection registry with limits
    let registry = ConnectionRegistry::new(ConnectionLimits {
        max_total: config.server.max_connections,
    });
    let registry_for_idle = registry.clone();

    // Create metrics collector
    let metrics = Metrics::new();

    // Create shutdown coordinator
    let shutdown = ShutdownCoordinator::new();

    // Create connection config for pm-ws
    let connection_config = ConnectionConfig {
        send_buffer_size: config.websocket.send_buffer_size,
        heartbeat_interval_secs: config.websocket.heartbeat_interval_secs,
        heartbeat_timeout_secs: config.websocket.heartbeat_timeout_secs,
    };

    // Build application state
    let app_state = AppState {
        pool,
        circuit_breaker,
        jwt_validator,
        desktop_user_id,
        rate_limiter_factory,
        registry,
        metrics,
        shutdown: shutdown.clone(),
        config: connection_config,
        api_config: config.api.clone(),
    };

    // Build router
    let app = build_router(app_state);

    // Create TCP listener
    let bind_addr = config.bind_addr();
    let listener = TcpListener::bind(&bind_addr).await?;

    // Get actual bound address (important when port is 0 / auto-assigned)
    let actual_addr = listener.local_addr()?;
    info!("Server listening on {}", actual_addr);

    // Write port discovery file for CLI auto-discovery
    match pm_config::PortFileInfo::write(actual_addr.port(), &config.server.host) {
        Ok(path) => info!("Port file written: {}", path.display()),
        Err(e) => warn!(
            "Failed to write port file (CLI auto-discovery may not work): {}",
            e
        ),
    }

    // Spawn signal handler for graceful shutdown
    let shutdown_for_signal = shutdown.clone();
    tokio::spawn(async move {
        match tokio::signal::ctrl_c().await {
            Ok(()) => {
                info!("Received SIGINT (Ctrl+C), initiating graceful shutdown");
                shutdown_for_signal.shutdown();
            }
            Err(e) => {
                error!("Failed to listen for SIGINT: {}", e);
            }
        }
    });

    // Idle shutdown monitoring (when configured)
    if config.server.idle_shutdown_secs > 0 {
        let idle_timeout = config.server.idle_shutdown_secs;
        let shutdown_for_idle = shutdown.clone();

        info!("Idle shutdown enabled: {}s timeout", idle_timeout);

        tokio::spawn(async move {
            let grace_period = idle_timeout.min(60);
            info!("Idle shutdown grace period: {}s", grace_period);
            tokio::time::sleep(std::time::Duration::from_secs(grace_period)).await;

            let check_interval = (idle_timeout / 2).max(10);

            loop {
                tokio::time::sleep(std::time::Duration::from_secs(check_interval)).await;

                if registry_for_idle.total_count().await == 0 {
                    info!(
                        "No active connections, checking again in {}s...",
                        check_interval
                    );

                    tokio::time::sleep(std::time::Duration::from_secs(check_interval)).await;

                    if registry_for_idle.total_count().await == 0 {
                        warn!(
                            "No connections for {}s, initiating auto-shutdown",
                            idle_timeout
                        );
                        shutdown_for_idle.shutdown();
                        break;
                    } else {
                        info!("Connection established, continuing...");
                    }
                }
            }
        });
    }

    // Start server with graceful shutdown
    info!("Server ready to accept connections");
    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            shutdown.subscribe_guard().wait().await;
            info!("Graceful shutdown complete");
        })
        .await?;

    // Clean up port discovery file
    if let Err(e) = pm_config::PortFileInfo::remove() {
        warn!("Failed to remove port file: {}", e);
    }

    Ok(())
}

/// Ensure the LLM user exists in the database
async fn ensure_llm_user(pool: &sqlx::SqlitePool, config: &pm_config::Config) {
    let llm_user_id = &config.api.llm_user_id;
    let llm_user_name = &config.api.llm_user_name;

    match sqlx::query("INSERT OR IGNORE INTO users (id, email, name) VALUES (?, ?, ?)")
        .bind(llm_user_id)
        .bind(format!("{}@system.local", llm_user_id))
        .bind(llm_user_name)
        .execute(pool)
        .await
    {
        Ok(result) => {
            if result.rows_affected() > 0 {
                log::info!("Created LLM user: {} ({})", llm_user_name, llm_user_id);
            }
        }
        Err(e) => {
            log::warn!("Failed to create LLM user (may already exist): {}", e);
        }
    }
}
