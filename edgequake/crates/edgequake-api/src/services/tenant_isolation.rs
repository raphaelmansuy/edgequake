//! Tenant isolation SSOT — three defense layers (SPEC-027 phase 35).
//!
//! ## Isolation layers (code is law)
//!
//! | Layer | Mechanism | Default | SSOT module |
//! |-------|-----------|---------|-------------|
//! | **1 — App (KV/graph)** | Handler filters + metadata match | Always on | `handlers/isolation.rs`, `isolation_context.rs` |
//! | **2 — Auth bind** | JWT/header merge + membership verify | Opt-in | `middleware.rs`, `identity_storage.rs` |
//! | **3 — PostgreSQL RLS** | `acquire_rls_connection` on dedicated conn | **Default on** | `edgequake-storage/rls.rs`, `conversation.rs` |
//!
//! ## Dual KV + PG for auth — First Principles verdict (phase 38)
//!
//! **PostgreSQL is identity SSOT** when `DATABASE_URL` pool is available (default).
//! Bootstrap migrations 048–058 align PG on every deploy. KV mirror env is **ignored** when pool exists.
//!
//! - **PG** — users, memberships, refresh tokens, API keys, RBAC rows, RLS (default on)
//! - **KV** — auth reads/writes only when no PG pool (in-memory tests) or opt-in mirror

use uuid::Uuid;

use crate::middleware::{resolve_tenant_uuid, resolve_workspace_uuid, TenantContext};

/// Resolved tenant/workspace scope for PostgreSQL RLS (parsed UUIDs).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PgIsolationScope {
    pub tenant_id: Uuid,
    pub workspace_id: Option<Uuid>,
    pub user_id: Option<Uuid>,
}

impl PgIsolationScope {
    /// Default tenant/workspace scope for bootstrap and pre-auth PG operations.
    pub fn default_identity(user_id: Option<Uuid>) -> Self {
        let (tenant_id, workspace_id) = crate::services::identity_storage::default_identity_scope();
        Self {
            tenant_id,
            workspace_id: Some(workspace_id),
            user_id,
        }
    }

    /// Build from explicit tenant/workspace/user UUIDs (membership checks).
    pub fn for_membership(tenant_id: Uuid, workspace_id: Uuid, user_id: Uuid) -> Self {
        Self {
            tenant_id,
            workspace_id: Some(workspace_id),
            user_id: Some(user_id),
        }
    }

    /// Build from request tenant context + optional authenticated user id.
    pub fn from_tenant_context(ctx: &TenantContext, user_id: Option<Uuid>) -> Option<Self> {
        let tenant_id = resolve_tenant_uuid(ctx.tenant_id.as_deref())?;
        let workspace_id = resolve_workspace_uuid(ctx.workspace_id.as_deref());
        Some(Self {
            tenant_id,
            workspace_id,
            user_id,
        })
    }
}

/// Attach parsed PG isolation scope to request extensions when context is complete enough.
pub fn attach_pg_isolation_scope(
    request: &mut axum::http::Request<axum::body::Body>,
    ctx: &TenantContext,
    user_id: Option<&str>,
) {
    let user_uuid = user_id.and_then(|id| Uuid::parse_str(id).ok());
    if let Some(scope) = PgIsolationScope::from_tenant_context(ctx, user_uuid) {
        request.extensions_mut().insert(scope);
    }
}

#[cfg(feature = "postgres")]
pub async fn run_with_pg_rls<F, Fut, T>(
    pool: &sqlx::PgPool,
    scope: PgIsolationScope,
    operation: F,
) -> Result<T, crate::error::ApiError>
where
    F: for<'c> FnOnce(&'c mut sqlx::PgConnection) -> Fut,
    Fut: std::future::Future<Output = edgequake_storage::error::Result<T>>,
{
    edgequake_storage::adapters::postgres::with_acquired_tenant_context(
        pool,
        scope.tenant_id,
        scope.workspace_id,
        scope.user_id,
        operation,
    )
    .await
    .map_err(|e| crate::error::ApiError::Internal(format!("PG RLS operation failed: {e}")))
}

/// Acquire a PG connection with optional RLS context (SPEC-027 phase 42).
#[cfg(feature = "postgres")]
pub async fn acquire_optional_pg_connection(
    pool: &sqlx::PgPool,
    security: &crate::state::ApiSecurityConfig,
    scope: Option<PgIsolationScope>,
) -> Result<sqlx::pool::PoolConnection<sqlx::Postgres>, crate::error::ApiError> {
    if security.pg_rls_enabled {
        if let Some(scope) = scope {
            return edgequake_storage::adapters::postgres::acquire_rls_connection(
                pool,
                scope.tenant_id,
                scope.workspace_id,
                scope.user_id,
            )
            .await
            .map_err(|e| crate::error::ApiError::Internal(format!("PG RLS acquire failed: {e}")));
        }
    }

    pool.acquire()
        .await
        .map_err(|e| crate::error::ApiError::Internal(format!("PG acquire failed: {e}")))
}

/// Release a PG connection after an optional RLS operation.
#[cfg(feature = "postgres")]
pub async fn release_optional_pg_connection(
    conn: &mut sqlx::PgConnection,
    security: &crate::state::ApiSecurityConfig,
    scope: Option<PgIsolationScope>,
) {
    if security.pg_rls_enabled && scope.is_some() {
        if let Err(e) = edgequake_storage::adapters::postgres::release_rls_connection(conn).await {
            tracing::warn!(
                target: "edgequake.tenant_isolation",
                error = %e,
                "Failed to clear RLS context after PG operation"
            );
        }
    }
}

/// Run a PG operation with RLS when enabled and scope is present; otherwise plain acquire (SPEC-027 phase 41).
#[cfg(feature = "postgres")]
pub async fn with_optional_pg_rls<F, Fut, T>(
    pool: &sqlx::PgPool,
    security: &crate::state::ApiSecurityConfig,
    scope: Option<PgIsolationScope>,
    operation: F,
) -> Result<T, crate::error::ApiError>
where
    F: for<'c> FnOnce(&'c mut sqlx::PgConnection) -> Fut,
    Fut: std::future::Future<Output = edgequake_storage::error::Result<T>>,
{
    if security.pg_rls_enabled {
        if let Some(scope) = scope {
            return run_with_pg_rls(pool, scope, operation).await;
        }
    }

    let mut conn = pool
        .acquire()
        .await
        .map_err(|e| crate::error::ApiError::Internal(format!("PG acquire failed: {e}")))?;

    operation(&mut conn)
        .await
        .map_err(|e| crate::error::ApiError::Internal(format!("PG operation failed: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pg_scope_from_resolved_context() {
        let ctx = TenantContext {
            tenant_id: Some("default".to_string()),
            workspace_id: Some("default".to_string()),
            user_id: None,
        };
        let scope = PgIsolationScope::from_tenant_context(&ctx, None).expect("default aliases");
        assert_eq!(scope.tenant_id, crate::middleware::default_tenant_uuid());
        assert_eq!(
            scope.workspace_id,
            Some(edgequake_core::default_workspace_uuid())
        );
    }
}
