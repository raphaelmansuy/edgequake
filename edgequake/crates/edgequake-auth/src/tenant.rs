//! Multi-tenancy support for EdgeQuake.
//!
//! This module is conditionally compiled when the `multi-tenant` feature is enabled.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::AuthError;
use crate::types::Role;

/// Tenant information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tenant {
    /// Unique tenant identifier.
    pub tenant_id: Uuid,

    /// Tenant name.
    pub name: String,

    /// Tenant slug (URL-safe identifier).
    pub slug: String,

    /// Whether the tenant is active.
    pub is_active: bool,

    /// Tenant plan/tier.
    pub plan: TenantPlan,

    /// Maximum workspaces allowed.
    pub max_workspaces: u32,

    /// Maximum users allowed.
    pub max_users: u32,

    /// Custom settings.
    pub settings: Option<serde_json::Value>,

    /// Creation timestamp.
    pub created_at: DateTime<Utc>,

    /// Last update timestamp.
    pub updated_at: DateTime<Utc>,
}

/// Tenant plan/tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TenantPlan {
    /// Free tier.
    Free,
    /// Basic paid tier.
    Basic,
    /// Professional tier.
    Pro,
    /// Enterprise tier.
    Enterprise,
}

impl TenantPlan {
    /// Get default workspace limit for this plan.
    ///
    /// SPEC-028: Updated to support 500 workspaces by default for Pro/Enterprise.
    /// WHY: Enable large-scale knowledge base organization without artificial limits.
    pub fn default_max_workspaces(&self) -> u32 {
        match self {
            Self::Free => 10,        // Reasonable for trials
            Self::Basic => 100,      // Small teams
            Self::Pro => 500,        // SPEC-028: 500 workspaces target
            Self::Enterprise => 500, // SPEC-028: 500 workspaces target (can be customized)
        }
    }

    /// Get default user limit for this plan.
    pub fn default_max_users(&self) -> u32 {
        match self {
            Self::Free => 3,
            Self::Basic => 10,
            Self::Pro => 50,
            Self::Enterprise => u32::MAX,
        }
    }

    /// Convert to string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Free => "free",
            Self::Basic => "basic",
            Self::Pro => "pro",
            Self::Enterprise => "enterprise",
        }
    }

    /// Parse from string.
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "basic" => Self::Basic,
            "pro" => Self::Pro,
            "enterprise" => Self::Enterprise,
            _ => Self::Free,
        }
    }
}

/// Workspace within a tenant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    /// Unique workspace identifier.
    pub workspace_id: Uuid,

    /// Parent tenant ID.
    pub tenant_id: Uuid,

    /// Workspace name.
    pub name: String,

    /// Workspace slug.
    pub slug: String,

    /// Whether the workspace is active.
    pub is_active: bool,

    /// Custom settings.
    pub settings: Option<serde_json::Value>,

    /// Creation timestamp.
    pub created_at: DateTime<Utc>,

    /// Last update timestamp.
    pub updated_at: DateTime<Utc>,
}

/// Membership linking users to tenants/workspaces.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantMembership {
    /// Membership ID.
    pub membership_id: Uuid,

    /// User ID.
    pub user_id: Uuid,

    /// Tenant ID.
    pub tenant_id: Uuid,

    /// Workspace ID (optional - if None, applies to all workspaces).
    pub workspace_id: Option<Uuid>,

    /// Role within this tenant/workspace.
    pub role: Role,

    /// Whether the membership is active.
    pub is_active: bool,

    /// When the user joined.
    pub joined_at: DateTime<Utc>,
}

/// Current tenant context for a request.
#[derive(Debug, Clone)]
pub struct TenantContext {
    /// Current tenant.
    pub tenant: Arc<Tenant>,

    /// Current workspace (if selected).
    pub workspace: Option<Arc<Workspace>>,

    /// User's role in this tenant/workspace.
    pub role: Role,

    /// User ID.
    pub user_id: Uuid,
}

impl TenantContext {
    /// Create a new tenant context.
    pub fn new(tenant: Tenant, user_id: Uuid, role: Role) -> Self {
        Self {
            tenant: Arc::new(tenant),
            workspace: None,
            role,
            user_id,
        }
    }

    /// Set the current workspace.
    pub fn with_workspace(mut self, workspace: Workspace) -> Self {
        self.workspace = Some(Arc::new(workspace));
        self
    }

    /// Get tenant ID.
    pub fn tenant_id(&self) -> Uuid {
        self.tenant.tenant_id
    }

    /// Get workspace ID if present.
    pub fn workspace_id(&self) -> Option<Uuid> {
        self.workspace.as_ref().map(|w| w.workspace_id)
    }

    /// Check if tenant is active.
    pub fn is_active(&self) -> bool {
        self.tenant.is_active && self.workspace.as_ref().map_or(true, |w| w.is_active)
    }

    /// Check if user is tenant admin.
    pub fn is_tenant_admin(&self) -> bool {
        matches!(self.role, Role::Admin)
    }

    /// Check if user can manage workspace.
    pub fn can_manage_workspace(&self) -> bool {
        matches!(self.role, Role::Admin | Role::User)
    }
}

/// Service for managing tenant contexts.
#[derive(Debug, Clone)]
pub struct TenantService;

impl TenantService {
    /// Create a new tenant service.
    pub fn new() -> Self {
        Self
    }

    /// Validate tenant access.
    pub fn validate_tenant_access(&self, context: &TenantContext) -> Result<(), AuthError> {
        if !context.tenant.is_active {
            return Err(AuthError::TenantNotFound);
        }

        if let Some(workspace) = &context.workspace {
            if !workspace.is_active {
                return Err(AuthError::WorkspaceNotFound);
            }
        }

        Ok(())
    }

    /// Check workspace limit for tenant.
    pub fn check_workspace_limit(
        &self,
        tenant: &Tenant,
        current_count: u32,
    ) -> Result<(), AuthError> {
        if current_count >= tenant.max_workspaces {
            return Err(AuthError::TenantLimitExceeded {
                limit: format!("max_workspaces: {}", tenant.max_workspaces),
            });
        }
        Ok(())
    }

    /// Check user limit for tenant.
    pub fn check_user_limit(&self, tenant: &Tenant, current_count: u32) -> Result<(), AuthError> {
        if current_count >= tenant.max_users {
            return Err(AuthError::TenantLimitExceeded {
                limit: format!("max_users: {}", tenant.max_users),
            });
        }
        Ok(())
    }
}

impl Default for TenantService {
    fn default() -> Self {
        Self::new()
    }
}

/// Request types for tenant management.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTenantRequest {
    /// Tenant name.
    pub name: String,

    /// Tenant slug (optional - will be generated from name if not provided).
    pub slug: Option<String>,

    /// Initial plan.
    pub plan: Option<TenantPlan>,
}

/// Response for tenant creation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTenantResponse {
    /// Created tenant.
    pub tenant: Tenant,
}

/// Request to create a workspace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWorkspaceRequest {
    /// Workspace name.
    pub name: String,

    /// Workspace slug (optional).
    pub slug: Option<String>,
}

/// Response for workspace creation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWorkspaceResponse {
    /// Created workspace.
    pub workspace: Workspace,
}

/// Request to add a member to a tenant/workspace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddMemberRequest {
    /// User ID to add.
    pub user_id: Uuid,

    /// Role for the member.
    pub role: String,

    /// Workspace ID (optional - if None, adds to tenant level).
    pub workspace_id: Option<Uuid>,
}

/// Response for member addition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddMemberResponse {
    /// Created membership.
    pub membership: TenantMembership,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_tenant() -> Tenant {
        Tenant {
            tenant_id: Uuid::new_v4(),
            name: "Test Tenant".to_string(),
            slug: "test-tenant".to_string(),
            is_active: true,
            plan: TenantPlan::Pro,
            max_workspaces: 500, // SPEC-028: Updated to 500
            max_users: 50,
            settings: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn test_workspace(tenant_id: Uuid) -> Workspace {
        Workspace {
            workspace_id: Uuid::new_v4(),
            tenant_id,
            name: "Test Workspace".to_string(),
            slug: "test-workspace".to_string(),
            is_active: true,
            settings: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn test_tenant_plan_limits() {
        // SPEC-028: Updated workspace limits
        assert_eq!(TenantPlan::Free.default_max_workspaces(), 10);
        assert_eq!(TenantPlan::Free.default_max_users(), 3);
        assert_eq!(TenantPlan::Basic.default_max_workspaces(), 100);
        assert_eq!(TenantPlan::Pro.default_max_workspaces(), 500);
        assert_eq!(TenantPlan::Enterprise.default_max_workspaces(), 500);
    }

    #[test]
    fn test_tenant_context() {
        let tenant = test_tenant();
        let workspace = test_workspace(tenant.tenant_id);
        let user_id = Uuid::new_v4();

        let context =
            TenantContext::new(tenant.clone(), user_id, Role::Admin).with_workspace(workspace);

        assert!(context.is_active());
        assert!(context.is_tenant_admin());
        assert!(context.can_manage_workspace());
        assert!(context.workspace_id().is_some());
    }

    #[test]
    fn test_validate_inactive_tenant() {
        let mut tenant = test_tenant();
        tenant.is_active = false;

        let context = TenantContext::new(tenant, Uuid::new_v4(), Role::User);
        let service = TenantService::new();

        let result = service.validate_tenant_access(&context);
        assert!(matches!(result, Err(AuthError::TenantNotFound)));
    }

    #[test]
    fn test_workspace_limit() {
        let tenant = test_tenant();
        let service = TenantService::new();

        // SPEC-028: Pro plan now has 500 workspaces limit
        // Under limit
        assert!(service.check_workspace_limit(&tenant, 499).is_ok());

        // At limit (500 workspaces = max_workspaces for Pro)
        assert!(service.check_workspace_limit(&tenant, 500).is_err());

        // Over limit
        assert!(service.check_workspace_limit(&tenant, 600).is_err());
    }

    #[test]
    fn test_plan_string_conversion() {
        assert_eq!(TenantPlan::Pro.as_str(), "pro");
        assert_eq!(TenantPlan::from_str("PRO"), TenantPlan::Pro);
        assert_eq!(TenantPlan::from_str("unknown"), TenantPlan::Free);
    }
}
