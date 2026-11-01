//! `GET /api/v1/me/access` (SPEC-181) — everything a surface needs to decide what to offer.
//!
//! One call answers "what may I do, in which scope, and how much room is left". The WebUI reads
//! nothing else about rights, so a capability cannot drift between the button and the handler
//! that would refuse it ([LAW-181-1](specs/181-admin/01-first-principles.md),
//! [LAW-181-3](specs/181-admin/01-first-principles.md)).
//!
//! The grant algebra lives in [`crate::handlers::me_settings_access::scope_for`], which is pure
//! and already unit-tested (SPEC-175). This handler only loads rows and projects them.

use axum::extract::{Query, State};
use axum::Json;
use edgequake_core::{Membership, MembershipRole, Workspace};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::ApiError;
use crate::handlers::auth::{ApiAuthenticated, RequestAuthContext};
use crate::handlers::me_settings_access::strongest_role;
use crate::services::capabilities::{
    capabilities_for_platform, capabilities_for_role, Capability, CapabilityAxis,
};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct MeAccessQuery {
    pub tenant_id: Option<Uuid>,
    pub workspace_id: Option<Uuid>,
}

/// One budget: how much of a plan limit is consumed.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct Budget {
    pub used: usize,
    /// `0` means unlimited.
    pub limit: usize,
}

impl Budget {
    fn new(used: usize, limit: usize) -> Self {
        Self { used, limit }
    }

    /// `true` when the caller may still create one more.
    pub fn has_room(&self) -> bool {
        self.limit == 0 || self.used < self.limit
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PlatformAccess {
    pub role: String,
    pub admin: bool,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct OrganizationAccess {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    /// Effective organization role. `None` for a person whose only grants are workspace-scoped.
    pub role: Option<String>,
    pub level: u8,
    pub is_active: bool,
    /// `true` when the organization is archived: it grants nothing and must be restored first.
    pub is_archived: bool,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct WorkspaceAccess {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    /// Effective role, after the organization's implicit grant.
    pub role: String,
    pub level: u8,
    /// `true` when the role comes from the organization grant rather than a workspace row.
    pub inherited: bool,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct QuotaAccess {
    pub workspaces: Budget,
    pub seats: Budget,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CreatePermissions {
    pub organization: bool,
    pub workspace: bool,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct DeletePermissions {
    pub organization: bool,
}

/// Why a create action is unavailable, so the UI can choose *absent* (role) versus *disabled
/// with a fix* (quota). `None` when the action is available.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CreateReason {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MeAccessResponse {
    pub user_id: String,
    pub platform: PlatformAccess,
    /// `None` when the caller holds no organization grant and no platform grant.
    pub organization: Option<OrganizationAccess>,
    /// Only workspaces the caller can read.
    pub workspaces: Vec<WorkspaceAccess>,
    pub quota: QuotaAccess,
    pub can_create: CreatePermissions,
    pub can_delete: DeletePermissions,
    pub reason: CreateReason,
}

/// Pure projection of loaded rows into the response.
///
/// Kept free of I/O so the whole matrix can be asserted in a unit test with synthetic rows.
pub struct AccessInputs<'a> {
    pub user_id: &'a str,
    pub platform_role: edgequake_auth::Role,
    pub tenant: Option<&'a edgequake_core::Tenant>,
    pub memberships: &'a [Membership],
    pub workspaces: &'a [Workspace],
    pub seats_used: usize,
}

pub fn build_me_access(input: AccessInputs<'_>) -> MeAccessResponse {
    let platform_admin = matches!(input.platform_role, edgequake_auth::Role::Admin);
    let platform_role_id = input.platform_role.as_str().to_string();
    let platform_caps: Vec<String> = capabilities_for_platform(input.platform_role)
        .into_iter()
        .map(|c| c.id().to_string())
        .collect();

    let platform = PlatformAccess {
        role: platform_role_id,
        admin: platform_admin,
        capabilities: platform_caps,
    };

    let Some(tenant) = input.tenant else {
        // No organization selected (or the caller has none). A platform admin can still create.
        return MeAccessResponse {
            user_id: input.user_id.to_string(),
            platform,
            organization: None,
            workspaces: Vec::new(),
            quota: QuotaAccess {
                workspaces: Budget::new(0, 0),
                seats: Budget::new(0, 0),
            },
            can_create: CreatePermissions {
                organization: platform_admin,
                workspace: false,
            },
            can_delete: DeletePermissions {
                organization: false,
            },
            reason: CreateReason {
                organization: None,
                workspace: Some("no_organization".into()),
            },
        };
    };

    let tenant_active = tenant.is_active && tenant.archived_at.is_none();
    // Fail closed: a live tenant's rows decide; an archived one grants a non-admin nothing.
    let effective: &[Membership] = if tenant_active {
        input.memberships
    } else {
        &[]
    };

    let active_workspaces: Vec<&Workspace> = input
        .workspaces
        .iter()
        // SPEC-181: archived workspaces are not offered; they are restored or purged.
        .filter(|w| w.is_active && w.archived_at.is_none() && w.tenant_id == tenant.tenant_id)
        .collect();

    let org_role = strongest_role(effective, tenant.tenant_id, None);
    // A grant stored *on this workspace* (workspace_id = X). Distinct from `strongest_role`,
    // which also matches the tenant-wide row: that one is the inherited case.
    let workspace_row = |ws: Uuid| -> Option<MembershipRole> {
        effective
            .iter()
            .filter(|m| m.tenant_id == tenant.tenant_id && m.workspace_id == Some(ws))
            .map(|m| m.role)
            .max_by_key(MembershipRole::level)
    };

    let organization = {
        let caps: Vec<String> = if platform_admin {
            // Support access: a platform admin holds every organization capability.
            crate::services::capabilities::ALL_CAPABILITIES
                .into_iter()
                .filter(|c| c.axis() == CapabilityAxis::Organization)
                .map(|c| c.id().to_string())
                .collect()
        } else if tenant_active {
            org_role
                .map(|r| {
                    capabilities_for_role(CapabilityAxis::Organization, r)
                        .into_iter()
                        .map(|c| c.id().to_string())
                        .collect()
                })
                .unwrap_or_default()
        } else {
            // Fail closed: an archived organization grants a member nothing, but it is still
            // reported so the UI can say "restore it first" instead of "you have no access".
            Vec::new()
        };
        Some(OrganizationAccess {
            id: tenant.tenant_id,
            name: tenant.name.clone(),
            slug: tenant.slug.clone(),
            role: org_role.map(|r| r.to_string()),
            level: org_role.map_or(0, |r| r.level()),
            is_active: tenant_active,
            is_archived: tenant.archived_at.is_some(),
            capabilities: caps,
        })
    };

    let admin_ids = crate::handlers::me_settings_access::administered_workspace_ids(
        platform_admin,
        tenant.tenant_id,
        effective,
        input.workspaces,
    );

    let mut workspaces: Vec<WorkspaceAccess> = active_workspaces
        .iter()
        .filter_map(|w| {
            let own = workspace_row(w.workspace_id);
            // A tenant-wide grant reaches every workspace; a workspace grant only its own.
            let effective_role = match (org_role, own) {
                (Some(o), Some(wr)) => Some(if o.level() >= wr.level() { o } else { wr }),
                (Some(o), None) => Some(o),
                (None, Some(wr)) => Some(wr),
                (None, None) => None,
            };
            // Platform admins read everything, as support.
            let role = if platform_admin {
                Some(MembershipRole::Owner)
            } else {
                effective_role
            }?;
            Some(WorkspaceAccess {
                id: w.workspace_id,
                name: w.name.clone(),
                slug: w.slug.clone(),
                role: role.to_string(),
                level: role.level(),
                // Inherited means "no row on this workspace"; the access came from the
                // organization grant, so the member editor must not offer to edit it here.
                inherited: own.is_none(),
                capabilities: capabilities_for_role(CapabilityAxis::Workspace, role)
                    .into_iter()
                    .map(|c| c.id().to_string())
                    .collect(),
            })
        })
        .collect();
    workspaces.sort_by_key(|w| w.name.to_lowercase());

    let workspaces_used = input
        .workspaces
        .iter()
        .filter(|w| w.tenant_id == tenant.tenant_id && w.archived_at.is_none())
        .count();
    let quota = QuotaAccess {
        workspaces: Budget::new(workspaces_used, tenant.max_workspaces),
        seats: Budget::new(input.seats_used, tenant.max_users),
    };

    let org_caps: Vec<String> = organization
        .as_ref()
        .map(|o| o.capabilities.clone())
        .unwrap_or_default();
    let has_org_cap = |c: Capability| org_caps.iter().any(|id| id == c.id());

    let can_create_org = platform
        .capabilities
        .iter()
        .any(|c| c == Capability::PlatformOrganizationsCreate.id());
    let role_allows_ws = has_org_cap(Capability::OrganizationWorkspacesCreate);
    let can_create_ws = role_allows_ws && tenant_active && quota.workspaces.has_room();

    // Distinguish "you may not" (absent) from "you may, but full" (disabled + fix).
    let ws_reason = if can_create_ws {
        None
    } else if !tenant_active {
        Some("scope_inactive".to_string())
    } else if role_allows_ws {
        Some("workspace_quota_reached".to_string())
    } else {
        Some(crate::services::capabilities::required_reason(
            Capability::OrganizationWorkspacesCreate,
        ))
    };

    let _ = admin_ids;

    MeAccessResponse {
        user_id: input.user_id.to_string(),
        platform,
        organization,
        workspaces,
        quota,
        can_create: CreatePermissions {
            organization: can_create_org,
            workspace: can_create_ws,
        },
        can_delete: DeletePermissions {
            organization: has_org_cap(Capability::OrganizationDelete),
        },
        reason: CreateReason {
            organization: if can_create_org {
                None
            } else {
                Some(crate::services::capabilities::required_reason(
                    Capability::PlatformOrganizationsCreate,
                ))
            },
            workspace: ws_reason,
        },
    }
}

/// `GET /api/v1/me/access?tenant_id=&workspace_id=`
#[utoipa::path(
    get,
    path = "/api/v1/me/access",
    tag = "Access",
    params(
        ("tenant_id" = Option<Uuid>, Query, description = "Organization to inspect; defaults to the caller's default"),
        ("workspace_id" = Option<Uuid>, Query, description = "Workspace to include when it is not listed"),
    ),
    responses(
        (status = 200, description = "The caller's grants, capabilities and quota", body = MeAccessResponse),
    )
)]
pub async fn get_me_access(
    State(state): State<AppState>,
    ApiAuthenticated(auth): ApiAuthenticated,
    Query(query): Query<MeAccessQuery>,
) -> Result<Json<MeAccessResponse>, ApiError> {
    let tenant_id = query
        .tenant_id
        .unwrap_or_else(crate::middleware::default_tenant_uuid);

    let memberships = load_memberships(&state, &auth).await?;
    let tenant = state
        .workspace_service
        .get_tenant(tenant_id)
        .await
        .map_err(internal)?;
    let workspaces = match &tenant {
        Some(t) if t.is_active && t.archived_at.is_none() => state
            .workspace_service
            .list_workspaces(t.tenant_id)
            .await
            .map_err(internal)?,
        // An archived organization exposes no workspace list.
        _ => Vec::new(),
    };
    let seats_used = distinct_members(&memberships, tenant_id);

    Ok(Json(build_me_access(AccessInputs {
        user_id: &auth.user_id,
        platform_role: auth.role,
        tenant: tenant.as_ref(),
        memberships: &memberships,
        workspaces: &workspaces,
        seats_used,
    })))
}

async fn load_memberships(
    state: &AppState,
    auth: &RequestAuthContext,
) -> Result<Vec<Membership>, ApiError> {
    match Uuid::parse_str(&auth.user_id) {
        Ok(user) => state
            .workspace_service
            .get_user_memberships(user)
            .await
            .map_err(internal),
        // A master API key or an anonymous dev principal has no memberships; fail closed.
        Err(_) => Ok(Vec::new()),
    }
}

/// Distinct active members of the organization, for the seat budget.
fn distinct_members(memberships: &[Membership], tenant: Uuid) -> usize {
    let mut ids: Vec<Uuid> = memberships
        .iter()
        .filter(|m| m.is_active && m.tenant_id == tenant)
        .map(|m| m.user_id)
        .collect();
    ids.sort_unstable();
    ids.dedup();
    ids.len()
}

fn internal(e: edgequake_core::Error) -> ApiError {
    ApiError::Internal(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::collections::HashMap;

    fn tenant(plan_workspaces: usize, plan_users: usize) -> edgequake_core::Tenant {
        let mut t = edgequake_core::Tenant::new("Acme", "acme");
        t.max_workspaces = plan_workspaces;
        t.max_users = plan_users;
        t
    }

    fn ws(id: Uuid, tenant: Uuid, name: &str) -> Workspace {
        let mut w = Workspace::new(tenant, name, name.to_lowercase());
        w.workspace_id = id;
        w
    }

    fn grant(tenant: Uuid, workspace: Option<Uuid>, role: MembershipRole) -> Membership {
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

    fn build(
        role: edgequake_auth::Role,
        t: Option<&edgequake_core::Tenant>,
        memberships: &[Membership],
        workspaces: &[Workspace],
        seats: usize,
    ) -> MeAccessResponse {
        build_me_access(AccessInputs {
            user_id: "u",
            platform_role: role,
            tenant: t,
            memberships,
            workspaces,
            seats_used: seats,
        })
    }

    #[test]
    fn platform_admin_can_create_an_organization_and_everything_else() {
        let r = build(edgequake_auth::Role::Admin, None, &[], &[], 0);
        assert!(r.can_create.organization);
        assert!(r.reason.organization.is_none());
        assert!(r.platform.admin);
    }

    #[test]
    fn a_member_without_an_org_grant_gets_no_create_and_a_reason() {
        let t = tenant(10, 3);
        let r = build(edgequake_auth::Role::User, Some(&t), &[], &[], 0);
        assert!(!r.can_create.organization);
        assert!(!r.can_create.workspace);
        assert_eq!(
            r.reason.workspace.as_deref(),
            Some("capability_required:organization.workspaces.create")
        );
    }

    #[test]
    fn organization_admin_creates_workspaces_but_not_organizations() {
        let t = tenant(10, 3);
        let m = [grant(t.tenant_id, None, MembershipRole::Admin)];
        let r = build(edgequake_auth::Role::User, Some(&t), &m, &[], 1);
        assert!(!r.can_create.organization);
        assert!(r.can_create.workspace);
        assert!(!r.can_delete.organization);
    }

    #[test]
    fn organization_owner_can_delete_the_organization() {
        let t = tenant(10, 3);
        let m = [grant(t.tenant_id, None, MembershipRole::Owner)];
        let r = build(edgequake_auth::Role::User, Some(&t), &m, &[], 1);
        assert!(r.can_delete.organization);
    }

    #[test]
    fn quota_full_disables_the_action_with_a_quota_reason_not_a_role_reason() {
        let t = tenant(1, 3);
        let m = [grant(t.tenant_id, None, MembershipRole::Owner)];
        let w = [ws(Uuid::new_v4(), t.tenant_id, "Legal")];
        let r = build(edgequake_auth::Role::User, Some(&t), &m, &w, 1);
        assert!(!r.can_create.workspace, "quota is full");
        assert_eq!(
            r.reason.workspace.as_deref(),
            Some("workspace_quota_reached"),
            "the caller can fix quota, so the reason must not claim a missing role"
        );
    }

    #[test]
    fn unlimited_seats_never_block() {
        let t = tenant(10, 0);
        let m = [grant(t.tenant_id, None, MembershipRole::Admin)];
        let r = build(edgequake_auth::Role::User, Some(&t), &m, &[], 99);
        assert!(r.quota.seats.has_room(), "limit 0 means unlimited");
        assert!(r.can_create.workspace);
    }

    #[test]
    fn workspace_admin_sees_only_their_workspace_and_cannot_create_one() {
        let t = tenant(10, 3);
        let legal = Uuid::new_v4();
        let finance = Uuid::new_v4();
        let m = [grant(t.tenant_id, Some(legal), MembershipRole::Admin)];
        let w = [
            ws(legal, t.tenant_id, "Legal"),
            ws(finance, t.tenant_id, "Finance"),
        ];
        let r = build(edgequake_auth::Role::User, Some(&t), &m, &w, 1);
        assert_eq!(r.workspaces.len(), 1, "Finance is not granted");
        assert_eq!(r.workspaces[0].name, "Legal");
        assert!(!r.workspaces[0].inherited);
        assert!(
            !r.can_create.workspace,
            "a workspace admin adds no workspace"
        );
        assert_eq!(
            r.organization.as_ref().and_then(|o| o.role.as_deref()),
            None
        );
    }

    #[test]
    fn two_grants_merge_into_the_highest_effective_level() {
        let t = tenant(10, 3);
        let legal = Uuid::new_v4();
        let m = [
            grant(t.tenant_id, None, MembershipRole::Member),
            grant(t.tenant_id, Some(legal), MembershipRole::Admin),
        ];
        let w = [ws(legal, t.tenant_id, "Legal")];
        let r = build(edgequake_auth::Role::User, Some(&t), &m, &w, 1);
        let legal_access = r.workspaces.iter().find(|x| x.id == legal).unwrap();
        assert_eq!(legal_access.role, "admin");
        assert_eq!(legal_access.level, 3);
    }

    #[test]
    fn an_organization_admin_inherits_every_workspace() {
        let t = tenant(10, 3);
        let legal = Uuid::new_v4();
        let finance = Uuid::new_v4();
        let m = [grant(t.tenant_id, None, MembershipRole::Admin)];
        let w = [
            ws(legal, t.tenant_id, "Legal"),
            ws(finance, t.tenant_id, "Finance"),
        ];
        let r = build(edgequake_auth::Role::User, Some(&t), &m, &w, 1);
        assert_eq!(r.workspaces.len(), 2);
        assert!(r.workspaces.iter().all(|x| x.inherited));
        assert!(r.workspaces.iter().all(|x| x.role == "admin"));
    }

    #[test]
    fn an_archived_organization_grants_nothing_to_a_member() {
        let mut t = tenant(10, 3);
        t.archived_at = Some(Utc::now());
        let m = [grant(t.tenant_id, None, MembershipRole::Owner)];
        let w = [ws(Uuid::new_v4(), t.tenant_id, "Legal")];
        let r = build(edgequake_auth::Role::User, Some(&t), &m, &w, 1);
        assert!(r.workspaces.is_empty());
        assert!(!r.can_create.workspace);
        assert_eq!(r.reason.workspace.as_deref(), Some("scope_inactive"));
        assert!(r.organization.as_ref().unwrap().is_archived);
    }

    #[test]
    fn platform_admin_reaches_every_workspace_as_support() {
        let t = tenant(10, 3);
        let w = [ws(Uuid::new_v4(), t.tenant_id, "Legal")];
        let r = build(edgequake_auth::Role::Admin, Some(&t), &[], &w, 0);
        assert_eq!(r.workspaces.len(), 1);
        assert_eq!(r.workspaces[0].role, "owner");
        assert!(r.can_create.workspace);
    }

    #[test]
    fn capability_lists_are_axis_pure() {
        let t = tenant(10, 3);
        let m = [grant(t.tenant_id, None, MembershipRole::Owner)];
        let w = [ws(Uuid::new_v4(), t.tenant_id, "Legal")];
        let r = build(edgequake_auth::Role::User, Some(&t), &m, &w, 1);
        for cap in &r.organization.as_ref().unwrap().capabilities {
            assert!(cap.starts_with("organization."), "{cap}");
        }
        for cap in &r.workspaces[0].capabilities {
            assert!(cap.starts_with("workspace."), "{cap}");
        }
    }
}
