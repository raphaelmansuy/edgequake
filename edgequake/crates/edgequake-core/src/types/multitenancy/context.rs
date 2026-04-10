//! Tenant context for request-scoped isolation (RLS).

use uuid::Uuid;

use super::membership::MembershipRole;

/// Context for the current tenant/workspace scope.
///
/// This is used to set PostgreSQL session variables for RLS enforcement.
#[derive(Debug, Clone, Default)]
pub struct TenantContext {
    /// Current tenant ID.
    pub tenant_id: Option<Uuid>,
    /// Current workspace ID.
    pub workspace_id: Option<Uuid>,
    /// Current user ID.
    pub user_id: Option<Uuid>,
    /// User's role in the current context.
    pub role: Option<MembershipRole>,
}

impl TenantContext {
    /// Create a new tenant context.
    pub fn new(tenant_id: Uuid) -> Self {
        Self {
            tenant_id: Some(tenant_id),
            workspace_id: None,
            user_id: None,
            role: None,
        }
    }

    /// Set the workspace scope.
    pub fn with_workspace(mut self, workspace_id: Uuid) -> Self {
        self.workspace_id = Some(workspace_id);
        self
    }

    /// Set the user context.
    pub fn with_user(mut self, user_id: Uuid, role: MembershipRole) -> Self {
        self.user_id = Some(user_id);
        self.role = Some(role);
        self
    }

    /// Check if the context is valid (has at least tenant_id).
    pub fn is_valid(&self) -> bool {
        self.tenant_id.is_some()
    }

    /// Check if user can write in this context.
    pub fn can_write(&self) -> bool {
        self.role.map(|r| r.can_write()).unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_all_none() {
        let ctx = TenantContext::default();
        assert!(ctx.tenant_id.is_none());
        assert!(ctx.workspace_id.is_none());
        assert!(ctx.user_id.is_none());
        assert!(ctx.role.is_none());
    }

    #[test]
    fn test_new_sets_tenant() {
        let tid = Uuid::new_v4();
        let ctx = TenantContext::new(tid);
        assert_eq!(ctx.tenant_id, Some(tid));
        assert!(ctx.is_valid());
    }

    #[test]
    fn test_default_not_valid() {
        assert!(!TenantContext::default().is_valid());
    }

    #[test]
    fn test_builder_chain() {
        let tid = Uuid::new_v4();
        let wid = Uuid::new_v4();
        let uid = Uuid::new_v4();
        let ctx = TenantContext::new(tid)
            .with_workspace(wid)
            .with_user(uid, MembershipRole::Admin);
        assert_eq!(ctx.workspace_id, Some(wid));
        assert_eq!(ctx.user_id, Some(uid));
        assert_eq!(ctx.role, Some(MembershipRole::Admin));
    }

    #[test]
    fn test_can_write_no_role() {
        assert!(!TenantContext::default().can_write());
    }

    #[test]
    fn test_can_write_readonly() {
        let tid = Uuid::new_v4();
        let uid = Uuid::new_v4();
        let ctx = TenantContext::new(tid).with_user(uid, MembershipRole::Readonly);
        assert!(!ctx.can_write());
    }

    #[test]
    fn test_can_write_member() {
        let tid = Uuid::new_v4();
        let uid = Uuid::new_v4();
        let ctx = TenantContext::new(tid).with_user(uid, MembershipRole::Member);
        assert!(ctx.can_write());
    }
}
