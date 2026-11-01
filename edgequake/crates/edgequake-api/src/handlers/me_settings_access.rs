//! SPEC-175 — `GET /api/v1/me/settings-access`: the caller's grants for the settings shell.
//!
//! The server is the authority (LAW-175-13). The WebUI decides what to *offer* from these
//! flags; every write is still checked by its own handler. This endpoint reports only the
//! caller's own grants, so it is exempt from membership binding (see `global_resource`).
//!
//! Grant rules are pure functions over memberships, so they are unit-tested without I/O.

use axum::extract::Query;
use axum::{extract::State, Json};
use edgequake_core::{Membership, MembershipRole, Workspace};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::ApiError;
use crate::handlers::auth::{ApiAuthenticated, RequestAuthContext};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct SettingsAccessQuery {
    pub tenant_id: Option<Uuid>,
    pub workspace_id: Option<Uuid>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SettingsAccessResponse {
    pub user_id: String,
    /// Platform admin: the `Admin` global role.
    pub instance_admin: bool,
    /// Highest tenant-wide membership role (`owner|admin|member|readonly`), if any.
    pub tenant_role: Option<String>,
    /// Super admin rights in this tenant (tenant owner/admin, or platform admin).
    pub tenant_manage: bool,
    pub workspace_id: Option<String>,
    /// Owner or admin of the requested workspace, or a tenant manager.
    pub workspace_manage: bool,
    /// Workspaces the caller may administer in this tenant.
    pub administered_workspace_ids: Vec<String>,
}

/// Tenant- and workspace-level management grants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Grants {
    pub tenant_manage: bool,
    pub workspace_manage: bool,
}

/// Derive grants from the caller's strongest tenant-wide role and workspace role.
/// Platform admin manages everything. A tenant-wide owner or admin manages every
/// workspace of the tenant. A workspace-scoped owner or admin manages only that workspace.
pub fn grants(
    platform_admin: bool,
    tenant_wide: Option<MembershipRole>,
    workspace_role: Option<MembershipRole>,
) -> Grants {
    let tenant_manage = platform_admin || tenant_wide.is_some_and(|r| r.can_manage_users());
    let workspace_manage =
        tenant_manage || workspace_role.is_some_and(|r| r.can_manage_workspaces());
    Grants {
        tenant_manage,
        workspace_manage,
    }
}

/// Workspaces the caller may administer. Pure over memberships and the tenant's workspaces.
pub fn administered_workspace_ids(
    platform_admin: bool,
    tenant: Uuid,
    memberships: &[Membership],
    workspaces: &[Workspace],
) -> Vec<Uuid> {
    let tenant_wide = strongest_role(memberships, tenant, None);
    if platform_admin || tenant_wide.is_some_and(|r| r.can_manage_users()) {
        return workspaces
            .iter()
            .filter(|w| w.is_active && w.tenant_id == tenant)
            .map(|w| w.workspace_id)
            .collect();
    }
    let mut ids: Vec<Uuid> = memberships
        .iter()
        .filter(|m| m.is_active && m.tenant_id == tenant && m.role.can_manage_workspaces())
        .filter_map(|m| m.workspace_id)
        .filter(|ws| {
            workspaces
                .iter()
                .any(|w| w.workspace_id == *ws && w.is_active && w.tenant_id == tenant)
        })
        .collect();
    ids.sort();
    ids.dedup();
    ids
}

/// What the caller may do in one tenant and, optionally, one workspace (LAW-175-13).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CallerScope {
    pub grants: Grants,
    /// Platform admin, tenant-wide member, or a member of the workspace.
    pub member: bool,
}

/// Load the caller's scope for `tenant` / `workspace` from live memberships.
/// Fails closed: an inactive tenant grants nothing to non-platform admins.
pub(crate) async fn caller_scope(
    state: &AppState,
    auth: &RequestAuthContext,
    tenant: Uuid,
    workspace: Option<Uuid>,
) -> Result<CallerScope, ApiError> {
    let memberships = match Uuid::parse_str(&auth.user_id) {
        Ok(user) => state
            .workspace_service
            .get_user_memberships(user)
            .await
            .map_err(internal)?,
        Err(_) => Vec::new(),
    };
    let tenant_active = state
        .workspace_service
        .get_tenant(tenant)
        .await
        .map_err(internal)?
        .is_some_and(|t| t.is_active);
    Ok(scope_for(
        matches!(auth.role, edgequake_auth::Role::Admin),
        tenant_active,
        &memberships,
        tenant,
        workspace,
    ))
}

/// Pure scope rule over loaded memberships. Platform admins keep grants even when the
/// tenant is inactive, but membership-derived access requires an active tenant.
pub fn scope_for(
    platform_admin: bool,
    tenant_active: bool,
    memberships: &[Membership],
    tenant: Uuid,
    workspace: Option<Uuid>,
) -> CallerScope {
    let effective: &[Membership] = if tenant_active { memberships } else { &[] };
    let tenant_wide = strongest_role(effective, tenant, None);
    let workspace_role = workspace.and_then(|ws| strongest_role(effective, tenant, Some(ws)));
    CallerScope {
        grants: grants(platform_admin, tenant_wide, workspace_role),
        member: (platform_admin && tenant_active)
            || tenant_wide.is_some()
            || workspace_role.is_some(),
    }
}

/// Strongest role among active memberships in `tenant` that match `workspace`.
/// `None` means tenant-wide rows only (workspace_id is null).
///
/// SPEC-181: `pub(crate)` so `me/access` reuses the one grant algebra instead of restating it.
pub(crate) fn strongest_role(
    memberships: &[Membership],
    tenant: Uuid,
    workspace: Option<Uuid>,
) -> Option<MembershipRole> {
    memberships
        .iter()
        .filter(|m| m.is_active && m.tenant_id == tenant)
        .filter(|m| match workspace {
            Some(ws) => m.applies_to(tenant, Some(ws)),
            None => m.workspace_id.is_none(),
        })
        .map(|m| m.role)
        .max_by_key(MembershipRole::level)
}

fn role_label(role: MembershipRole) -> &'static str {
    match role {
        MembershipRole::Owner => "owner",
        MembershipRole::Admin => "admin",
        MembershipRole::Member => "member",
        MembershipRole::Readonly => "readonly",
    }
}

fn internal(e: edgequake_core::Error) -> ApiError {
    ApiError::Internal(e.to_string())
}

/// `GET /api/v1/me/settings-access?tenant_id=&workspace_id=`
#[utoipa::path(
    get,
    path = "/api/v1/me/settings-access",
    tag = "Settings",
    params(
        ("tenant_id" = Option<Uuid>, Query, description = "Tenant to check; defaults to the default tenant"),
        ("workspace_id" = Option<Uuid>, Query, description = "Workspace to check for workspace-level management"),
    ),
    responses(
        (status = 200, description = "The caller's settings grants", body = SettingsAccessResponse),
    )
)]
pub async fn get_settings_access(
    State(state): State<AppState>,
    ApiAuthenticated(auth): ApiAuthenticated,
    Query(query): Query<SettingsAccessQuery>,
) -> Result<Json<SettingsAccessResponse>, ApiError> {
    let platform_admin = matches!(auth.role, edgequake_auth::Role::Admin);
    let tenant = query
        .tenant_id
        .unwrap_or_else(crate::middleware::default_tenant_uuid);
    let memberships = match Uuid::parse_str(&auth.user_id) {
        Ok(user) => state
            .workspace_service
            .get_user_memberships(user)
            .await
            .map_err(internal)?,
        Err(_) => Vec::new(),
    };
    let tenant_active = state
        .workspace_service
        .get_tenant(tenant)
        .await
        .map_err(internal)?
        .is_some_and(|t| t.is_active);
    // Fail closed: an unknown or inactive tenant grants nothing to non-platform admins.
    let effective_memberships: &[Membership] = if tenant_active { &memberships } else { &[] };
    let workspaces = if tenant_active {
        state
            .workspace_service
            .list_workspaces(tenant)
            .await
            .map_err(internal)?
    } else {
        Vec::new()
    };
    let selected = query.workspace_id.filter(|ws| {
        workspaces
            .iter()
            .any(|w| w.workspace_id == *ws && w.is_active)
    });

    let tenant_wide = strongest_role(effective_memberships, tenant, None);
    let workspace_role =
        selected.and_then(|ws| strongest_role(effective_memberships, tenant, Some(ws)));
    let decided = grants(platform_admin, tenant_wide, workspace_role);
    let admin_ids =
        administered_workspace_ids(platform_admin, tenant, effective_memberships, &workspaces);

    Ok(Json(SettingsAccessResponse {
        user_id: auth.user_id.clone(),
        instance_admin: platform_admin,
        tenant_role: tenant_wide.map(|r| role_label(r).to_string()),
        tenant_manage: decided.tenant_manage,
        workspace_id: selected.map(|ws| ws.to_string()),
        workspace_manage: decided.workspace_manage,
        administered_workspace_ids: admin_ids.iter().map(Uuid::to_string).collect(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::collections::HashMap;

    fn membership(tenant: Uuid, workspace: Option<Uuid>, role: MembershipRole) -> Membership {
        Membership {
            membership_id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            tenant_id: tenant,
            workspace_id: workspace,
            role,
            is_active: true,
            joined_at: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    #[test]
    fn scope_workspace_admin_is_member_and_manager_of_its_workspace_only() {
        let tenant = Uuid::new_v4();
        let ws = Uuid::new_v4();
        let other = Uuid::new_v4();
        let mems = [membership(tenant, Some(ws), MembershipRole::Admin)];
        let here = scope_for(false, true, &mems, tenant, Some(ws));
        assert!(here.member && here.grants.workspace_manage && !here.grants.tenant_manage);
        let elsewhere = scope_for(false, true, &mems, tenant, Some(other));
        assert!(!elsewhere.member && !elsewhere.grants.workspace_manage);
    }

    #[test]
    fn scope_readonly_member_reads_but_cannot_manage() {
        let tenant = Uuid::new_v4();
        let ws = Uuid::new_v4();
        let mems = [membership(tenant, Some(ws), MembershipRole::Readonly)];
        let scope = scope_for(false, true, &mems, tenant, Some(ws));
        assert!(scope.member);
        assert!(!scope.grants.workspace_manage);
    }

    #[test]
    fn scope_tenant_wide_member_reaches_every_workspace_as_member() {
        let tenant = Uuid::new_v4();
        let ws = Uuid::new_v4();
        let mems = [membership(tenant, None, MembershipRole::Member)];
        let scope = scope_for(false, true, &mems, tenant, Some(ws));
        assert!(scope.member);
        assert!(!scope.grants.workspace_manage);
    }

    #[test]
    fn scope_inactive_tenant_grants_membership_nothing() {
        let tenant = Uuid::new_v4();
        let mems = [membership(tenant, None, MembershipRole::Owner)];
        let scope = scope_for(false, false, &mems, tenant, None);
        assert!(!scope.member && !scope.grants.tenant_manage);
    }

    #[test]
    fn scope_non_member_of_tenant_is_not_a_member() {
        let scope = scope_for(false, true, &[], Uuid::new_v4(), Some(Uuid::new_v4()));
        assert!(!scope.member);
    }

    #[test]
    fn scope_platform_admin_is_member_and_manager_when_tenant_active() {
        let scope = scope_for(true, true, &[], Uuid::new_v4(), Some(Uuid::new_v4()));
        assert!(scope.member && scope.grants.workspace_manage);
    }

    #[test]
    fn platform_admin_manages_everything() {
        let g = grants(true, None, None);
        assert!(g.tenant_manage && g.workspace_manage);
    }

    #[test]
    fn tenant_owner_manages_tenant_and_workspaces() {
        let g = grants(false, Some(MembershipRole::Owner), None);
        assert!(g.tenant_manage && g.workspace_manage);
    }

    #[test]
    fn workspace_admin_manages_only_workspaces() {
        let g = grants(false, None, Some(MembershipRole::Admin));
        assert!(!g.tenant_manage);
        assert!(g.workspace_manage);
    }

    #[test]
    fn member_and_readonly_manage_nothing() {
        assert_eq!(
            grants(
                false,
                Some(MembershipRole::Member),
                Some(MembershipRole::Member)
            ),
            Grants {
                tenant_manage: false,
                workspace_manage: false
            }
        );
        assert_eq!(
            grants(false, Some(MembershipRole::Readonly), None),
            Grants {
                tenant_manage: false,
                workspace_manage: false
            }
        );
    }

    #[test]
    fn tenant_admin_lists_every_active_workspace() {
        let tenant = Uuid::new_v4();
        let ws1 = Uuid::new_v4();
        let ws2 = Uuid::new_v4();
        let memberships = vec![membership(tenant, None, MembershipRole::Admin)];
        let workspaces = vec![ws_row(ws1, tenant, true), ws_row(ws2, tenant, true)];
        let ids = administered_workspace_ids(false, tenant, &memberships, &workspaces);
        assert_eq!(ids.len(), 2);
    }

    #[test]
    fn workspace_member_lists_nothing_and_admin_lists_only_theirs() {
        let tenant = Uuid::new_v4();
        let mine = Uuid::new_v4();
        let other = Uuid::new_v4();
        let workspaces = vec![ws_row(mine, tenant, true), ws_row(other, tenant, true)];
        let memberships = vec![
            membership(tenant, Some(mine), MembershipRole::Admin),
            membership(tenant, Some(other), MembershipRole::Member),
        ];
        assert_eq!(
            administered_workspace_ids(false, tenant, &memberships, &workspaces),
            vec![mine]
        );
    }

    #[test]
    fn inactive_workspaces_and_foreign_tenants_are_ignored() {
        let tenant = Uuid::new_v4();
        let other_tenant = Uuid::new_v4();
        let ws = Uuid::new_v4();
        let workspaces = vec![ws_row(ws, tenant, false)];
        let memberships = vec![membership(other_tenant, Some(ws), MembershipRole::Owner)];
        assert!(administered_workspace_ids(false, tenant, &memberships, &workspaces).is_empty());
    }

    fn ws_row(id: Uuid, tenant: Uuid, active: bool) -> Workspace {
        let mut ws = Workspace::new(tenant, "w", "w");
        ws.workspace_id = id;
        ws.is_active = active;
        ws
    }
}
