//! Role & permission catalog (SPEC-169 / SPEC-181) — the single source of truth the WebUI
//! renders.
//!
//! SPEC-181 moved the capability table into [`crate::services::capabilities`], which owns the
//! predicates the handlers call. This module keeps the *role* metadata (labels, levels,
//! descriptions) and the shape the members screen consumes, so the wire contract of
//! `GET /roles` is unchanged apart from two additive fields (`axis`, `min_role`).

use edgequake_auth::{RbacService, Role};
use edgequake_core::MembershipRole;
use serde::Serialize;
use utoipa::ToSchema;

use crate::services::capabilities::capability_infos;

/// Re-exported so `openapi.rs` can name the schema in the same module it always did.
pub use crate::services::capabilities::CapabilityInfo;

/// Organization roles, lowest privilege first (matrix column order).
pub const ORGANIZATION_ROLES: [MembershipRole; 4] = [
    MembershipRole::Readonly,
    MembershipRole::Member,
    MembershipRole::Admin,
    MembershipRole::Owner,
];

/// Instance (platform-wide) roles carried in the session JWT.
const INSTANCE_ROLES: [Role; 3] = [Role::Readonly, Role::User, Role::Admin];

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct OrganizationRoleInfo {
    /// Wire id (`owner`, `admin`, `member`, `readonly`).
    pub id: String,
    /// Product label (`Viewer` for `readonly`).
    pub label: String,
    pub description: String,
    /// Privilege rank (1 = lowest); a manager can only assign roles at or below their own.
    pub level: u8,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct InstanceRoleInfo {
    pub id: String,
    pub label: String,
    pub description: String,
    /// Number of fine-grained RBAC permissions the role carries.
    pub permission_count: usize,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RoleCatalog {
    pub organization_roles: Vec<OrganizationRoleInfo>,
    pub instance_roles: Vec<InstanceRoleInfo>,
    /// One row per capability, carrying `axis` so a workspace capability is never drawn under
    /// the organization columns.
    pub capabilities: Vec<CapabilityInfo>,
}

/// Product label for an organization role id.
pub fn organization_role_label(role: MembershipRole) -> &'static str {
    match role {
        MembershipRole::Owner => "Owner",
        MembershipRole::Admin => "Admin",
        MembershipRole::Member => "Member",
        MembershipRole::Readonly => "Viewer",
    }
}

fn organization_role_description(role: MembershipRole) -> &'static str {
    match role {
        MembershipRole::Owner => {
            "Full control of the organization, including ownership transfer and deletion."
        }
        MembershipRole::Admin => "Manages members, roles and workspaces. Cannot grant Owner.",
        MembershipRole::Member => "Works with knowledge: uploads, edits, queries and chat.",
        MembershipRole::Readonly => "Read-only access: browse documents, graph and run queries.",
    }
}

fn instance_role_label(role: &Role) -> &'static str {
    match role {
        Role::Admin => "Instance admin",
        Role::User => "Standard",
        Role::Readonly => "Read-only",
    }
}

fn instance_role_description(role: &Role) -> &'static str {
    match role {
        Role::Admin => {
            "Platform administrator: SSO, models, quotas and every organization. Use sparingly."
        }
        Role::User => "Regular account. Access is decided by organization roles.",
        Role::Readonly => "Account-wide read-only guard, regardless of organization role.",
    }
}

/// Build the catalog (pure; cheap enough to compute per request).
pub fn build_role_catalog(rbac: &RbacService) -> RoleCatalog {
    RoleCatalog {
        organization_roles: ORGANIZATION_ROLES
            .iter()
            .rev()
            .map(|r| OrganizationRoleInfo {
                id: r.to_string(),
                label: organization_role_label(*r).to_string(),
                description: organization_role_description(*r).to_string(),
                level: r.level(),
            })
            .collect(),
        instance_roles: INSTANCE_ROLES
            .iter()
            .rev()
            .map(|r| InstanceRoleInfo {
                id: r.as_str().to_string(),
                label: instance_role_label(r).to_string(),
                description: instance_role_description(r).to_string(),
                permission_count: rbac.role_permissions(r).len(),
            })
            .collect(),
        capabilities: capability_infos(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::capabilities::Capability;

    fn catalog() -> RoleCatalog {
        build_role_catalog(&RbacService::new())
    }

    fn holders(c: &RoleCatalog, id: &str) -> Vec<String> {
        c.capabilities
            .iter()
            .find(|x| x.id == id)
            .unwrap_or_else(|| panic!("capability {id}"))
            .roles
            .clone()
    }

    #[test]
    fn roles_are_listed_highest_privilege_first_with_viewer_label() {
        let c = catalog();
        let ids: Vec<_> = c.organization_roles.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(ids, ["owner", "admin", "member", "readonly"]);
        assert_eq!(c.organization_roles[3].label, "Viewer");
        let levels: Vec<_> = c.organization_roles.iter().map(|r| r.level).collect();
        assert!(levels.windows(2).all(|w| w[0] > w[1]));
    }

    #[test]
    fn capability_matrix_mirrors_enforcement_predicates() {
        let c = catalog();
        assert_eq!(
            holders(&c, "workspace.content.write"),
            ["owner", "admin", "member"],
            "viewers never write"
        );
        assert_eq!(
            holders(&c, "organization.members.manage"),
            ["owner", "admin"]
        );
        assert_eq!(
            holders(&c, "organization.workspaces.create"),
            ["owner", "admin"]
        );
        assert_eq!(holders(&c, "organization.grant_owner"), ["owner"]);
        assert_eq!(holders(&c, "organization.delete"), ["owner"]);
        assert_eq!(holders(&c, "workspace.read").len(), 4);
    }

    #[test]
    fn every_role_in_the_matrix_exists_in_the_role_list() {
        let c = catalog();
        let known: Vec<_> = c.organization_roles.iter().map(|r| r.id.clone()).collect();
        for cap in &c.capabilities {
            assert!(cap.roles.iter().all(|r| known.contains(r)), "{}", cap.id);
        }
    }

    #[test]
    fn instance_admin_carries_the_most_permissions() {
        let c = catalog();
        assert_eq!(c.instance_roles[0].id, "admin");
        let counts: Vec<_> = c
            .instance_roles
            .iter()
            .map(|r| r.permission_count)
            .collect();
        assert!(counts[0] > counts[1] && counts[1] >= counts[2]);
    }

    #[test]
    fn every_catalog_capability_is_reachable_from_the_enum() {
        // Guards against a hand-written row that names a capability the handlers cannot decide.
        let c = catalog();
        for cap in &c.capabilities {
            assert!(
                Capability::parse(&cap.id).is_some(),
                "{} is not a known capability",
                cap.id
            );
        }
        assert_eq!(
            c.capabilities.len(),
            crate::services::capabilities::ALL_CAPABILITIES.len()
        );
    }
}
