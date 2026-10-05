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
//! 1. [`with_rls_transaction`] — **preferred** (SPEC-083 S-03): BEGIN → set GUC → work → COMMIT
//! 2. [`with_acquired_tenant_context`] — delegates to `with_rls_transaction`
//! 3. `set_tenant_context_on_conn` / `clear_tenant_context_on_conn` — low-level helpers
//!
//! **Legacy:** [`acquire_rls_connection`] sets transaction-local GUCs (`is_local=true`)
//! outside an explicit `BEGIN`, so the GUC dies when that statement ends. Prefer
//! [`with_rls_transaction`] for all multi-statement (and most single-statement) work.
//!
//! # Example
//!
//! ```ignore
//! use edgequake_storage::adapters::postgres::with_rls_transaction;
//!
//! let row = with_rls_transaction(&pool, tenant_id, workspace_id, Some(user_id), move |conn| {
//!     Box::pin(async move {
//!         sqlx::query_as::<_, MyRow>("SELECT * FROM t WHERE id = $1")
//!             .bind(id)
//!             .fetch_one(&mut *conn)
//!             .await
//!             .map_err(|e| /* ... */)
//!     })
//! }).await?;
//! ```

use std::future::Future;
use std::pin::Pin;

use sqlx::PgPool;
use uuid::Uuid;

use crate::error::{Result, StorageError};

/// Boxed future returned by RLS transaction callbacks (ties Future lifetime to conn).
pub type RlsTxFuture<'c, T> = Pin<Box<dyn Future<Output = Result<T>> + Send + 'c>>;

/// Guard for PostgreSQL RLS context.
///
/// Sets the tenant/workspace context when created, and optionally clears
/// it when dropped (depending on configuration).
#[deprecated(
    since = "0.12.12",
    note = "Pool-level RLS leaks session vars across concurrent checkouts. Use acquire_rls_connection or with_acquired_tenant_context instead (SPEC-027 SEC-014)."
)]
#[derive(Debug)]
#[allow(dead_code)] // deprecated surface retained for API compatibility (SPEC-090 F-090-18)
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
        // SPEC-090 F-090-18: no-op — pool-level clear_on_drop raced wrong connections.
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
    sqlx::query("SELECT public.clear_tenant_context()")
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
    sqlx::query("SELECT public.set_tenant_context($1, $2, $3)")
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

/// Establish the mandatory RLS role and transaction-local context on an open transaction.
/// SET LOCAL ROLE is rolled back by SQLx on cancellation as well as normal completion.
pub async fn enforce_tenant_context(
    conn: &mut sqlx::PgConnection,
    tenant_id: Uuid,
    workspace_id: Option<Uuid>,
    user_id: Option<Uuid>,
) -> Result<()> {
    if let Some(workspace) = workspace_id {
        let owned: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM public.workspaces WHERE workspace_id=$1 AND tenant_id=$2 AND is_active)")
            .bind(workspace).bind(tenant_id).fetch_one(&mut *conn).await.map_err(StorageError::from)?;
        if !owned {
            return Err(StorageError::InvalidInput(
                "Tenant does not own an active workspace".into(),
            ));
        }
    }
    install_rls_context(conn, tenant_id, workspace_id, user_id).await
}

/// Role demotion and context installation share one implementation across scoped ports.
async fn install_rls_context(
    conn: &mut sqlx::PgConnection,
    tenant_id: Uuid,
    workspace_id: Option<Uuid>,
    user_id: Option<Uuid>,
) -> Result<()> {
    sqlx::query("SET LOCAL ROLE edgequake_tenant_access").execute(&mut *conn).await
        .map_err(|e| StorageError::Database(format!("Cannot assume tenant RLS role (apply migration 167 and grant role to runtime user): {e}")))?;
    let unsafe_role: bool = sqlx::query_scalar(
        "SELECT rolsuper OR rolbypassrls, public.set_tenant_context($1, $2, $3) \
         FROM pg_roles WHERE rolname = current_user",
    )
    .bind(tenant_id)
    .bind(workspace_id)
    .bind(user_id)
    .fetch_one(&mut *conn)
    .await
    .map_err(StorageError::from)?;
    if unsafe_role {
        return Err(StorageError::InvalidConfig(
            "Tenant RLS role must not bypass row security".into(),
        ));
    }
    Ok(())
}

/// Authority operations take AccessScope at their boundary and own one scoped transaction.
pub async fn begin_tenant_transaction(
    pool: &PgPool,
    scope: &edgequake_storage_contracts::AccessScope,
) -> Result<sqlx::Transaction<'static, sqlx::Postgres>> {
    let mut tx = pool.begin().await.map_err(StorageError::from)?;
    enforce_tenant_context(
        &mut tx,
        scope.tenant().into_uuid(),
        Some(scope.workspace().into_uuid()),
        None,
    )
    .await?;
    Ok(tx)
}

/// Older typed vector ports carry a workspace and optional tenant. Resolve its
/// owner before demotion, then enforce both dimensions inside the held transaction.
pub(crate) async fn enforce_workspace_context(
    conn: &mut sqlx::PgConnection,
    tenant: Option<Uuid>,
    workspace: Option<Uuid>,
) -> Result<()> {
    let workspace = workspace.ok_or_else(|| {
        StorageError::InvalidInput("Vector search requires a workspace scope".into())
    })?;
    let owner: Option<Uuid> = sqlx::query_scalar(
        "SELECT tenant_id FROM public.workspaces WHERE workspace_id=$1 AND is_active",
    )
    .bind(workspace)
    .fetch_optional(&mut *conn)
    .await
    .map_err(StorageError::from)?;
    let owner =
        owner.ok_or_else(|| StorageError::InvalidInput("Unknown workspace scope".into()))?;
    if tenant.is_some_and(|tenant| tenant != owner) {
        return Err(StorageError::InvalidInput(
            "Tenant does not own workspace scope".into(),
        ));
    }
    install_rls_context(conn, owner, Some(workspace), None).await
}

/// Acquire a pooled connection with RLS tenant context set (SPEC-027 SEC-014).
///
/// **Legacy** — prefer [`with_rls_transaction`]. `set_tenant_context` uses
/// `set_config(..., is_local = true)`, so the GUC is cleared when the setting
/// statement ends unless it runs inside an explicit `BEGIN`…`COMMIT`.
#[deprecated(
    note = "SPEC-083 S-03: use with_rls_transaction — is_local=true GUC dies outside BEGIN"
)]
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

/// Run an operation inside an explicit transaction with RLS GUC set (SPEC-083 S-03).
///
/// # Why this exists
///
/// `set_tenant_context()` uses `set_config(..., is_local = true)` (transaction-local GUC).
/// Calling it in autocommit clears the GUC when that statement ends, so the next query
/// sees `current_tenant_id() = NULL` and RLS policies never match.
///
/// **Invariant**: GUC MUST be set inside `BEGIN` … `COMMIT` on the same connection.
/// Prefer this helper (or [`with_acquired_tenant_context`], which delegates here) over
/// bare `acquire_rls_connection` + autocommit queries.
pub async fn with_rls_transaction<F, T>(
    pool: &PgPool,
    tenant_id: Uuid,
    workspace_id: Option<Uuid>,
    user_id: Option<Uuid>,
    operation: F,
) -> Result<T>
where
    for<'c> F: FnOnce(&'c mut sqlx::PgConnection) -> RlsTxFuture<'c, T> + Send,
    T: Send,
{
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| StorageError::Database(format!("Failed to begin RLS transaction: {e}")))?;

    // GUC is transaction-local — must run after BEGIN (see module docs / migration 096).
    // WHY `&mut *tx`: sqlx Executor is implemented for `&mut PgConnection`, not
    // `&mut Transaction` in this sqlx version — explicit deref is required.
    #[allow(clippy::explicit_auto_deref)]
    {
        enforce_tenant_context(&mut tx, tenant_id, workspace_id, user_id).await?;
    }

    #[allow(clippy::explicit_auto_deref)]
    let op_result = operation(&mut *tx).await;
    match op_result {
        Ok(value) => {
            tx.commit().await.map_err(|e| {
                StorageError::Database(format!("Failed to commit RLS transaction: {e}"))
            })?;
            Ok(value)
        }
        Err(err) => {
            if let Err(rollback_err) = tx.rollback().await {
                tracing::warn!(
                    error.source = "postgres_rls",
                    error.message = %rollback_err,
                    "Failed to rollback RLS transaction after operation error"
                );
            }
            Err(err)
        }
    }
}

/// Run an operation with RLS context on a **single acquired connection** (SPEC-027 SEC-014).
///
/// Delegates to [`with_rls_transaction`] so GUCs remain visible for the whole operation
/// (SPEC-083 S-03). Prefer this or `with_rls_transaction` over pool-level context.
pub async fn with_acquired_tenant_context<F, T>(
    pool: &PgPool,
    tenant_id: Uuid,
    workspace_id: Option<Uuid>,
    user_id: Option<Uuid>,
    operation: F,
) -> Result<T>
where
    for<'c> F: FnOnce(&'c mut sqlx::PgConnection) -> RlsTxFuture<'c, T> + Send,
    T: Send,
{
    with_rls_transaction(pool, tenant_id, workspace_id, user_id, operation).await
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

    /// Parameterized WHERE clause for manual query building.
    ///
    /// Returns `(sql_with_placeholders, bind_values)` — use `$1`, `$2`, … in order.
    pub fn where_clause(&self) -> (String, Vec<Uuid>) {
        match self.workspace_id {
            Some(ws_id) => (
                "(tenant_id = $1 AND workspace_id = $2)".to_string(),
                vec![self.tenant_id, ws_id],
            ),
            None => ("tenant_id = $1".to_string(), vec![self.tenant_id]),
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
        let (clause, binds) = builder.where_clause();
        assert_eq!(clause, "tenant_id = $1");
        assert_eq!(binds, vec![tenant_id]);

        // With workspace scope
        let builder = RlsQueryBuilder::new(tenant_id).workspace(workspace_id);
        let (clause, binds) = builder.where_clause();
        assert_eq!(clause, "(tenant_id = $1 AND workspace_id = $2)");
        assert_eq!(binds, vec![tenant_id, workspace_id]);
    }
}
