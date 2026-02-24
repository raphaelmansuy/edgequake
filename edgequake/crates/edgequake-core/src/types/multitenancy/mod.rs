//! Multi-tenant workspace and isolation types.
//!
//! This module provides the domain types for multi-tenant isolation:
//! - `Tenant` - A top-level organization/customer
//! - `Workspace` - A document workspace within a tenant (knowledge base)
//! - `Membership` - User access to tenants/workspaces
//! - `TenantContext` - Current request context for RLS

mod context;
mod membership;
mod metrics;
mod requests;
mod tenant;
mod workspace;

pub use context::*;
pub use membership::*;
pub use metrics::*;
pub use requests::*;
pub use tenant::*;
pub use workspace::*;

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_tenant_creation() {
        let tenant = Tenant::new("Acme Corp", "acme-corp")
            .with_plan(TenantPlan::Pro)
            .with_description("Main production tenant");

        assert_eq!(tenant.name, "Acme Corp");
        assert_eq!(tenant.slug, "acme-corp");
        assert_eq!(tenant.plan, TenantPlan::Pro);
        // SPEC-028: Pro plan now allows 500 workspaces
        assert_eq!(tenant.max_workspaces, 500);
        assert!(tenant.is_active);
    }

    #[test]
    fn test_workspace_creation() {
        let tenant_id = Uuid::new_v4();
        let workspace = Workspace::new(tenant_id, "Knowledge Base", "kb-1")
            .with_description("Primary KB")
            .with_max_documents(5000);

        assert_eq!(workspace.tenant_id, tenant_id);
        assert_eq!(workspace.name, "Knowledge Base");
        assert_eq!(workspace.max_documents(), Some(5000));
    }

    #[test]
    fn test_membership_roles() {
        let user_id = Uuid::new_v4();
        let tenant_id = Uuid::new_v4();
        let workspace_id = Uuid::new_v4();

        let owner = Membership::new(user_id, tenant_id, MembershipRole::Owner);
        let member =
            Membership::new(user_id, tenant_id, MembershipRole::Member).for_workspace(workspace_id);

        assert!(owner.has_role(MembershipRole::Admin));
        assert!(owner.can_access_workspace(&workspace_id));
        assert!(member.can_access_workspace(&workspace_id));
        assert!(!member.can_access_workspace(&Uuid::new_v4()));
    }

    #[test]
    fn test_tenant_context() {
        let ctx = TenantContext::new(Uuid::new_v4())
            .with_workspace(Uuid::new_v4())
            .with_user(Uuid::new_v4(), MembershipRole::Member);

        assert!(ctx.is_valid());
        assert!(ctx.can_write());
    }
}
