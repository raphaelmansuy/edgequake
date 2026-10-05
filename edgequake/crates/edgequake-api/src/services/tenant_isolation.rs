//! Tenant isolation SSOT — three defense layers (SPEC-027 phase 35 / SPEC-083 X-37).
//!
//! ## IsolationPolicy (SSOT — code is law)
//!
//! | Surface | Mechanism | Enforcement |
//! |---------|-----------|-------------|
//! | **Relational PG** | RLS GUC `app.current_*` inside `with_rls_transaction` (`is_local=true`) | FORCE RLS + fail-closed policies |
//! | **Graph (AGE)** | Tenant/workspace properties and scoped transactions | Label RLS plus query filters |
//! | **Vectors** | Typed workspace/model scope | RLS on PostgreSQL; mandatory provider scope filters |
//! | **KV** | Workspace-prefixed keys (`wsdoc:`, `staging:hash:`) | Malformed / mixed-workspace upsert rejected in `PostgresKVStorage` |
//! | **WebSocket / tasks** | `WsSession` + `get_task_for_context` | No cross-tenant progress/cancel |
//!
//! ## Isolation layers (code is law)
//!
//! | Layer | Mechanism | Default | SSOT module |
//! |-------|-----------|---------|-------------|
//! | **1 — App (KV/graph)** | Handler filters + metadata match | Always on | `handlers/isolation.rs`, `isolation_context.rs` |
//! | **2 — Auth bind** | JWT/header merge + active membership verify | Required for production auth | `middleware.rs`, `identity_storage.rs` |
//! | **3 — PostgreSQL RLS** | `with_rls_transaction` / `with_optional_pg_rls` | **Default on** | `edgequake-storage/rls.rs`, `conversation.rs` |
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
        let workspace_id = match ctx.workspace_id.as_deref() {
            None => None,
            Some(raw) => Some(resolve_workspace_uuid(Some(raw))?),
        };
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
pub async fn run_with_pg_rls<F, T>(
    pool: &sqlx::PgPool,
    scope: PgIsolationScope,
    operation: F,
) -> Result<T, crate::error::ApiError>
where
    for<'c> F: FnOnce(
            &'c mut sqlx::PgConnection,
        ) -> edgequake_storage::adapters::postgres::RlsTxFuture<'c, T>
        + Send,
    T: Send,
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

/// Run a PG operation with RLS when enabled. Missing scope fails closed.
///
/// SPEC-083 S-03: this is the only supported API-layer entry for tenant-scoped PG.
/// Autocommit `acquire_rls_connection` was removed from this module — GUC must live
/// inside `with_rls_transaction` (via [`run_with_pg_rls`]).
#[cfg(feature = "postgres")]
pub async fn with_optional_pg_rls<F, T>(
    pool: &sqlx::PgPool,
    security: &crate::state::ApiSecurityConfig,
    scope: Option<PgIsolationScope>,
    operation: F,
) -> Result<T, crate::error::ApiError>
where
    for<'c> F: FnOnce(
            &'c mut sqlx::PgConnection,
        ) -> edgequake_storage::adapters::postgres::RlsTxFuture<'c, T>
        + Send,
    T: Send,
{
    if security.pg_rls_enabled {
        let scope = scope.ok_or_else(|| {
            crate::error::ApiError::forbidden_reason("PostgreSQL tenant scope is required")
        })?;
        return run_with_pg_rls(pool, scope, operation).await;
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
