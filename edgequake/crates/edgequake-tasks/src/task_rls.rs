//! Tenant scope for task writes when the login cannot bypass FORCE RLS.
//!
//! Superuser and BYPASSRLS sessions (Docker, CI) never call [`install_tenant_scope`].
//! A Lakebase service principal does: `tasks` rejects INSERT/UPDATE unless
//! `current_tenant_id()` is set in the same transaction as the statement.

use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use crate::error::{TaskError, TaskResult};

pub async fn session_bypasses_rls(conn: &mut PgConnection) -> TaskResult<bool> {
    let bypass: bool = sqlx::query_scalar(
        "SELECT COALESCE(bool_or(rolsuper OR rolbypassrls), false) \
         FROM pg_roles WHERE rolname = current_user",
    )
    .fetch_one(&mut *conn)
    .await
    .map_err(|e| TaskError::StorageError(format!("Failed to read RLS bypass: {e}")))?;
    Ok(bypass)
}

pub async fn pool_bypasses_rls(pool: &PgPool) -> TaskResult<bool> {
    let mut conn = pool.acquire().await.map_err(|e| {
        TaskError::StorageError(format!("Failed to acquire connection for RLS check: {e}"))
    })?;
    session_bypasses_rls(&mut conn).await
}

pub async fn install_tenant_scope(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    workspace_id: Option<Uuid>,
) -> TaskResult<()> {
    if let Some(workspace) = workspace_id {
        let owned: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM public.workspaces \
             WHERE workspace_id = $1 AND tenant_id = $2 AND is_active)",
        )
        .bind(workspace)
        .bind(tenant_id)
        .fetch_one(&mut *conn)
        .await
        .map_err(|e| TaskError::StorageError(format!("Failed to verify workspace owner: {e}")))?;
        if !owned {
            return Err(TaskError::StorageError(
                "Tenant does not own an active workspace".into(),
            ));
        }
    }
    sqlx::query("SET LOCAL ROLE edgequake_tenant_access")
        .execute(&mut *conn)
        .await
        .map_err(|e| {
            TaskError::StorageError(format!(
                "Cannot assume tenant RLS role (apply migration 167 and grant role to runtime user): {e}"
            ))
        })?;
    let unsafe_role: bool = sqlx::query_scalar(
        "SELECT rolsuper OR rolbypassrls, public.set_tenant_context($1, $2, $3) \
         FROM pg_roles WHERE rolname = current_user",
    )
    .bind(tenant_id)
    .bind(workspace_id)
    .bind(None::<Uuid>)
    .fetch_one(&mut *conn)
    .await
    .map_err(|e| TaskError::StorageError(format!("Failed to set task RLS context: {e}")))?;
    if unsafe_role {
        return Err(TaskError::StorageError(
            "Tenant RLS role must not bypass row security".into(),
        ));
    }
    Ok(())
}

pub async fn active_tenants(conn: &mut PgConnection) -> TaskResult<Vec<Uuid>> {
    sqlx::query_scalar("SELECT tenant_id FROM public.tenants WHERE is_active ORDER BY tenant_id")
        .fetch_all(&mut *conn)
        .await
        .map_err(|e| TaskError::StorageError(format!("Failed to list tenants for task RLS: {e}")))
}
