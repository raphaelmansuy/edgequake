//! Workspace service for managing workspaces within tenants.
//!
//! This service provides CRUD operations for workspaces (knowledge bases)
//! and integrates with the RLS system for isolation.
//!
//! ## Implements
//!
//! @implements FEAT0016 (Workspace Management)
//! @implements FEAT0820 (Workspace CRUD operations)
//! @implements FEAT0821 (Tenant management)
//! @implements FEAT0822 (Membership and role management)
//! @implements FEAT0823 (Workspace statistics)
//!
//! ## Use Cases
//!
//! - **UC2410**: Admin creates new workspace for team
//! - **UC2411**: User lists workspaces they have access to
//! - **UC2412**: Admin invites user to workspace with role
//! - **UC2413**: System reports workspace usage statistics
//!
//! ## Enforces
//!
//! - **BR0820**: Workspace names unique within tenant
//! - **BR0821**: Workspace deletion cascades to all resources

mod in_memory;

use async_trait::async_trait;
use std::sync::Arc;
use uuid::Uuid;

use crate::error::Result;
use crate::types::{
    CreateWorkspaceRequest, Membership, MembershipRole, MetricsSnapshot, MetricsTriggerType,
    Tenant, TenantContext, UpdateWorkspaceRequest, Workspace, WorkspaceStats,
};

pub use in_memory::InMemoryWorkspaceService;

/// Result returned by `update_tenant_quota`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UpdateTenantQuotaResult {
    /// The tenant whose quota was updated.
    pub tenant_id: Uuid,
    /// New max_workspaces value.
    pub max_workspaces: usize,
    /// Previous max_workspaces value.
    pub previous_max_workspaces: usize,
    /// Current number of workspaces (used during validation).
    pub current_workspace_count: usize,
}

/// Service trait for workspace management.
#[async_trait]
pub trait WorkspaceService: Send + Sync {
    // ============ Tenant Operations ============

    /// Create a new tenant.
    async fn create_tenant(&self, tenant: Tenant) -> Result<Tenant>;

    /// Get a tenant by ID.
    async fn get_tenant(&self, tenant_id: Uuid) -> Result<Option<Tenant>>;

    /// Get a tenant by slug.
    async fn get_tenant_by_slug(&self, slug: &str) -> Result<Option<Tenant>>;

    /// Update a tenant.
    async fn update_tenant(&self, tenant: Tenant) -> Result<Tenant>;

    /// Delete a tenant and all its workspaces.
    async fn delete_tenant(&self, tenant_id: Uuid) -> Result<()>;

    /// List all tenants (admin only).
    async fn list_tenants(&self, limit: usize, offset: usize) -> Result<Vec<Tenant>>;

    // ============ Workspace Operations ============

    /// Create a new workspace within a tenant.
    async fn create_workspace(
        &self,
        tenant_id: Uuid,
        request: CreateWorkspaceRequest,
    ) -> Result<Workspace>;

    /// Insert a workspace with a specific ID (for syncing from external storage).
    async fn insert_workspace(&self, workspace: Workspace) -> Result<Workspace>;

    /// Get a workspace by ID.
    async fn get_workspace(&self, workspace_id: Uuid) -> Result<Option<Workspace>>;

    /// Get a workspace by tenant and slug.
    async fn get_workspace_by_slug(&self, tenant_id: Uuid, slug: &str)
        -> Result<Option<Workspace>>;

    /// Update a workspace.
    async fn update_workspace(
        &self,
        workspace_id: Uuid,
        request: UpdateWorkspaceRequest,
    ) -> Result<Workspace>;

    /// Delete a workspace and all its data.
    async fn delete_workspace(&self, workspace_id: Uuid) -> Result<()>;

    /// List all workspaces for a tenant.
    async fn list_workspaces(&self, tenant_id: Uuid) -> Result<Vec<Workspace>>;

    /// Get workspace statistics.
    async fn get_workspace_stats(&self, workspace_id: Uuid) -> Result<WorkspaceStats>;

    /// Seed the built-in default tenant + workspace at their canonical UUIDs.
    ///
    /// No-op for production backends that bootstrap the default workspace
    /// themselves (e.g. via migrations). The in-memory test backend overrides
    /// this so that ingestion paths resolving `workspace_id = "default"` find a
    /// real workspace record (P-G2b: async upload requires it).
    async fn seed_default_workspace(&self) {
        // default no-op
    }

    // ============ Metrics Operations ============

    /// Record a metrics snapshot for time-series analysis.
    async fn record_metrics_snapshot(
        &self,
        workspace_id: Uuid,
        trigger_type: MetricsTriggerType,
    ) -> Result<MetricsSnapshot>;

    /// Get metrics history for a workspace.
    async fn get_metrics_history(
        &self,
        workspace_id: Uuid,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<MetricsSnapshot>>;

    // ============ Membership Operations ============

    /// Add a membership (user access to tenant/workspace).
    async fn add_membership(&self, membership: Membership) -> Result<Membership>;

    /// Get memberships for a user.
    async fn get_user_memberships(&self, user_id: Uuid) -> Result<Vec<Membership>>;

    /// Get memberships for a tenant.
    async fn get_tenant_memberships(&self, tenant_id: Uuid) -> Result<Vec<Membership>>;

    /// Update a membership role.
    async fn update_membership_role(
        &self,
        membership_id: Uuid,
        role: MembershipRole,
    ) -> Result<Membership>;

    /// Remove a membership.
    async fn remove_membership(&self, membership_id: Uuid) -> Result<()>;

    /// Check if a user has access to a tenant.
    async fn check_tenant_access(&self, user_id: Uuid, tenant_id: Uuid) -> Result<bool>;

    /// Check if a user has access to a workspace.
    async fn check_workspace_access(&self, user_id: Uuid, workspace_id: Uuid) -> Result<bool>;

    /// Get user's role in a tenant.
    async fn get_user_role(&self, user_id: Uuid, tenant_id: Uuid)
        -> Result<Option<MembershipRole>>;

    // ============ Context Operations ============

    /// Build a tenant context for RLS from user and request info.
    async fn build_context(
        &self,
        user_id: Uuid,
        tenant_id: Uuid,
        workspace_id: Option<Uuid>,
    ) -> Result<TenantContext>;

    // ============ Quota Operations (SPEC-0001) ============

    /// Update the max_workspaces quota for a specific tenant.
    async fn update_tenant_quota(
        &self,
        tenant_id: Uuid,
        new_max_workspaces: usize,
    ) -> Result<UpdateTenantQuotaResult>;

    /// Get the server-wide default max_workspaces for new tenants.
    async fn get_server_default_max_workspaces(&self) -> Result<usize>;

    /// Set the server-wide default max_workspaces for new tenants.
    async fn set_server_default_max_workspaces(&self, value: usize) -> Result<usize>;
}

/// Factory for creating workspace services.
pub struct WorkspaceServiceFactory;

impl WorkspaceServiceFactory {
    /// Create an in-memory workspace service (for testing).
    pub fn in_memory() -> Arc<dyn WorkspaceService> {
        Arc::new(InMemoryWorkspaceService::new())
    }

    /// Create a workspace service with a default tenant.
    pub async fn with_default_tenant() -> Arc<dyn WorkspaceService> {
        Arc::new(InMemoryWorkspaceService::with_default_tenant().await)
    }
}
