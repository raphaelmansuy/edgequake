//! Multi-tenant workspace and isolation types.
//!
//! This module provides the domain types for multi-tenant isolation:
//! - `Tenant` - A top-level organization/customer
//! - `Workspace` - A document workspace within a tenant (knowledge base)
//! - `Membership` - User access to tenants/workspaces
//! - `TenantContext` - Current request context for RLS

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// A tenant represents an organization or customer in the multi-tenant system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tenant {
    /// Unique tenant identifier.
    pub tenant_id: Uuid,
    /// Human-readable name.
    pub name: String,
    /// URL-safe slug for routing.
    pub slug: String,
    /// Optional description.
    pub description: Option<String>,
    /// Subscription plan.
    pub plan: TenantPlan,
    /// Maximum number of workspaces allowed.
    pub max_workspaces: usize,
    /// Maximum number of users allowed.
    pub max_users: usize,
    /// Whether the tenant is active.
    pub is_active: bool,
    /// Creation timestamp.
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Last update timestamp.
    pub updated_at: chrono::DateTime<chrono::Utc>,
    /// Custom metadata.
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Tenant {
    /// Create a new tenant with defaults.
    pub fn new(name: impl Into<String>, slug: impl Into<String>) -> Self {
        let now = chrono::Utc::now();
        Self {
            tenant_id: Uuid::new_v4(),
            name: name.into(),
            slug: slug.into(),
            description: None,
            plan: TenantPlan::Free,
            max_workspaces: 5,
            max_users: 10,
            is_active: true,
            created_at: now,
            updated_at: now,
            metadata: HashMap::new(),
        }
    }

    /// Set the tenant plan.
    pub fn with_plan(mut self, plan: TenantPlan) -> Self {
        self.plan = plan;
        self.max_workspaces = plan.default_max_workspaces();
        self.max_users = plan.default_max_users();
        self
    }

    /// Set the description.
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }
}

/// Tenant subscription plans.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TenantPlan {
    /// Free tier with limited resources.
    #[default]
    Free,
    /// Basic paid tier.
    Basic,
    /// Professional tier.
    Pro,
    /// Enterprise tier with custom limits.
    Enterprise,
}

impl TenantPlan {
    /// Get the default max workspaces for this plan.
    pub fn default_max_workspaces(&self) -> usize {
        match self {
            TenantPlan::Free => 2,
            TenantPlan::Basic => 5,
            TenantPlan::Pro => 20,
            TenantPlan::Enterprise => 100,
        }
    }

    /// Get the default max users for this plan.
    pub fn default_max_users(&self) -> usize {
        match self {
            TenantPlan::Free => 3,
            TenantPlan::Basic => 10,
            TenantPlan::Pro => 50,
            TenantPlan::Enterprise => 500,
        }
    }

    /// Get the default max documents per workspace.
    pub fn default_max_documents(&self) -> usize {
        match self {
            TenantPlan::Free => 100,
            TenantPlan::Basic => 1000,
            TenantPlan::Pro => 10000,
            TenantPlan::Enterprise => 100000,
        }
    }
}

impl std::fmt::Display for TenantPlan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TenantPlan::Free => write!(f, "free"),
            TenantPlan::Basic => write!(f, "basic"),
            TenantPlan::Pro => write!(f, "pro"),
            TenantPlan::Enterprise => write!(f, "enterprise"),
        }
    }
}

impl std::str::FromStr for TenantPlan {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "free" => Ok(TenantPlan::Free),
            "basic" => Ok(TenantPlan::Basic),
            "pro" => Ok(TenantPlan::Pro),
            "enterprise" => Ok(TenantPlan::Enterprise),
            _ => Err(format!("Unknown plan: {}", s)),
        }
    }
}

/// A workspace (knowledge base) within a tenant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    /// Unique workspace identifier.
    pub workspace_id: Uuid,
    /// Owning tenant ID.
    pub tenant_id: Uuid,
    /// Human-readable name.
    pub name: String,
    /// URL-safe slug (unique within tenant).
    pub slug: String,
    /// Optional description.
    pub description: Option<String>,
    /// Whether the workspace is active.
    pub is_active: bool,
    /// Creation timestamp.
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Last update timestamp.
    pub updated_at: chrono::DateTime<chrono::Utc>,
    /// Custom metadata including quotas.
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Workspace {
    /// Create a new workspace.
    pub fn new(tenant_id: Uuid, name: impl Into<String>, slug: impl Into<String>) -> Self {
        let now = chrono::Utc::now();
        Self {
            workspace_id: Uuid::new_v4(),
            tenant_id,
            name: name.into(),
            slug: slug.into(),
            description: None,
            is_active: true,
            created_at: now,
            updated_at: now,
            metadata: HashMap::new(),
        }
    }

    /// Set the description.
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Set max documents quota.
    pub fn with_max_documents(mut self, max: usize) -> Self {
        self.metadata
            .insert("max_documents".to_string(), serde_json::json!(max));
        self
    }

    /// Get max documents quota.
    pub fn max_documents(&self) -> Option<usize> {
        self.metadata
            .get("max_documents")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
    }
}

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
        matches!(self, MembershipRole::Member | MembershipRole::Admin | MembershipRole::Owner)
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

/// Request to create a new workspace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWorkspaceRequest {
    /// Human-readable name.
    pub name: String,
    /// Optional slug (generated from name if not provided).
    pub slug: Option<String>,
    /// Optional description.
    pub description: Option<String>,
    /// Optional max documents quota.
    pub max_documents: Option<usize>,
}

/// Request to update a workspace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateWorkspaceRequest {
    /// New name (optional).
    pub name: Option<String>,
    /// New description (optional).
    pub description: Option<String>,
    /// Whether the workspace is active.
    pub is_active: Option<bool>,
    /// Max documents quota.
    pub max_documents: Option<usize>,
}

/// Statistics for a workspace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceStats {
    /// Workspace ID.
    pub workspace_id: Uuid,
    /// Total documents.
    pub document_count: usize,
    /// Total entities.
    pub entity_count: usize,
    /// Total relationships.
    pub relationship_count: usize,
    /// Total chunks.
    pub chunk_count: usize,
    /// Storage used in bytes.
    pub storage_bytes: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tenant_creation() {
        let tenant = Tenant::new("Acme Corp", "acme-corp")
            .with_plan(TenantPlan::Pro)
            .with_description("Main production tenant");

        assert_eq!(tenant.name, "Acme Corp");
        assert_eq!(tenant.slug, "acme-corp");
        assert_eq!(tenant.plan, TenantPlan::Pro);
        assert_eq!(tenant.max_workspaces, 20);
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
        let member = Membership::new(user_id, tenant_id, MembershipRole::Member)
            .for_workspace(workspace_id);

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
