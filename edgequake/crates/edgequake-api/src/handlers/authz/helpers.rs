//! Shared helpers for SPEC-146 authz PAP handlers.

use edgequake_auth::Permission;
use edgequake_authz::AllowSetProvider;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::handlers::auth::RequestAuthContext;
use crate::services::spec146_authz::require_perm;
use crate::state::AppState;

#[cfg(feature = "postgres")]
use sqlx::PgPool;

/// Require PostgreSQL pool (authz PAP is Postgres-only).
#[cfg(feature = "postgres")]
pub(super) fn require_pool(state: &AppState) -> ApiResult<&PgPool> {
    state.pg_pool.as_ref().ok_or_else(|| ApiError::ServiceUnavailable {
        message: "PostgreSQL required for authz APIs".into(),
        retry_after_secs: 5,
    })
}

#[cfg(not(feature = "postgres"))]
pub(super) fn require_pool(_state: &AppState) -> ApiResult<()> {
    Err(ApiError::ServiceUnavailable {
        message: "PostgreSQL required for authz APIs (postgres feature disabled)".into(),
        retry_after_secs: 5,
    })
}

pub(super) fn require_read(auth: &RequestAuthContext) -> ApiResult<()> {
    require_perm(&auth.role, Permission::DocumentListMeta)
}

pub(super) fn require_manage(auth: &RequestAuthContext) -> ApiResult<()> {
    require_perm(&auth.role, Permission::PolicyManage)
}

pub(super) fn require_break_glass(auth: &RequestAuthContext) -> ApiResult<()> {
    // Admin/master paths: BreakGlass OR PolicyManage OR SystemAdmin via admin role.
    if require_perm(&auth.role, Permission::BreakGlass).is_ok() {
        return Ok(());
    }
    require_perm(&auth.role, Permission::PolicyManage)
}

/// Always bump policy_generation after ACL/label/policy/attr mutations.
pub(super) async fn bump_generation(state: &AppState, workspace_id: Uuid) -> ApiResult<u64> {
    let Some(provider) = state.allow_set_provider.as_ref() else {
        // Flag off / no provider: still try direct SQL bump when pool present.
        #[cfg(feature = "postgres")]
        {
            if let Some(pool) = state.pg_pool.as_ref() {
                return direct_bump(pool, workspace_id).await;
            }
        }
        return Ok(1);
    };
    AllowSetProvider::bump_policy_generation(provider.as_ref(), workspace_id)
        .await
        .map_err(|e| ApiError::Internal(format!("bump policy_generation: {e}")))
}

#[cfg(feature = "postgres")]
async fn direct_bump(pool: &PgPool, workspace_id: Uuid) -> ApiResult<u64> {
    let gen: i64 = sqlx::query_scalar(
        r#"
        INSERT INTO workspace_authz_state (workspace_id, policy_generation, updated_at)
        VALUES ($1, 2, NOW())
        ON CONFLICT (workspace_id) DO UPDATE
          SET policy_generation = workspace_authz_state.policy_generation + 1,
              updated_at = NOW()
        RETURNING policy_generation
        "#,
    )
    .bind(workspace_id)
    .fetch_one(pool)
    .await
    .map_err(|e| ApiError::Internal(format!("bump policy_generation: {e}")))?;
    Ok(gen as u64)
}

pub(super) fn sha256_hex(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    hex::encode(hasher.finalize())
}
