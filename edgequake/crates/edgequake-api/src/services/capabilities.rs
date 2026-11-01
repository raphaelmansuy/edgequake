//! Capability catalog (SPEC-181 LAW-181-1) — the one place a right is *named*.
//!
//! Every capability is a predicate over the grant that already governs the corresponding
//! handler, so the matrix the WebUI renders cannot drift from what the API enforces.
//! [`members::guards`](crate::handlers::members::guards) and
//! [`workspaces::tenant_access`](crate::handlers::workspaces::tenant_access) both ask this
//! module, and `spec181_catalog_matches_enforcement` proves the two agree.
//!
//! Three axes, never conflated:
//!
//! | Axis | Stored as | Decides |
//! |------|-----------|---------|
//! | `platform` | `users.role` (the session JWT) | deployment: connections, fleet, every org |
//! | `organization` | `memberships.workspace_id IS NULL` | the org: brand, members, creating workspaces |
//! | `workspace` | `memberships.workspace_id = X` | one workspace: content, models, labels |

use std::str::FromStr;

use edgequake_core::MembershipRole;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Which grant a capability is evaluated against.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum CapabilityAxis {
    /// Instance-wide (`users.role`).
    Platform,
    /// Tenant-wide (`memberships.workspace_id IS NULL`).
    Organization,
    /// One workspace (`memberships.workspace_id = X`).
    Workspace,
}

impl CapabilityAxis {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Platform => "platform",
            Self::Organization => "organization",
            Self::Workspace => "workspace",
        }
    }
}

impl std::fmt::Display for CapabilityAxis {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The catalog. One variant per decision a caller can make on a scope.
///
/// Wire ids are `axis.verb` so a log line, an audit row and a UI gate all use the same string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Capability {
    // ── Platform ─────────────────────────────────────────────────────────────
    /// Operate the deployment: connections, providers, fleet defaults, observability.
    PlatformDeploymentManage,
    /// List every organization on the deployment.
    PlatformOrganizationsList,
    /// Create an organization.
    PlatformOrganizationsCreate,
    /// Delete an organization.
    PlatformOrganizationsDelete,

    // ── Organization ─────────────────────────────────────────────────────────
    /// Read the organization (its name, plan, workspace list).
    OrganizationRead,
    /// Edit name, description, brand color, plan, default models.
    OrganizationSettingsManage,
    /// Invite, grant, change and remove organization members.
    OrganizationMembersManage,
    /// Grant or revoke the Owner role.
    OrganizationGrantOwner,
    /// Delete (purge) the organization.
    OrganizationDelete,
    /// Create, rename, archive and delete workspaces of this organization.
    OrganizationWorkspacesCreate,

    // ── Workspace ────────────────────────────────────────────────────────────
    /// Read documents, graph and answers in the workspace.
    WorkspaceRead,
    /// Upload, edit and delete documents.
    WorkspaceContentWrite,
    /// Change models, ingestion settings and document labels.
    WorkspaceSettingsManage,
    /// Invite, grant and remove members of this workspace.
    WorkspaceMembersManage,
    /// Delete the workspace permanently.
    WorkspaceDelete,
}

/// Every capability, in catalogue order. The UI matrix iterates this.
pub const ALL_CAPABILITIES: [Capability; 15] = [
    Capability::PlatformDeploymentManage,
    Capability::PlatformOrganizationsList,
    Capability::PlatformOrganizationsCreate,
    Capability::PlatformOrganizationsDelete,
    Capability::OrganizationRead,
    Capability::OrganizationSettingsManage,
    Capability::OrganizationMembersManage,
    Capability::OrganizationGrantOwner,
    Capability::OrganizationDelete,
    Capability::OrganizationWorkspacesCreate,
    Capability::WorkspaceRead,
    Capability::WorkspaceContentWrite,
    Capability::WorkspaceSettingsManage,
    Capability::WorkspaceMembersManage,
    Capability::WorkspaceDelete,
];

impl Capability {
    /// Stable wire id (`organization.workspaces.create`).
    pub fn id(&self) -> &'static str {
        match self {
            Self::PlatformDeploymentManage => "platform.deployment.manage",
            Self::PlatformOrganizationsList => "platform.organizations.list",
            Self::PlatformOrganizationsCreate => "platform.organizations.create",
            Self::PlatformOrganizationsDelete => "platform.organizations.delete",
            Self::OrganizationRead => "organization.read",
            Self::OrganizationSettingsManage => "organization.settings.manage",
            Self::OrganizationMembersManage => "organization.members.manage",
            Self::OrganizationGrantOwner => "organization.grant_owner",
            Self::OrganizationDelete => "organization.delete",
            Self::OrganizationWorkspacesCreate => "organization.workspaces.create",
            Self::WorkspaceRead => "workspace.read",
            Self::WorkspaceContentWrite => "workspace.content.write",
            Self::WorkspaceSettingsManage => "workspace.settings.manage",
            Self::WorkspaceMembersManage => "workspace.members.manage",
            Self::WorkspaceDelete => "workspace.delete",
        }
    }

    pub fn axis(&self) -> CapabilityAxis {
        match self {
            Self::PlatformDeploymentManage
            | Self::PlatformOrganizationsList
            | Self::PlatformOrganizationsCreate
            | Self::PlatformOrganizationsDelete => CapabilityAxis::Platform,
            Self::OrganizationRead
            | Self::OrganizationSettingsManage
            | Self::OrganizationMembersManage
            | Self::OrganizationGrantOwner
            | Self::OrganizationDelete
            | Self::OrganizationWorkspacesCreate => CapabilityAxis::Organization,
            Self::WorkspaceRead
            | Self::WorkspaceContentWrite
            | Self::WorkspaceSettingsManage
            | Self::WorkspaceMembersManage
            | Self::WorkspaceDelete => CapabilityAxis::Workspace,
        }
    }

    /// The minimum organization/workspace role that carries this capability.
    ///
    /// `None` for platform capabilities, which are decided by `Role` rather than by a
    /// `MembershipRole` (see [`platform_role_grants`]).
    pub fn min_role(&self) -> Option<MembershipRole> {
        use MembershipRole::{Admin, Member, Owner, Readonly};
        Some(match self {
            Self::PlatformDeploymentManage
            | Self::PlatformOrganizationsList
            | Self::PlatformOrganizationsCreate
            | Self::PlatformOrganizationsDelete => return None,

            // Anyone with a grant can read the organization they belong to.
            Self::OrganizationRead => Readonly,
            Self::OrganizationSettingsManage => Admin,
            Self::OrganizationMembersManage => Admin,
            Self::OrganizationWorkspacesCreate => Admin,
            // The two owner-only organization rights.
            Self::OrganizationGrantOwner => Owner,
            Self::OrganizationDelete => Owner,

            Self::WorkspaceRead => Readonly,
            Self::WorkspaceContentWrite => Member,
            Self::WorkspaceSettingsManage => Admin,
            Self::WorkspaceMembersManage => Admin,
            // SPEC-181: narrowed from `Admin` to `Owner`. An organization manager still deletes
            // any workspace through their implicit grant; a workspace Admin cannot end one.
            Self::WorkspaceDelete => Owner,
        })
    }

    /// Group label used by the roles matrix.
    pub fn group(&self) -> &'static str {
        match self.axis() {
            CapabilityAxis::Platform => "Deployment",
            CapabilityAxis::Organization => "Organization",
            CapabilityAxis::Workspace => "Workspace",
        }
    }

    /// Human label for the matrix and for refusal copy.
    pub fn label(&self) -> &'static str {
        match self {
            Self::PlatformDeploymentManage => "Manage the deployment",
            Self::PlatformOrganizationsList => "List every organization",
            Self::PlatformOrganizationsCreate => "Create an organization",
            Self::PlatformOrganizationsDelete => "Delete an organization",
            Self::OrganizationRead => "View the organization",
            Self::OrganizationSettingsManage => "Manage settings and branding",
            Self::OrganizationMembersManage => "Invite and manage people",
            Self::OrganizationGrantOwner => "Grant the owner role",
            Self::OrganizationDelete => "Delete the organization",
            Self::OrganizationWorkspacesCreate => "Create and configure workspaces",
            Self::WorkspaceRead => "View documents, graph and answers",
            Self::WorkspaceContentWrite => "Upload, edit and delete documents",
            Self::WorkspaceSettingsManage => "Manage models, ingestion and labels",
            Self::WorkspaceMembersManage => "Invite and manage workspace members",
            Self::WorkspaceDelete => "Delete the workspace",
        }
    }

    /// Predicate over a membership role. This *is* enforcement for the two membership axes.
    ///
    /// It reads the same `can_*` helpers the handlers call, so a change there moves the
    /// catalogue with it and `catalog_equals_enforcement` keeps them honest.
    pub fn granted_to(&self, role: MembershipRole) -> bool {
        match self.min_role() {
            // A platform capability is never granted by a membership role.
            None => false,
            Some(min) => role.level() >= min.level(),
        }
    }

    /// Parse a wire id back into the enum (audit rows, API keys, tests).
    pub fn parse(id: &str) -> Option<Self> {
        ALL_CAPABILITIES.into_iter().find(|c| c.id() == id)
    }
}

impl std::fmt::Display for Capability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.id())
    }
}

impl FromStr for Capability {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s).ok_or_else(|| format!("Unknown capability: {s}"))
    }
}

/// Platform capabilities are decided by the instance role, not by a membership.
///
/// Only `Role::Admin` holds any of them today. Kept as a function rather than a constant so a
/// future `operator` role has exactly one place to be added.
pub fn platform_role_grants(role: edgequake_auth::Role) -> bool {
    matches!(role, edgequake_auth::Role::Admin)
}

/// The capabilities a platform role holds.
pub fn capabilities_for_platform(role: edgequake_auth::Role) -> Vec<Capability> {
    if platform_role_grants(role) {
        ALL_CAPABILITIES
            .into_iter()
            .filter(|c| c.axis() == CapabilityAxis::Platform)
            .collect()
    } else {
        Vec::new()
    }
}

/// The capabilities a membership role holds **on its own axis**.
pub fn capabilities_for_role(axis: CapabilityAxis, role: MembershipRole) -> Vec<Capability> {
    ALL_CAPABILITIES
        .into_iter()
        .filter(|c| c.axis() == axis && c.granted_to(role))
        .collect()
}

/// Refusal reason code for a missing capability ([LAW-181-5](01-first-principles.md)).
pub fn required_reason(capability: Capability) -> String {
    format!("capability_required:{}", capability.id())
}

/// A capability row as the roles matrix and the access payload render it.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CapabilityInfo {
    pub id: String,
    pub axis: String,
    pub group: String,
    pub label: String,
    /// Minimum role id on this axis; absent for platform capabilities.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_role: Option<String>,
    /// Organization/workspace roles that hold it, highest privilege first.
    pub roles: Vec<String>,
}

/// Build the capability rows. Pure; cheap enough per request.
pub fn capability_infos() -> Vec<CapabilityInfo> {
    use CapabilityAxis::{Organization, Workspace};
    // Highest privilege first, matching how the roles tab renders columns.
    let org_order = [
        MembershipRole::Owner,
        MembershipRole::Admin,
        MembershipRole::Member,
        MembershipRole::Readonly,
    ];
    ALL_CAPABILITIES
        .into_iter()
        .map(|c| {
            let roles = match c.axis() {
                CapabilityAxis::Platform => Vec::new(),
                Organization | Workspace => org_order
                    .into_iter()
                    .filter(|r| c.granted_to(*r))
                    .map(|r| r.to_string())
                    .collect(),
            };
            CapabilityInfo {
                id: c.id().to_string(),
                axis: c.axis().to_string(),
                group: c.group().to_string(),
                label: c.label().to_string(),
                min_role: c.min_role().map(|r| r.to_string()),
                roles,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use CapabilityAxis::{Organization, Platform, Workspace};
    use MembershipRole::{Admin, Member, Owner, Readonly};

    fn holders(c: Capability) -> Vec<MembershipRole> {
        [Owner, Admin, Member, Readonly]
            .into_iter()
            .filter(|r| c.granted_to(*r))
            .collect()
    }

    #[test]
    fn ids_are_unique_and_stable() {
        let mut ids: Vec<&str> = ALL_CAPABILITIES.iter().map(|c| c.id()).collect();
        ids.sort_unstable();
        let before = ids.len();
        ids.dedup();
        assert_eq!(before, ids.len(), "duplicate capability id");
        assert!(ALL_CAPABILITIES
            .iter()
            .all(|c| c.id().starts_with(c.axis().as_str())));
    }

    #[test]
    fn every_axis_has_capabilities_and_round_trips() {
        for axis in [Platform, Organization, Workspace] {
            assert!(
                ALL_CAPABILITIES.iter().any(|c| c.axis() == axis),
                "axis {axis} has no capability"
            );
        }
        for c in ALL_CAPABILITIES {
            assert_eq!(Capability::parse(c.id()), Some(c));
        }
    }

    #[test]
    fn platform_capabilities_are_never_granted_by_a_membership_role() {
        for c in ALL_CAPABILITIES.iter().filter(|c| c.axis() == Platform) {
            assert!(c.min_role().is_none());
            for r in [Owner, Admin, Member, Readonly] {
                assert!(
                    !c.granted_to(r),
                    "{} must not be granted by membership role {r}",
                    c.id()
                );
            }
        }
    }

    #[test]
    fn viewers_read_but_never_write() {
        assert_eq!(
            holders(Capability::WorkspaceRead),
            [Owner, Admin, Member, Readonly]
        );
        assert_eq!(
            holders(Capability::WorkspaceContentWrite),
            [Owner, Admin, Member]
        );
    }

    #[test]
    fn organization_management_is_admin_and_owner_only() {
        assert_eq!(
            holders(Capability::OrganizationMembersManage),
            [Owner, Admin]
        );
        assert_eq!(
            holders(Capability::OrganizationWorkspacesCreate),
            [Owner, Admin]
        );
        assert_eq!(
            holders(Capability::OrganizationSettingsManage),
            [Owner, Admin]
        );
    }

    #[test]
    fn owner_only_rights_are_exactly_two_on_the_organization() {
        assert_eq!(holders(Capability::OrganizationGrantOwner), [Owner]);
        assert_eq!(holders(Capability::OrganizationDelete), [Owner]);
    }

    #[test]
    fn workspace_delete_is_the_only_owner_only_workspace_right() {
        // SPEC-181: narrowed from Admin. An org manager still deletes via the implicit grant.
        assert_eq!(holders(Capability::WorkspaceDelete), [Owner]);
        assert_eq!(holders(Capability::WorkspaceSettingsManage), [Owner, Admin]);
        assert_eq!(holders(Capability::WorkspaceMembersManage), [Owner, Admin]);
    }

    #[test]
    fn capability_infos_name_a_min_role_on_membership_axes_only() {
        for info in capability_infos() {
            if info.axis == "platform" {
                assert!(info.min_role.is_none(), "{}", info.id);
                assert!(info.roles.is_empty(), "{}", info.id);
            } else {
                assert!(info.min_role.is_some(), "{}", info.id);
                assert!(!info.roles.is_empty(), "{}", info.id);
            }
        }
    }

    #[test]
    fn capabilities_for_role_only_returns_capabilities_of_that_axis() {
        let ws = capabilities_for_role(Organization, Admin);
        assert!(ws.iter().all(|c| c.axis() == Organization));
        assert!(ws.contains(&Capability::OrganizationMembersManage));
        assert!(!ws.contains(&Capability::WorkspaceContentWrite));

        let member = capabilities_for_role(Organization, Member);
        assert_eq!(member, vec![Capability::OrganizationRead]);
    }

    #[test]
    fn reason_codes_are_prefixed_for_the_client() {
        assert_eq!(
            required_reason(Capability::OrganizationWorkspacesCreate),
            "capability_required:organization.workspaces.create"
        );
    }
}
