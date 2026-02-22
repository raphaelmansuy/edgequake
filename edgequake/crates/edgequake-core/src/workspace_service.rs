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

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::error::{Error, Result};
use crate::types::{
    CreateWorkspaceRequest, Membership, MembershipRole, MetricsSnapshot, MetricsTriggerType,
    Tenant, TenantContext, TenantPlan, UpdateWorkspaceRequest, Workspace, WorkspaceStats,
};

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

    // ============ Metrics Operations ============

    /// Record a metrics snapshot for time-series analysis.
    ///
    /// This captures the current workspace stats and stores them in
    /// workspace_metrics_history for trend analysis and debugging.
    ///
    /// OODA-20: Implements metrics recording per mission requirement.
    async fn record_metrics_snapshot(
        &self,
        workspace_id: Uuid,
        trigger_type: MetricsTriggerType,
    ) -> Result<MetricsSnapshot>;

    /// Get metrics history for a workspace.
    ///
    /// Returns snapshots in reverse chronological order (newest first).
    ///
    /// OODA-22: Implements metrics history query per mission requirement.
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
}

/// In-memory implementation of WorkspaceService for testing.
pub struct InMemoryWorkspaceService {
    tenants: RwLock<HashMap<Uuid, Tenant>>,
    workspaces: RwLock<HashMap<Uuid, Workspace>>,
    memberships: RwLock<HashMap<Uuid, Membership>>,
}

impl InMemoryWorkspaceService {
    /// Create a new in-memory workspace service.
    pub fn new() -> Self {
        Self {
            tenants: RwLock::new(HashMap::new()),
            workspaces: RwLock::new(HashMap::new()),
            memberships: RwLock::new(HashMap::new()),
        }
    }

    /// Create with a default tenant for testing.
    pub async fn with_default_tenant() -> Self {
        let service = Self::new();

        let tenant = Tenant::new("Default Tenant", "default").with_plan(TenantPlan::Pro);

        service.create_tenant(tenant).await.ok();

        service
    }

    fn generate_slug(name: &str) -> String {
        name.to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '-' })
            .collect::<String>()
            .trim_matches('-')
            .to_string()
    }
}

impl Default for InMemoryWorkspaceService {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl WorkspaceService for InMemoryWorkspaceService {
    async fn create_tenant(&self, tenant: Tenant) -> Result<Tenant> {
        let mut tenants = self.tenants.write().await;

        // Check slug uniqueness
        if tenants.values().any(|t| t.slug == tenant.slug) {
            return Err(Error::validation(format!(
                "Tenant with slug '{}' already exists",
                tenant.slug
            )));
        }

        tenants.insert(tenant.tenant_id, tenant.clone());
        tracing::info!(tenant_id = %tenant.tenant_id, "Created tenant");
        Ok(tenant)
    }

    async fn get_tenant(&self, tenant_id: Uuid) -> Result<Option<Tenant>> {
        let tenants = self.tenants.read().await;
        Ok(tenants.get(&tenant_id).cloned())
    }

    async fn get_tenant_by_slug(&self, slug: &str) -> Result<Option<Tenant>> {
        let tenants = self.tenants.read().await;
        Ok(tenants.values().find(|t| t.slug == slug).cloned())
    }

    async fn update_tenant(&self, tenant: Tenant) -> Result<Tenant> {
        let mut tenants = self.tenants.write().await;

        if !tenants.contains_key(&tenant.tenant_id) {
            return Err(Error::not_found(format!(
                "Tenant {} not found",
                tenant.tenant_id
            )));
        }

        tenants.insert(tenant.tenant_id, tenant.clone());
        Ok(tenant)
    }

    async fn delete_tenant(&self, tenant_id: Uuid) -> Result<()> {
        let mut tenants = self.tenants.write().await;
        let mut workspaces = self.workspaces.write().await;
        let mut memberships = self.memberships.write().await;

        tenants.remove(&tenant_id);

        // Remove all workspaces for this tenant
        workspaces.retain(|_, ws| ws.tenant_id != tenant_id);

        // Remove all memberships for this tenant
        memberships.retain(|_, m| m.tenant_id != tenant_id);

        tracing::info!(tenant_id = %tenant_id, "Deleted tenant and all workspaces");
        Ok(())
    }

    async fn list_tenants(&self, limit: usize, offset: usize) -> Result<Vec<Tenant>> {
        let tenants = self.tenants.read().await;
        Ok(tenants.values().skip(offset).take(limit).cloned().collect())
    }

    async fn create_workspace(
        &self,
        tenant_id: Uuid,
        request: CreateWorkspaceRequest,
    ) -> Result<Workspace> {
        // Check tenant exists
        {
            let tenants = self.tenants.read().await;
            let tenant = tenants
                .get(&tenant_id)
                .ok_or_else(|| Error::not_found(format!("Tenant {} not found", tenant_id)))?;

            // Check workspace limit
            let workspaces = self.workspaces.read().await;
            let current_count = workspaces
                .values()
                .filter(|ws| ws.tenant_id == tenant_id)
                .count();

            if current_count >= tenant.max_workspaces {
                return Err(Error::validation(format!(
                    "Tenant has reached maximum workspace limit ({})",
                    tenant.max_workspaces
                )));
            }
        }

        let slug = request
            .slug
            .unwrap_or_else(|| Self::generate_slug(&request.name));

        // Check slug uniqueness within tenant
        {
            let workspaces = self.workspaces.read().await;
            if workspaces
                .values()
                .any(|ws| ws.tenant_id == tenant_id && ws.slug == slug)
            {
                return Err(Error::validation(format!(
                    "Workspace with slug '{}' already exists in this tenant",
                    slug
                )));
            }
        }

        let mut workspace = Workspace::new(tenant_id, &request.name, &slug);

        if let Some(desc) = request.description {
            workspace = workspace.with_description(desc);
        }

        if let Some(max_docs) = request.max_documents {
            workspace = workspace.with_max_documents(max_docs);
        }

        // SPEC-032: Apply LLM configuration from request
        // Uses auto-detection for provider if not specified
        if let Some(model) = request.llm_model {
            workspace = workspace.with_llm_model(&model);
            // Explicit provider overrides auto-detection
            if let Some(provider) = request.llm_provider {
                workspace = workspace.with_llm_provider(&provider);
            }
        } else if let Some(provider) = request.llm_provider {
            // Provider specified without model - use default model for provider
            workspace = workspace.with_llm_provider(&provider);
        }

        // SPEC-032: Apply embedding configuration from request
        // Uses auto-detection for provider/dimension if not specified
        if let Some(model) = request.embedding_model {
            workspace = workspace.with_embedding_model(&model);
            // Auto-detect provider if not specified
            if let Some(provider) = request.embedding_provider {
                workspace = workspace.with_embedding_provider(&provider);
            } else {
                let detected = Workspace::detect_provider_from_model(&model);
                workspace = workspace.with_embedding_provider(detected);
            }
            // Auto-detect dimension if not specified
            if let Some(dim) = request.embedding_dimension {
                workspace = workspace.with_embedding_dimension(dim);
            } else {
                let detected = Workspace::detect_dimension_from_model(&model);
                workspace = workspace.with_embedding_dimension(detected);
            }
        }

        let mut workspaces = self.workspaces.write().await;
        workspaces.insert(workspace.workspace_id, workspace.clone());

        tracing::info!(
            workspace_id = %workspace.workspace_id,
            tenant_id = %tenant_id,
            "Created workspace"
        );

        Ok(workspace)
    }

    async fn insert_workspace(&self, workspace: Workspace) -> Result<Workspace> {
        // Validate tenant exists
        {
            let tenants = self.tenants.read().await;
            if !tenants.contains_key(&workspace.tenant_id) {
                return Err(Error::not_found(format!(
                    "Tenant {} not found",
                    workspace.tenant_id
                )));
            }
        }

        // Check slug uniqueness within tenant
        {
            let workspaces = self.workspaces.read().await;
            if workspaces.values().any(|ws| {
                ws.tenant_id == workspace.tenant_id
                    && ws.slug == workspace.slug
                    && ws.workspace_id != workspace.workspace_id
            }) {
                return Err(Error::validation(format!(
                    "Workspace with slug '{}' already exists in this tenant",
                    workspace.slug
                )));
            }
        }

        let mut workspaces = self.workspaces.write().await;
        workspaces.insert(workspace.workspace_id, workspace.clone());

        tracing::info!(
            workspace_id = %workspace.workspace_id,
            tenant_id = %workspace.tenant_id,
            "Inserted workspace with specific ID"
        );

        Ok(workspace)
    }

    async fn get_workspace(&self, workspace_id: Uuid) -> Result<Option<Workspace>> {
        let workspaces = self.workspaces.read().await;
        Ok(workspaces.get(&workspace_id).cloned())
    }

    async fn get_workspace_by_slug(
        &self,
        tenant_id: Uuid,
        slug: &str,
    ) -> Result<Option<Workspace>> {
        let workspaces = self.workspaces.read().await;
        Ok(workspaces
            .values()
            .find(|ws| ws.tenant_id == tenant_id && ws.slug == slug)
            .cloned())
    }

    async fn update_workspace(
        &self,
        workspace_id: Uuid,
        request: UpdateWorkspaceRequest,
    ) -> Result<Workspace> {
        let mut workspaces = self.workspaces.write().await;

        let workspace = workspaces
            .get_mut(&workspace_id)
            .ok_or_else(|| Error::not_found(format!("Workspace {} not found", workspace_id)))?;

        if let Some(name) = request.name {
            workspace.name = name;
        }

        if let Some(desc) = request.description {
            workspace.description = Some(desc);
        }

        if let Some(is_active) = request.is_active {
            workspace.is_active = is_active;
        }

        if let Some(max_docs) = request.max_documents {
            workspace
                .metadata
                .insert("max_documents".to_string(), serde_json::json!(max_docs));
        }

        // SPEC-032: LLM model configuration updates
        if let Some(llm_model) = request.llm_model {
            workspace.llm_model = llm_model;
        }
        if let Some(llm_provider) = request.llm_provider {
            workspace.llm_provider = llm_provider;
        }
        // SPEC-032: Embedding model configuration updates
        if let Some(embedding_model) = request.embedding_model {
            workspace.embedding_model = embedding_model;
        }
        if let Some(embedding_provider) = request.embedding_provider {
            workspace.embedding_provider = embedding_provider;
        }
        if let Some(embedding_dimension) = request.embedding_dimension {
            workspace.embedding_dimension = embedding_dimension;
        }

        workspace.updated_at = chrono::Utc::now();

        Ok(workspace.clone())
    }

    async fn delete_workspace(&self, workspace_id: Uuid) -> Result<()> {
        let mut workspaces = self.workspaces.write().await;
        let mut memberships = self.memberships.write().await;

        workspaces.remove(&workspace_id);
        memberships.retain(|_, m| m.workspace_id != Some(workspace_id));

        tracing::info!(workspace_id = %workspace_id, "Deleted workspace");
        Ok(())
    }

    async fn list_workspaces(&self, tenant_id: Uuid) -> Result<Vec<Workspace>> {
        let workspaces = self.workspaces.read().await;
        Ok(workspaces
            .values()
            .filter(|ws| ws.tenant_id == tenant_id)
            .cloned()
            .collect())
    }

    async fn get_workspace_stats(&self, workspace_id: Uuid) -> Result<WorkspaceStats> {
        // WHY zeros: In-memory implementation is a stub for single-tenant mode.
        // Real metrics require storage adapters which are not available here.
        // TODO: Accept storage adapters in constructor for real-time counting.
        Ok(WorkspaceStats {
            workspace_id,
            document_count: 0,
            entity_count: 0,
            relationship_count: 0,
            chunk_count: 0,
            embedding_count: 0,
            storage_bytes: 0,
        })
    }

    async fn record_metrics_snapshot(
        &self,
        workspace_id: Uuid,
        trigger_type: MetricsTriggerType,
    ) -> Result<MetricsSnapshot> {
        // WHY stub: In-memory implementation doesn't persist history.
        // Returns a snapshot with current (zero) stats for testing compatibility.
        // OODA-20: Real implementation is in PostgresWorkspaceService.
        Ok(MetricsSnapshot {
            id: Uuid::new_v4(),
            workspace_id,
            recorded_at: chrono::Utc::now(),
            trigger_type,
            document_count: 0,
            chunk_count: 0,
            entity_count: 0,
            relationship_count: 0,
            embedding_count: 0,
            storage_bytes: 0,
        })
    }

    async fn get_metrics_history(
        &self,
        _workspace_id: Uuid,
        _limit: usize,
        _offset: usize,
    ) -> Result<Vec<MetricsSnapshot>> {
        // WHY empty: In-memory implementation doesn't persist history.
        // OODA-22: Real implementation is in PostgresWorkspaceService.
        Ok(Vec::new())
    }

    async fn add_membership(&self, membership: Membership) -> Result<Membership> {
        let mut memberships = self.memberships.write().await;

        // Check for existing membership
        let exists = memberships.values().any(|m| {
            m.user_id == membership.user_id
                && m.tenant_id == membership.tenant_id
                && m.workspace_id == membership.workspace_id
        });

        if exists {
            return Err(Error::validation("Membership already exists"));
        }

        memberships.insert(membership.membership_id, membership.clone());

        tracing::info!(
            membership_id = %membership.membership_id,
            user_id = %membership.user_id,
            tenant_id = %membership.tenant_id,
            "Added membership"
        );

        Ok(membership)
    }

    async fn get_user_memberships(&self, user_id: Uuid) -> Result<Vec<Membership>> {
        let memberships = self.memberships.read().await;
        Ok(memberships
            .values()
            .filter(|m| m.user_id == user_id && m.is_active)
            .cloned()
            .collect())
    }

    async fn get_tenant_memberships(&self, tenant_id: Uuid) -> Result<Vec<Membership>> {
        let memberships = self.memberships.read().await;
        Ok(memberships
            .values()
            .filter(|m| m.tenant_id == tenant_id && m.is_active)
            .cloned()
            .collect())
    }

    async fn update_membership_role(
        &self,
        membership_id: Uuid,
        role: MembershipRole,
    ) -> Result<Membership> {
        let mut memberships = self.memberships.write().await;

        let membership = memberships
            .get_mut(&membership_id)
            .ok_or_else(|| Error::not_found(format!("Membership {} not found", membership_id)))?;

        membership.role = role;

        Ok(membership.clone())
    }

    async fn remove_membership(&self, membership_id: Uuid) -> Result<()> {
        let mut memberships = self.memberships.write().await;
        memberships.remove(&membership_id);
        Ok(())
    }

    async fn check_tenant_access(&self, user_id: Uuid, tenant_id: Uuid) -> Result<bool> {
        let memberships = self.memberships.read().await;
        Ok(memberships
            .values()
            .any(|m| m.user_id == user_id && m.tenant_id == tenant_id && m.is_active))
    }

    async fn check_workspace_access(&self, user_id: Uuid, workspace_id: Uuid) -> Result<bool> {
        let workspaces = self.workspaces.read().await;
        let workspace = match workspaces.get(&workspace_id) {
            Some(ws) => ws,
            None => return Ok(false),
        };

        let memberships = self.memberships.read().await;
        Ok(memberships.values().any(|m| {
            m.user_id == user_id
                && m.tenant_id == workspace.tenant_id
                && m.is_active
                && m.can_access_workspace(&workspace_id)
        }))
    }

    async fn get_user_role(
        &self,
        user_id: Uuid,
        tenant_id: Uuid,
    ) -> Result<Option<MembershipRole>> {
        let memberships = self.memberships.read().await;
        Ok(memberships
            .values()
            .find(|m| m.user_id == user_id && m.tenant_id == tenant_id && m.is_active)
            .map(|m| m.role))
    }

    async fn build_context(
        &self,
        user_id: Uuid,
        tenant_id: Uuid,
        workspace_id: Option<Uuid>,
    ) -> Result<TenantContext> {
        // Check access
        if !self.check_tenant_access(user_id, tenant_id).await? {
            return Err(Error::validation(format!(
                "User {} does not have access to tenant {}",
                user_id, tenant_id
            )));
        }

        // If workspace specified, check access
        if let Some(ws_id) = workspace_id {
            if !self.check_workspace_access(user_id, ws_id).await? {
                return Err(Error::validation(format!(
                    "User {} does not have access to workspace {}",
                    user_id, ws_id
                )));
            }
        }

        let role = self.get_user_role(user_id, tenant_id).await?;

        let mut ctx = TenantContext::new(tenant_id);
        if let Some(ws_id) = workspace_id {
            ctx = ctx.with_workspace(ws_id);
        }
        if let Some(r) = role {
            ctx = ctx.with_user(user_id, r);
        }

        Ok(ctx)
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_tenant() {
        let service = InMemoryWorkspaceService::new();

        let tenant = Tenant::new("Test Tenant", "test-tenant").with_plan(TenantPlan::Basic);

        let created = service.create_tenant(tenant).await.unwrap();
        assert_eq!(created.name, "Test Tenant");
        assert_eq!(created.slug, "test-tenant");
        assert_eq!(created.plan, TenantPlan::Basic);
    }

    #[tokio::test]
    async fn test_create_workspace() {
        let service = InMemoryWorkspaceService::new();

        // Create tenant first
        let tenant = Tenant::new("Test Tenant", "test");
        let tenant = service.create_tenant(tenant).await.unwrap();

        // Create workspace
        let request = CreateWorkspaceRequest {
            name: "My Knowledge Base".to_string(),
            slug: Some("my-kb".to_string()),
            description: Some("Test KB".to_string()),
            max_documents: Some(1000),
            llm_model: None,
            llm_provider: None,
            embedding_model: None,
            embedding_provider: None,
            embedding_dimension: None,
            vision_llm_model: None,
            vision_llm_provider: None,
        };

        let workspace = service
            .create_workspace(tenant.tenant_id, request)
            .await
            .unwrap();
        assert_eq!(workspace.name, "My Knowledge Base");
        assert_eq!(workspace.slug, "my-kb");
        assert_eq!(workspace.max_documents(), Some(1000));
    }

    #[tokio::test]
    async fn test_workspace_limit() {
        let service = InMemoryWorkspaceService::new();

        // Create tenant with limit of 2 workspaces
        let mut tenant = Tenant::new("Limited Tenant", "limited");
        tenant.max_workspaces = 2;
        let tenant = service.create_tenant(tenant).await.unwrap();

        // Create 2 workspaces (should succeed)
        for i in 0..2 {
            let request = CreateWorkspaceRequest {
                name: format!("Workspace {}", i),
                slug: Some(format!("ws-{}", i)),
                description: None,
                max_documents: None,
                llm_model: None,
                llm_provider: None,
                embedding_model: None,
                embedding_provider: None,
                embedding_dimension: None,
                vision_llm_model: None,
                vision_llm_provider: None,
            };
            service
                .create_workspace(tenant.tenant_id, request)
                .await
                .unwrap();
        }

        // Third workspace should fail
        let request = CreateWorkspaceRequest {
            name: "Workspace 3".to_string(),
            slug: Some("ws-3".to_string()),
            description: None,
            max_documents: None,
            llm_model: None,
            llm_provider: None,
            embedding_model: None,
            embedding_provider: None,
            embedding_dimension: None,
            vision_llm_model: None,
            vision_llm_provider: None,
        };
        let result = service.create_workspace(tenant.tenant_id, request).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_membership_access() {
        let service = InMemoryWorkspaceService::new();

        let tenant = Tenant::new("Test", "test");
        let tenant = service.create_tenant(tenant).await.unwrap();

        let user_id = Uuid::new_v4();

        // No access initially
        assert!(!service
            .check_tenant_access(user_id, tenant.tenant_id)
            .await
            .unwrap());

        // Add membership
        let membership = Membership::new(user_id, tenant.tenant_id, MembershipRole::Member);
        service.add_membership(membership).await.unwrap();

        // Now has access
        assert!(service
            .check_tenant_access(user_id, tenant.tenant_id)
            .await
            .unwrap());
    }

    #[tokio::test]
    async fn test_build_context() {
        let service = InMemoryWorkspaceService::new();

        let tenant = Tenant::new("Test", "test");
        let tenant = service.create_tenant(tenant).await.unwrap();

        let user_id = Uuid::new_v4();

        // Without membership, should fail
        let result = service.build_context(user_id, tenant.tenant_id, None).await;
        assert!(result.is_err());

        // Add membership
        let membership = Membership::new(user_id, tenant.tenant_id, MembershipRole::Admin);
        service.add_membership(membership).await.unwrap();

        // Now should succeed
        let ctx = service
            .build_context(user_id, tenant.tenant_id, None)
            .await
            .unwrap();
        assert!(ctx.is_valid());
        assert_eq!(ctx.tenant_id, Some(tenant.tenant_id));
        assert!(ctx.can_write());
    }
}
