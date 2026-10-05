//! Tenant lifecycle authorization (SPEC-158 LAW-158-6 / EC-158-14).
//!
//! * create / delete — platform admin only.
//! * update — platform admin, or tenant owner/admin member.
//! * read / list — platform admin; otherwise members only when membership scoping is on
//!   (SSO active or `EDGEQUAKE_STRICT_TENANT_BIND`); legacy single-tenant installs keep read access.

use edgequake_core::{MembershipRole, Tenant};
use uuid::Uuid;

use crate::error::ApiError;
use crate::handlers::auth::RequestAuthContext;
use crate::state::AppState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TenantAccess {
    Read,
    Manage,
}

/// True when only members may see a tenant (multi-tenant / SSO deployments).
pub fn membership_scoped(state: &AppState) -> bool {
    crate::services::workspace_visibility::membership_scoped(state)
}

fn is_platform_admin(ctx: &RequestAuthContext) -> bool {
    matches!(ctx.role, edgequake_auth::Role::Admin)
}

fn manages(role: MembershipRole) -> bool {
    matches!(role, MembershipRole::Owner | MembershipRole::Admin)
}

fn denied() -> ApiError {
    ApiError::forbidden_reason("tenant_access_denied")
}

/// Platform admin only (create / delete tenant).
pub fn require_platform_admin(ctx: &RequestAuthContext) -> Result<(), ApiError> {
    if is_platform_admin(ctx) {
        Ok(())
    } else {
        Err(ApiError::forbidden_reason("Admin role required"))
    }
}

/// Authorize `ctx` for `need` on one tenant.
pub async fn require_tenant_access(
    state: &AppState,
    ctx: &RequestAuthContext,
    tenant_id: Uuid,
    need: TenantAccess,
) -> Result<(), ApiError> {
    if is_platform_admin(ctx) || (need == TenantAccess::Read && !membership_scoped(state)) {
        return Ok(());
    }
    let tenant = state
        .workspace_service
        .get_tenant(tenant_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(denied)?;
    if !tenant.is_active {
        return Err(denied());
    }
    let user_id = Uuid::parse_str(&ctx.user_id).map_err(|_| denied())?;
    if need == TenantAccess::Read
        && state
            .workspace_service
            .get_user_memberships(user_id)
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?
            .iter()
            .any(|m| m.is_active && m.tenant_id == tenant_id)
    {
        return Ok(());
    }
    let role = state
        .workspace_service
        .get_user_role(user_id, tenant_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(denied)?;
    match need {
        TenantAccess::Read => Ok(()),
        TenantAccess::Manage if manages(role) => Ok(()),
        TenantAccess::Manage => Err(denied()),
    }
}

/// Tenants visible to a member (active tenants they hold a membership in).
pub async fn member_tenants(
    state: &AppState,
    ctx: &RequestAuthContext,
) -> Result<Vec<Tenant>, ApiError> {
    let Ok(user_id) = Uuid::parse_str(&ctx.user_id) else {
        return Ok(Vec::new());
    };
    let internal = |e: edgequake_core::Error| ApiError::Internal(e.to_string());
    let mut seen = std::collections::HashSet::new();
    let mut tenants = Vec::new();
    for m in state
        .workspace_service
        .get_user_memberships(user_id)
        .await
        .map_err(internal)?
    {
        if m.is_active && seen.insert(m.tenant_id) {
            if let Some(t) = state
                .workspace_service
                .get_tenant(m.tenant_id)
                .await
                .map_err(internal)?
            {
                if t.is_active {
                    tenants.push(t);
                }
            }
        }
    }
    tenants.sort_by_key(|t| t.created_at);
    Ok(tenants)
}
