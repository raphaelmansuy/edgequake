//! Membership and role types for tenant/workspace access control.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// A user's membership in a tenant/workspace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Membership {
    /// Unique membership identifier.
    pub membership_id: Uuid,
    /// User ID (from auth system).
    pub user_id: Uuid,
    /// Tenant ID.
    pub tenant_id: Uuid,
    /// Optional workspace ID (None = all workspaces in tenant).
    pub workspace_id: Option<Uuid>,
    /// Role within the tenant/workspace.
    pub role: MembershipRole,
    /// Whether the membership is active.
    pub is_active: bool,
    /// When the user joined.
    pub joined_at: chrono::DateTime<chrono::Utc>,
    /// Custom metadata.
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Membership {
    /// Create a new membership.
    pub fn new(user_id: Uuid, tenant_id: Uuid, role: MembershipRole) -> Self {
        Self {
            membership_id: Uuid::new_v4(),
            user_id,
            tenant_id,
            workspace_id: None,
            role,
            is_active: true,
            joined_at: chrono::Utc::now(),
            metadata: HashMap::new(),
        }
    }

    /// Scope membership to a specific workspace.
    pub fn for_workspace(mut self, workspace_id: Uuid) -> Self {
        self.workspace_id = Some(workspace_id);
        self
    }

    /// Check if user has at least the given role.
    pub fn has_role(&self, required: MembershipRole) -> bool {
        self.role.level() >= required.level()
    }

    /// Check if user can access a specific workspace.
    pub fn can_access_workspace(&self, workspace_id: &Uuid) -> bool {
        if !self.is_active {
            return false;
        }
        // None = access to all workspaces
        self.workspace_id.is_none() || self.workspace_id.as_ref() == Some(workspace_id)
    }
}

/// Roles for tenant/workspace membership.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum MembershipRole {
    /// Read-only access.
    Readonly,
    /// Standard member (read/write).
    #[default]
    Member,
    /// Administrator (can manage users).
    Admin,
    /// Owner (full control, can delete tenant).
    Owner,
}

impl MembershipRole {
    /// Get the permission level (higher = more permissions).
    pub fn level(&self) -> u8 {
        match self {
            MembershipRole::Readonly => 1,
            MembershipRole::Member => 2,
            MembershipRole::Admin => 3,
            MembershipRole::Owner => 4,
        }
    }

    /// Check if this role can write data.
    pub fn can_write(&self) -> bool {
        matches!(
            self,
            MembershipRole::Member | MembershipRole::Admin | MembershipRole::Owner
        )
    }

    /// Check if this role can manage users.
    pub fn can_manage_users(&self) -> bool {
        matches!(self, MembershipRole::Admin | MembershipRole::Owner)
    }

    /// Check if this role can delete the tenant.
    pub fn can_delete_tenant(&self) -> bool {
        matches!(self, MembershipRole::Owner)
    }
}

impl std::fmt::Display for MembershipRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MembershipRole::Readonly => write!(f, "readonly"),
            MembershipRole::Member => write!(f, "member"),
            MembershipRole::Admin => write!(f, "admin"),
            MembershipRole::Owner => write!(f, "owner"),
        }
    }
}

impl std::str::FromStr for MembershipRole {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "readonly" => Ok(MembershipRole::Readonly),
            "member" => Ok(MembershipRole::Member),
            "admin" => Ok(MembershipRole::Admin),
            "owner" => Ok(MembershipRole::Owner),
            _ => Err(format!("Unknown role: {}", s)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_role_default_is_member() {
        assert_eq!(MembershipRole::default(), MembershipRole::Member);
    }

    #[test]
    fn test_role_levels_ascending() {
        assert!(MembershipRole::Readonly.level() < MembershipRole::Member.level());
        assert!(MembershipRole::Member.level() < MembershipRole::Admin.level());
        assert!(MembershipRole::Admin.level() < MembershipRole::Owner.level());
    }

    #[test]
    fn test_role_can_write() {
        assert!(!MembershipRole::Readonly.can_write());
        assert!(MembershipRole::Member.can_write());
        assert!(MembershipRole::Admin.can_write());
        assert!(MembershipRole::Owner.can_write());
    }

    #[test]
    fn test_role_can_manage_users() {
        assert!(!MembershipRole::Readonly.can_manage_users());
        assert!(!MembershipRole::Member.can_manage_users());
        assert!(MembershipRole::Admin.can_manage_users());
        assert!(MembershipRole::Owner.can_manage_users());
    }

    #[test]
    fn test_role_can_delete_tenant() {
        assert!(!MembershipRole::Admin.can_delete_tenant());
        assert!(MembershipRole::Owner.can_delete_tenant());
    }

    #[test]
    fn test_role_display_roundtrip() {
        let roles = [
            MembershipRole::Readonly,
            MembershipRole::Member,
            MembershipRole::Admin,
            MembershipRole::Owner,
        ];
        for role in &roles {
            let s = role.to_string();
            let parsed: MembershipRole = s.parse().unwrap();
            assert_eq!(*role, parsed);
        }
    }

    #[test]
    fn test_role_from_str_error() {
        assert!("superadmin".parse::<MembershipRole>().is_err());
    }

    #[test]
    fn test_membership_new_defaults() {
        let uid = Uuid::new_v4();
        let tid = Uuid::new_v4();
        let m = Membership::new(uid, tid, MembershipRole::Admin);
        assert_eq!(m.user_id, uid);
        assert_eq!(m.tenant_id, tid);
        assert_eq!(m.role, MembershipRole::Admin);
        assert!(m.is_active);
        assert!(m.workspace_id.is_none());
        assert!(m.metadata.is_empty());
    }

    #[test]
    fn test_membership_has_role() {
        let uid = Uuid::new_v4();
        let tid = Uuid::new_v4();
        let m = Membership::new(uid, tid, MembershipRole::Admin);
        assert!(m.has_role(MembershipRole::Member));
        assert!(m.has_role(MembershipRole::Admin));
        assert!(!m.has_role(MembershipRole::Owner));
    }

    #[test]
    fn test_membership_can_access_workspace() {
        let uid = Uuid::new_v4();
        let tid = Uuid::new_v4();
        let wid = Uuid::new_v4();
        // No workspace scope = access to all
        let m = Membership::new(uid, tid, MembershipRole::Member);
        assert!(m.can_access_workspace(&wid));
        // Scoped to specific workspace
        let m2 = m.clone().for_workspace(wid);
        assert!(m2.can_access_workspace(&wid));
        assert!(!m2.can_access_workspace(&Uuid::new_v4()));
    }

    #[test]
    fn test_inactive_membership_no_access() {
        let uid = Uuid::new_v4();
        let tid = Uuid::new_v4();
        let wid = Uuid::new_v4();
        let mut m = Membership::new(uid, tid, MembershipRole::Owner);
        m.is_active = false;
        assert!(!m.can_access_workspace(&wid));
    }
}
