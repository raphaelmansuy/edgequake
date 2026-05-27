//! PostgreSQL connection pool management.
//!
//! Provides connection pooling with lazy initialization and extension setup.
//!
//! ## Implements
//!
//! - [`FEAT0246`]: Connection pool with lazy initialization
//! - [`FEAT0247`]: Extension auto-setup (pgvector, AGE, pgcrypto)
//!
//! ## Use Cases
//!
//! - [`UC0901`]: System establishes database connection
//!
//! ## Enforces
//!
//! - [`BR0246`]: Connection reuse via pooling
//! - [`BR0247`]: Extension availability validation

use std::sync::Arc;
use tokio::sync::RwLock;

use sqlx::postgres::{PgPool, PgPoolOptions};
use sqlx::Row;

use super::config::PostgresConfig;
use crate::error::{Result, StorageError};

/// PostgreSQL connection pool wrapper.
#[derive(Clone)]
pub struct PostgresPool {
    pool: Arc<RwLock<Option<PgPool>>>,
    config: PostgresConfig,
}

impl PostgresPool {
    /// Create a new pool with the given configuration.
    pub fn new(config: PostgresConfig) -> Self {
        Self {
            pool: Arc::new(RwLock::new(None)),
            config,
        }
    }

    /// Wrap an already-initialized `PgPool` (SPEC-011: shared pool across adapters).
    ///
    /// Skips lazy pool creation and extension setup — caller must have initialized
    /// extensions on the underlying pool already.
    pub fn from_existing(pool: PgPool, config: PostgresConfig) -> Self {
        Self {
            pool: Arc::new(RwLock::new(Some(pool))),
            config,
        }
    }

    /// Get the configuration.
    pub fn config(&self) -> &PostgresConfig {
        &self.config
    }

    /// Initialize the connection pool.
    pub async fn initialize(&self) -> Result<()> {
        let mut pool_guard = self.pool.write().await;

        if pool_guard.is_some() {
            return Ok(());
        }

        let pool = PgPoolOptions::new()
            .max_connections(self.config.max_connections)
            .min_connections(self.config.min_connections)
            .acquire_timeout(self.config.connect_timeout)
            .idle_timeout(Some(self.config.idle_timeout))
            // WHY: Force search_path=public on every connection.
            // After migration 001 creates the 'edgequake' schema, PostgreSQL's
            // default search_path "$user",public resolves "$user"="edgequake" to
            // that schema first. Unqualified table references in storage queries
            // (e.g. INSERT INTO documents) could resolve to edgequake views instead
            // of the actual public tables. Pinning search_path to public ensures
            // consistent, correct table resolution on all pool connections.
            .after_connect(|conn, _| {
                Box::pin(async move {
                    sqlx::query("SET search_path TO public")
                        .execute(conn)
                        .await
                        .map(|_| ())
                })
            })
            .connect(&self.config.connection_url())
            .await
            .map_err(|e| StorageError::Connection(format!("Failed to connect: {}", e)))?;

        // Enable required extensions
        self.setup_extensions(&pool).await?;

        *pool_guard = Some(pool);
        Ok(())
    }

    /// Get a reference to the connection pool.
    pub async fn get(&self) -> Result<PgPool> {
        let pool_guard = self.pool.read().await;
        pool_guard
            .clone()
            .ok_or_else(|| StorageError::Connection("Pool not initialized".to_string()))
    }

    /// Close the connection pool.
    pub async fn close(&self) -> Result<()> {
        let mut pool_guard = self.pool.write().await;
        if let Some(pool) = pool_guard.take() {
            pool.close().await;
        }
        Ok(())
    }

    /// Check if the pool is connected.
    pub async fn is_connected(&self) -> bool {
        let pool_guard = self.pool.read().await;
        pool_guard.is_some()
    }

    /// Set up required PostgreSQL extensions.
    async fn setup_extensions(&self, pool: &PgPool) -> Result<()> {
        // Enable pgvector extension
        sqlx::query("CREATE EXTENSION IF NOT EXISTS vector")
            .execute(pool)
            .await
            .map_err(|e| {
                StorageError::Database(format!(
                    "Failed to create vector extension: {}. Make sure pgvector is installed.",
                    e
                ))
            })?;

        // Enable Apache AGE extension
        match sqlx::query("CREATE EXTENSION IF NOT EXISTS age CASCADE")
            .execute(pool)
            .await
        {
            Ok(_) => {
                // Verify AGE catalog is reachable, then reset search_path on this connection
                // so it is not returned to the pool with ag_catalog-first resolution.
                let mut conn = pool.acquire().await.map_err(|e| {
                    StorageError::Database(format!("Failed to acquire connection for AGE: {}", e))
                })?;
                sqlx::query("SET search_path = ag_catalog, \"$user\", public")
                    .execute(&mut *conn)
                    .await
                    .map_err(|e| {
                        StorageError::Database(format!("Failed to set AGE search_path: {}", e))
                    })?;
                sqlx::query("SET search_path TO public")
                    .execute(&mut *conn)
                    .await
                    .map_err(|e| {
                        StorageError::Database(format!("Failed to reset search_path: {}", e))
                    })?;
            }
            Err(e) => {
                // AGE is optional - log warning but continue
                tracing::warn!(
                    "Apache AGE extension not available: {}. Graph operations will use fallback.",
                    e
                );
            }
        }

        Ok(())
    }

    /// Execute a raw query (for testing).
    #[allow(dead_code)]
    pub async fn execute(&self, query: &str) -> Result<()> {
        let pool = self.get().await?;
        sqlx::query(query)
            .execute(&pool)
            .await
            .map_err(|e| StorageError::Database(format!("Query failed: {}", e)))?;
        Ok(())
    }

    /// Check database connectivity.
    pub async fn health_check(&self) -> Result<bool> {
        let pool = self.get().await?;
        let row = sqlx::query("SELECT 1 as health")
            .fetch_one(&pool)
            .await
            .map_err(|e| StorageError::Connection(format!("Health check failed: {}", e)))?;

        let health: i32 = row.get("health");
        Ok(health == 1)
    }
}

impl std::fmt::Debug for PostgresPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PostgresPool")
            .field("config", &self.config)
            .field(
                "connected",
                &self.pool.try_read().map(|g| g.is_some()).unwrap_or(false),
            )
            .finish()
    }
}
