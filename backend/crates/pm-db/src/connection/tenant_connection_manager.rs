use crate::{DbError, Result};

use pm_core::ErrorLocation;

use std::collections::HashMap;
use std::panic::Location;
use std::path::PathBuf;
use std::sync::Arc;

use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use tokio::sync::RwLock;

pub struct TenantConnectionManager {
    pools: Arc<RwLock<HashMap<String, SqlitePool>>>,
    base_path: PathBuf,
}

impl TenantConnectionManager {
    pub fn new(base_path: impl Into<PathBuf>) -> Self {
        Self {
            pools: Arc::new(RwLock::new(HashMap::new())),
            base_path: base_path.into(),
        }
    }

    pub async fn get_pool(&self, tenant_id: &str) -> Result<SqlitePool> {
        // Fast path: Check if pool already exists (read lock)
        {
            let pools = self.pools.read().await;
            if let Some(pool) = pools.get(tenant_id) {
                return Ok(pool.clone());
            }
        }

        // Slow path: Need to create pool (write lock for entire operation)
        let mut pools = self.pools.write().await;

        // Double-check: Another thread might have created it while we waited for write lock
        if let Some(pool) = pools.get(tenant_id) {
            return Ok(pool.clone());
        }

        // Create new pool (we hold write lock to prevent other threads from doing this)
        let pool = self.create_pool(tenant_id).await?;

        // Store in cache
        pools.insert(tenant_id.to_string(), pool.clone());

        Ok(pool)
    }

    async fn create_pool(&self, tenant_id: &str) -> Result<SqlitePool> {
        let db_path = self.get_database_path(tenant_id);

        // Create directory if it doesn't exist
        if let Some(parent) = db_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| DbError::Initialization {
                    message: format!("Failed to create tenant directory: {}", e),
                    location: ErrorLocation::from(Location::caller()),
                })?;
        }

        let _db_url = format!("sqlite:{}", db_path.display());

        let options = SqliteConnectOptions::new()
            .filename(&db_path)
            .create_if_missing(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await?;

        // Enable foreign keys
        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&pool)
            .await?;

        // Run migrations
        self.run_migrations(&pool).await?;

        Ok(pool)
    }

    async fn run_migrations(&self, pool: &SqlitePool) -> Result<()> {
        sqlx::migrate!("./migrations")
            .run(pool)
            .await
            .map_err(|e| DbError::Migration {
                message: format!("Migration failed: {}", e),
                location: ErrorLocation::from(Location::caller()),
            })?;

        Ok(())
    }

    fn get_database_path(&self, tenant_id: &str) -> PathBuf {
        self.base_path.join(tenant_id).join("main.db")
    }
}
