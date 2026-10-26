//! SPEC-180 access services. HTTP, upload and bulk adapters share these boundaries.
mod listing;
mod members;
mod mutation;
mod read;
mod task_listing;
mod upload;
pub use listing::*;
pub use members::*;
pub use mutation::*;
pub use read::*;
pub use task_listing::*;
pub use upload::*;

use crate::{
    error::{ApiError, ApiResult},
    middleware::TenantContext,
};
use axum::http::StatusCode;
use edgequake_core::clearance::VisibilityScope;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

#[derive(Clone)]
pub struct AccessActor {
    pub tenant: Uuid,
    pub workspace: Uuid,
    pub user: Option<Uuid>,
    pub actor: String,
}
impl AccessActor {
    pub fn from_context(ctx: &TenantContext) -> ApiResult<Self> {
        let parse = |value: Option<&str>| {
            value.and_then(|v| Uuid::parse_str(v).ok()).ok_or_else(|| {
                ApiError::BadRequest("Authorized tenant and workspace are required".into())
            })
        };
        let user = ctx
            .user_id
            .as_deref()
            .and_then(|v| Uuid::parse_str(v).ok())
            .or_else(|| {
                edgequake_storage::clearance_session::ClearanceSession::current()
                    .and_then(|s| s.user_id)
            });
        Ok(Self {
            tenant: parse(ctx.tenant_id.as_deref())?,
            workspace: parse(ctx.workspace_id.as_deref())?,
            actor: user
                .map(|v| v.to_string())
                .unwrap_or_else(|| "service".into()),
            user,
        })
    }
}
pub fn coded(status: StatusCode, code: &'static str, message: &str) -> ApiError {
    ApiError::Coded {
        status,
        code,
        message: message.into(),
    }
}
pub fn database(error: sqlx::Error) -> ApiError {
    tracing::error!(%error, "Document access database operation failed");
    if let Some(db) = error.as_database_error() {
        match db.code().as_deref() {
            Some("40001" | "40P01") => {
                return coded(
                    StatusCode::CONFLICT,
                    "POLICY_CHANGED",
                    "Access rules changed. Review the latest values",
                )
            }
            Some("55P03") => {
                return coded(
                    StatusCode::CONFLICT,
                    "OPERATION_IN_PROGRESS",
                    "Access is being updated. Check status before retrying",
                )
            }
            Some("23514") if db.message().contains("ACCESS_CHANGE_TOO_LARGE") => {
                return ApiError::unprocessable_coded(
                    "ACCESS_CHANGE_TOO_LARGE",
                    "Select fewer documents for this access change",
                )
            }
            _ => {}
        }
    }
    ApiError::Internal("Document access could not be saved".into())
}

pub async fn scoped_transaction(
    pool: &PgPool,
    actor: &AccessActor,
    lock: bool,
) -> ApiResult<(Transaction<'static, Postgres>, VisibilityScope)> {
    let mut tx = pool.begin().await.map_err(database)?;
    if !lock {
        sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ")
            .execute(&mut *tx)
            .await
            .map_err(database)?;
    }
    edgequake_storage::adapters::postgres::rls::enforce_tenant_context(
        &mut tx,
        actor.tenant,
        Some(actor.workspace),
        actor.user,
    )
    .await?;
    if lock {
        sqlx::query("SELECT workspace_id FROM public.workspace_clearance_settings WHERE workspace_id=$1 FOR UPDATE")
            .bind(actor.workspace).execute(&mut *tx).await.map_err(database)?;
    }
    let (scope, session) =
        crate::services::clearance::load_scope_conn(&mut tx, actor.workspace, actor.user).await?;
    edgequake_storage::adapters::postgres::rls::apply_clearance_session(&mut tx, &session).await?;
    // Enforcement-off legacy admission retains the route's existing authority.
    // Enforced admissions and mutations refresh RBAC inside the locked boundary.
    if lock && scope.subject.enforcement && !capabilities(&mut tx, actor, &scope).await?.can_edit {
        return Err(coded(
            StatusCode::FORBIDDEN,
            "ACCESS_CHANGE_FORBIDDEN",
            "Your workspace role cannot change document access",
        ));
    }
    Ok((tx, scope))
}

pub async fn require_writer(pool: &PgPool, actor: &AccessActor) -> ApiResult<()> {
    let role = if let Some(user) = actor.user {
        crate::services::clearance::membership_role(pool, user, actor.workspace).await?
    } else {
        None
    };
    if matches!(
        role.as_deref(),
        Some("owner" | "admin" | "member" | "editor")
    ) {
        return Ok(());
    }
    Err(coded(
        StatusCode::FORBIDDEN,
        "ACCESS_CHANGE_FORBIDDEN",
        "Your workspace role cannot change document access",
    ))
}

pub fn require_epoch(scope: &VisibilityScope, epoch: i64) -> ApiResult<()> {
    if !scope.subject.enforcement {
        return Err(ApiError::unprocessable_coded(
            "CLEARANCE_DISABLED",
            "Document access rules are off for this workspace",
        ));
    }
    if scope.policy_epoch != epoch {
        return Err(coded(
            StatusCode::CONFLICT,
            "POLICY_CHANGED",
            "Access rules changed. Review the latest values",
        ));
    }
    Ok(())
}

pub async fn audit(
    tx: &mut Transaction<'_, Postgres>,
    actor: &AccessActor,
    id: Uuid,
    action: &str,
    metadata: serde_json::Value,
) -> ApiResult<()> {
    sqlx::query("INSERT INTO public.audit_logs (tenant_id,workspace_id,user_id,event_type,event_category,event_action,resource_type,resource_id,result,severity,metadata) VALUES ($1,$2,$3,'Authorization','document_clearance',$4,'document',$5,'Success','Medium',$6)")
        .bind(actor.tenant.to_string()).bind(actor.workspace.to_string()).bind(&actor.actor).bind(action).bind(id.to_string()).bind(metadata)
        .execute(&mut **tx).await.map_err(database)?;
    Ok(())
}

#[cfg(test)]
mod tests;
