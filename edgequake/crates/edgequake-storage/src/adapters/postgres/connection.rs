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

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

use sqlx::postgres::{PgConnection, PgPool, PgPoolOptions};
use sqlx::Row;

use super::config::{db_pool_role_from_env, resolve_pool_max_connections, PostgresConfig};
use crate::error::{Result, StorageError};

/// Default idle-in-transaction timeout (SPEC-112 / OLTP safety net).
pub const DEFAULT_IDLE_IN_XACT_TIMEOUT_SECS: u64 = 60;

/// Default sqlx idle reaping (matches `PostgresConfig` idle_timeout).
pub const DEFAULT_POOL_IDLE_TIMEOUT_SECS: u64 = 600;

/// Default sqlx max connection lifetime (sqlx-like 30m).
pub const DEFAULT_POOL_MAX_LIFETIME_SECS: u64 = 1800;

/// SPEC-112 LAW-112-4: `edgequake:<role>` for `pg_stat_activity` attribution.
pub fn session_application_name(role: Option<&str>) -> &'static str {
    match role.map(|r| r.to_ascii_lowercase()).as_deref() {
        Some("query") => "edgequake:query",
        Some("ingest") => "edgequake:ingest",
        Some("queue") => "edgequake:queue",
        Some("admin") => "edgequake:admin",
        _ => "edgequake:default",
    }
}

/// `EDGEQUAKE_DB_IDLE_IN_XACT_TIMEOUT_SECS` (default 60; clamp 5..=3600).
pub fn idle_in_xact_timeout_secs() -> u64 {
    std::env::var("EDGEQUAKE_DB_IDLE_IN_XACT_TIMEOUT_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_IDLE_IN_XACT_TIMEOUT_SECS)
        .clamp(5, 3600)
}

/// `EDGEQUAKE_DB_POOL_IDLE_TIMEOUT_SECS` (default 600; clamp 30..=86400).
pub fn pool_idle_timeout() -> Duration {
    let secs = std::env::var("EDGEQUAKE_DB_POOL_IDLE_TIMEOUT_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_POOL_IDLE_TIMEOUT_SECS)
        .clamp(30, 86_400);
    Duration::from_secs(secs)
}

/// `EDGEQUAKE_DB_POOL_MAX_LIFETIME_SECS` (default 1800; clamp 60..=86400).
pub fn pool_max_lifetime() -> Duration {
    let secs = std::env::var("EDGEQUAKE_DB_POOL_MAX_LIFETIME_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_POOL_MAX_LIFETIME_SECS)
        .clamp(60, 86_400);
    Duration::from_secs(secs)
}

/// SPEC-112: pin application_name + search_path + idle_in_transaction after connect/reset.
///
/// `app_name` must be a trusted static from [`session_application_name`] (no user input).
/// One bound SELECT avoids SQL interpolation and three separate network round trips.
pub async fn apply_session_baseline(
    conn: &mut PgConnection,
    app_name: &str,
) -> std::result::Result<(), sqlx::Error> {
    debug_assert!(
        app_name.starts_with("edgequake:")
            && app_name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b':' || b == b'_'),
        "application_name must be a trusted edgequake:* label"
    );
    let idle_secs = idle_in_xact_timeout_secs();
    sqlx::query(
        "SELECT set_config('application_name', $1, false), \
                set_config('search_path', 'public', false), \
                set_config('idle_in_transaction_session_timeout', $2, false)",
    )
    .bind(app_name)
    .bind(format!("{idle_secs}s"))
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// SPEC-090 F-090-07 / LAW-P4 + SPEC-112: labeled session hygiene.
///
/// `after_release` runs `RESET ALL` then **re-applies** the baseline so
/// `application_name` / idle-in-xact timeout survive pool reuse (LAW-112-4/7).
pub fn with_session_hygiene_labeled(
    options: PgPoolOptions,
    app_name: &'static str,
) -> PgPoolOptions {
    options
        .after_connect(move |conn, _| {
            Box::pin(async move {
                apply_session_baseline(conn, app_name).await?;
                Ok(())
            })
        })
        .after_release(move |conn, _meta| {
            Box::pin(async move {
                // RESET ALL clears session GUCs leaked by DDL/reconcile without
                // discarding sqlx prepared statements. Must not run inside a txn.
                if let Err(e) = sqlx::query("RESET ALL").execute(&mut *conn).await {
                    tracing::warn!(
                        error = %e,
                        "SPEC-090: after_release RESET ALL failed; dropping connection"
                    );
                    return Ok(false);
                }
                if let Err(e) = apply_session_baseline(conn, app_name).await {
                    tracing::warn!(
                        error = %e,
                        app_name,
                        "SPEC-112: after_release baseline re-pin failed; dropping connection"
                    );
                    return Ok(false);
                }
                Ok(true)
            })
        })
}

/// SPEC-090 F-090-07 / LAW-P4: default-label hygiene (single-pool / tests).
///
/// Uses `EDGEQUAKE_DB_POOL_ROLE` when set, otherwise `edgequake:default`.
pub fn with_session_hygiene(options: PgPoolOptions) -> PgPoolOptions {
    let label = session_application_name(db_pool_role_from_env().as_deref());
    with_session_hygiene_labeled(options, label)
}

/// PostgreSQL connection pool wrapper.
#[derive(Clone)]
pub struct PostgresPool {
    pool: Arc<RwLock<Option<PgPool>>>,
    config: PostgresConfig,
    /// Set during extension setup — AGE available for graph storage.
    graph_extension_available: Arc<AtomicBool>,
}

impl PostgresPool {
    /// Create a new pool with the given configuration.
    pub fn new(config: PostgresConfig) -> Self {
        Self {
            pool: Arc::new(RwLock::new(None)),
            config,
            graph_extension_available: Arc::new(AtomicBool::new(false)),
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
            graph_extension_available: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Whether Apache AGE extension was successfully enabled at pool init.
    pub fn graph_extension_available(&self) -> bool {
        self.graph_extension_available.load(Ordering::Relaxed)
    }

    /// Probe `pg_extension` for AGE (used when pool was from_existing without setup).
    pub async fn probe_graph_extension_available(&self) -> Result<bool> {
        let pool = self.get().await?;
        let available: bool =
            sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'age')")
                .fetch_one(&pool)
                .await
                .map_err(|e| StorageError::Database(format!("AGE extension probe failed: {e}")))?;
        self.graph_extension_available
            .store(available, Ordering::Relaxed);
        Ok(available)
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

        let max_connections = resolve_pool_max_connections(self.config.max_connections);

        let pool = with_session_hygiene(
            PgPoolOptions::new()
                .max_connections(max_connections)
                .min_connections(self.config.min_connections)
                .acquire_timeout(self.config.connect_timeout)
                .idle_timeout(Some(self.config.idle_timeout))
                .max_lifetime(Some(pool_max_lifetime())),
        )
        .connect_with(self.config.connect_options())
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

        // Enable Apache AGE extension (fail-closed unless opt-out — SPEC-090 F-090-19)
        let age_ok = match sqlx::query("CREATE EXTENSION IF NOT EXISTS age CASCADE")
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
                true
            }
            Err(e) => {
                if Self::allow_no_graph_extension() {
                    tracing::warn!(
                        "Apache AGE extension not available: {}. Graph operations will use fallback (EDGEQUAKE_ALLOW_NO_GRAPH=1).",
                        e
                    );
                    false
                } else {
                    return Err(StorageError::Database(format!(
                        "Apache AGE extension required but unavailable: {e}. \
                         Set EDGEQUAKE_ALLOW_NO_GRAPH=1 to start without graph storage."
                    )));
                }
            }
        };
        self.graph_extension_available
            .store(age_ok, Ordering::Relaxed);

        Ok(())
    }

    fn allow_no_graph_extension() -> bool {
        matches!(
            std::env::var("EDGEQUAKE_ALLOW_NO_GRAPH")
                .unwrap_or_default()
                .to_ascii_lowercase()
                .as_str(),
            "1" | "true" | "yes" | "on"
        )
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
