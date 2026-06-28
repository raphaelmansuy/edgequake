//! PostgreSQL Row-Level Security (RLS) context management.
//!
//! This module provides utilities for setting and clearing tenant/workspace
//! context in PostgreSQL sessions to enable RLS policy enforcement.
//!
//! ## Implements
//!
//! - [`FEAT0260`]: Row-Level Security for multi-tenancy
//! - [`FEAT0261`]: Session-scoped tenant context
//! - [`FEAT0262`]: RAII context guard with auto-cleanup
//!
//! ## Use Cases
//!
//! - [`UC0902`]: System enforces tenant data isolation
//! - [`UC0903`]: System scopes queries to current tenant
//!
//! ## Enforces
//!
//! - [`BR0260`]: Mandatory tenant context for data access
//! - [`BR0261`]: Context cleanup on scope exit
//!
//! # How it works
//!
//! PostgreSQL RLS policies use session variables (set via `set_config()`) to
//! determine which rows a query can access. This module provides:
//!
//! 1. `RlsContext` - A guard that sets context on creation and clears on drop
//! 2. `set_tenant_context()` - Low-level function to set session variables
//! 3. `clear_tenant_context()` - Clears the session variables
//!
//! # Example
//!
//! ```ignore
//! use edgequake_storage::postgres::RlsContext;
//!
//! // Create context - automatically sets session vars
//! let ctx = RlsContext::new(&pool, tenant_id, Some(workspace_id)).await?;
//!
//! // All queries in this scope will be filtered by RLS
//! let docs = sqlx::query!("SELECT * FROM documents").fetch_all(&pool).await?;
//!
//! // Context is automatically cleared when `ctx` goes out of scope
//! ```

use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{Result, StorageError};

/// Guard for PostgreSQL RLS context.
///
/// Sets the tenant/workspace context when created, and optionally clears
/// it when dropped (depending on configuration).
#[deprecated(
    since = "0.12.12",
    note = "Pool-level RLS leaks session vars across concurrent checkouts. Use acquire_rls_connection or with_acquired_tenant_context instead (SPEC-027 SEC-014)."
)]
#[derive(Debug)]
pub struct RlsContext {
    pool: PgPool,
    tenant_id: Uuid,
    workspace_id: Option<Uuid>,
    clear_on_drop: bool,
}

#[allow(deprecated)]
impl RlsContext {
    /// Create a new RLS context and set session variables.
    ///
    /// # Arguments
    /// * `pool` - PostgreSQL connection pool
    /// * `tenant_id` - The tenant ID to scope queries to
    /// * `workspace_id` - Optional workspace ID for finer scoping
    ///
    /// # Returns
    /// A guard that will clear the context when dropped.
    pub async fn new(pool: &PgPool, tenant_id: Uuid, workspace_id: Option<Uuid>) -> Result<Self> {
        set_tenant_context(pool, tenant_id, workspace_id).await?;

        Ok(Self {
            pool: pool.clone(),
            tenant_id,
            workspace_id,
            clear_on_drop: true,
        })
    }

    /// Create a context that doesn't clear on drop.
    ///
    /// Useful when you want the context to persist for the connection lifetime.
    pub async fn persistent(
        pool: &PgPool,
        tenant_id: Uuid,
        workspace_id: Option<Uuid>,
    ) -> Result<Self> {
        set_tenant_context(pool, tenant_id, workspace_id).await?;

        Ok(Self {
            pool: pool.clone(),
            tenant_id,
            workspace_id,
            clear_on_drop: false,
        })
    }

    /// Get the current tenant ID.
    pub fn tenant_id(&self) -> Uuid {
        self.tenant_id
    }

    /// Get the current workspace ID.
    pub fn workspace_id(&self) -> Option<Uuid> {
        self.workspace_id
    }

    /// Explicitly clear the context.
    pub async fn clear(&self) -> Result<()> {
        clear_tenant_context(&self.pool).await
    }

    /// Update the workspace scope.
    pub async fn set_workspace(&mut self, workspace_id: Option<Uuid>) -> Result<()> {
        self.workspace_id = workspace_id;
        set_tenant_context(&self.pool, self.tenant_id, workspace_id).await
    }
}

#[allow(deprecated)]
impl Drop for RlsContext {
    fn drop(&mut self) {
        if self.clear_on_drop {
            // Spawn a task to clear context since Drop can't be async
            let pool = self.pool.clone();
            tokio::spawn(async move {
                if let Err(e) = clear_tenant_context(&pool).await {
                    tracing::warn!(
                        error.source = "postgres_rls",
                        error.action = "clear_context_on_drop",
                        error.message = %e,
                        "Failed to clear RLS context on drop"
                    );
                }
            });
        }
    }
}

/// Set the tenant/workspace context for RLS policies.
///
/// This calls the `set_tenant_context()` PostgreSQL function which sets
/// session variables that RLS policies use for filtering.
///
/// # Deprecated
///
/// Prefer [`acquire_rls_connection`] or [`with_acquired_tenant_context`] — setting
/// context on the pool can leak session variables to unrelated queries.
#[deprecated(
    since = "0.12.12",
    note = "Use acquire_rls_connection or with_acquired_tenant_context (SPEC-027 SEC-014)."
)]
pub async fn set_tenant_context(
    pool: &PgPool,
    tenant_id: Uuid,
    workspace_id: Option<Uuid>,
) -> Result<()> {
    set_tenant_context_on_conn(pool, tenant_id, workspace_id, None).await
}

/// Clear the tenant/workspace context.
///
/// This resets the session variables to empty, effectively disabling
/// RLS filtering (queries will only see rows with NULL tenant_id).
pub async fn clear_tenant_context(pool: &PgPool) -> Result<()> {
    clear_tenant_context_on_conn(pool).await
}

/// Clear RLS session variables on a connection (pool-safe).
pub async fn clear_tenant_context_on_conn(
    conn: impl sqlx::Executor<'_, Database = sqlx::Postgres>,
) -> Result<()> {
    sqlx::query("SELECT clear_tenant_context()")
        .execute(conn)
        .await
        .map_err(|e| StorageError::Database(format!("Failed to clear RLS context: {e}")))?;

    tracing::debug!("RLS context cleared");

    Ok(())
}

/// Set tenant/workspace/user context on a connection (pool-safe — use with `acquire()`).
pub async fn set_tenant_context_on_conn(
    conn: impl sqlx::Executor<'_, Database = sqlx::Postgres>,
    tenant_id: Uuid,
    workspace_id: Option<Uuid>,
    user_id: Option<Uuid>,
) -> Result<()> {
    sqlx::query("SELECT set_tenant_context($1, $2, $3)")
        .bind(tenant_id)
        .bind(workspace_id)
        .bind(user_id)
        .execute(conn)
        .await
        .map_err(|e| StorageError::Database(format!("Failed to set RLS context: {e}")))?;

    tracing::debug!(
        tenant_id = %tenant_id,
        workspace_id = ?workspace_id,
        user_id = ?user_id,
        "RLS context set on connection"
    );

    Ok(())
}

/// Acquire a pooled connection with RLS tenant context set (SPEC-027 SEC-014 SSOT).
///
/// Prefer this over pool-level `set_tenant_context` — session variables must not leak
/// across concurrent pool checkouts.
pub async fn acquire_rls_connection(
    pool: &PgPool,
    tenant_id: Uuid,
    workspace_id: Option<Uuid>,
    user_id: Option<Uuid>,
) -> Result<sqlx::pool::PoolConnection<sqlx::Postgres>> {
    let mut conn = pool
        .acquire()
        .await
        .map_err(|e| StorageError::Database(format!("Failed to acquire PG connection: {e}")))?;
    set_tenant_context_on_conn(&mut *conn, tenant_id, workspace_id, user_id).await?;
    Ok(conn)
}

/// Clear RLS context before returning a connection to the pool.
pub async fn release_rls_connection(conn: &mut sqlx::PgConnection) -> Result<()> {
    clear_tenant_context_on_conn(conn).await
}

/// Run an operation with RLS context on a **single acquired connection** (SPEC-027 SEC-014).
///
/// Session variables are transaction-local; using the pool directly without `acquire()`
/// can leak or miss context across concurrent requests.
pub async fn with_acquired_tenant_context<F, Fut, T>(
    pool: &PgPool,
    tenant_id: Uuid,
    workspace_id: Option<Uuid>,
    user_id: Option<Uuid>,
    operation: F,
) -> Result<T>
where
    F: for<'c> FnOnce(&'c mut sqlx::PgConnection) -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    let mut conn = acquire_rls_connection(pool, tenant_id, workspace_id, user_id).await?;

    let result = operation(&mut conn).await;

    if let Err(e) = release_rls_connection(&mut conn).await {
        tracing::warn!(
            error.source = "postgres_rls",
            error.message = %e,
            "Failed to clear RLS context after operation"
        );
    }

    result
}

/// Get the current tenant ID from the session.
pub async fn get_current_tenant_id(pool: &PgPool) -> Result<Option<Uuid>> {
    let result: Option<(Option<Uuid>,)> = sqlx::query_as("SELECT current_tenant_id()")
        .fetch_optional(pool)
        .await
        .map_err(|e| StorageError::Database(format!("Failed to get tenant ID: {}", e)))?;

    Ok(result.and_then(|r| r.0))
}

/// Get the current workspace ID from the session.
pub async fn get_current_workspace_id(pool: &PgPool) -> Result<Option<Uuid>> {
    let result: Option<(Option<Uuid>,)> = sqlx::query_as("SELECT current_workspace_id()")
        .fetch_optional(pool)
        .await
        .map_err(|e| StorageError::Database(format!("Failed to get workspace ID: {}", e)))?;

    Ok(result.and_then(|r| r.0))
}

/// Execute a query with tenant context.
///
/// This is a helper macro for executing queries with RLS context set.
/// The context is automatically cleared after the closure returns.
///
/// # Example
///
/// ```ignore
/// let docs = with_tenant_context!(&pool, tenant_id, workspace_id, async {
///     sqlx::query_as!(Document, "SELECT * FROM documents")
///         .fetch_all(&pool)
///         .await
/// })?;
/// ```
#[macro_export]
macro_rules! with_tenant_context {
    ($pool:expr, $tenant_id:expr, $workspace_id:expr, $body:expr) => {{
        let _ctx = $crate::postgres::rls::RlsContext::new($pool, $tenant_id, $workspace_id).await?;
        $body
    }};
}

/// Builder for RLS-scoped queries.
#[derive(Debug, Clone)]
pub struct RlsQueryBuilder {
    tenant_id: Uuid,
    workspace_id: Option<Uuid>,
}

impl RlsQueryBuilder {
    /// Create a new query builder for the given tenant.
    pub fn new(tenant_id: Uuid) -> Self {
        Self {
            tenant_id,
            workspace_id: None,
        }
    }

    /// Scope to a specific workspace.
    pub fn workspace(mut self, workspace_id: Uuid) -> Self {
        self.workspace_id = Some(workspace_id);
        self
    }

    /// Get the WHERE clause conditions for manual query building.
    ///
    /// Returns a tuple of (condition, parameters).
    pub fn where_clause(&self) -> String {
        match self.workspace_id {
            Some(ws_id) => format!(
                "(tenant_id = '{}' AND workspace_id = '{}')",
                self.tenant_id, ws_id
            ),
            None => format!("tenant_id = '{}'", self.tenant_id),
        }
    }

    /// Get the tenant ID.
    pub fn tenant_id(&self) -> Uuid {
        self.tenant_id
    }

    /// Get the workspace ID.
    pub fn workspace_id(&self) -> Option<Uuid> {
        self.workspace_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rls_query_builder() {
        let tenant_id = Uuid::new_v4();
        let workspace_id = Uuid::new_v4();

        // Tenant-only scope
        let builder = RlsQueryBuilder::new(tenant_id);
        let clause = builder.where_clause();
        assert!(clause.contains(&tenant_id.to_string()));
        assert!(!clause.contains(&workspace_id.to_string()));

        // With workspace scope
        let builder = RlsQueryBuilder::new(tenant_id).workspace(workspace_id);
        let clause = builder.where_clause();
        assert!(clause.contains(&tenant_id.to_string()));
        assert!(clause.contains(&workspace_id.to_string()));
    }
}
