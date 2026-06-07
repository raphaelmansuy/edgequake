#[cfg(feature = "postgres")]
use async_trait::async_trait;

#[cfg(feature = "postgres")]
use uuid::Uuid;

#[cfg(feature = "postgres")]
use crate::{
    error::Result,
    types::{
        CreateWorkspaceRequest, Membership, MembershipRole, MetricsSnapshot, MetricsTriggerType,
        Tenant, TenantContext, UpdateWorkspaceRequest, Workspace, WorkspaceStats,
    },
    workspace_service::{UpdateTenantQuotaResult, WorkspaceService},
};

#[cfg(feature = "postgres")]
use super::WorkspaceServiceImpl;

#[cfg(feature = "postgres")]
#[async_trait]
impl WorkspaceService for WorkspaceServiceImpl {
    async fn create_tenant(&self, tenant: Tenant) -> Result<Tenant> {
        self.pg_create_tenant(tenant).await
    }

    async fn get_tenant(&self, tenant_id: Uuid) -> Result<Option<Tenant>> {
        self.pg_get_tenant(tenant_id).await
    }

    async fn get_tenant_by_slug(&self, slug: &str) -> Result<Option<Tenant>> {
        self.pg_get_tenant_by_slug(slug).await
    }

    async fn update_tenant(&self, tenant: Tenant) -> Result<Tenant> {
        self.pg_update_tenant(tenant).await
    }

    async fn delete_tenant(&self, tenant_id: Uuid) -> Result<()> {
        self.pg_delete_tenant(tenant_id).await
    }

    async fn list_tenants(&self, limit: usize, offset: usize) -> Result<Vec<Tenant>> {
        self.pg_list_tenants(limit, offset).await
    }

    async fn insert_workspace(&self, workspace: Workspace) -> Result<Workspace> {
        self.pg_insert_workspace(workspace).await
    }

    async fn get_workspace(&self, workspace_id: Uuid) -> Result<Option<Workspace>> {
        self.pg_get_workspace(workspace_id).await
    }

    async fn delete_workspace(&self, workspace_id: Uuid) -> Result<()> {
        self.pg_delete_workspace(workspace_id).await
    }

    async fn list_workspaces(&self, tenant_id: Uuid) -> Result<Vec<Workspace>> {
        self.pg_list_workspaces(tenant_id).await
    }

    async fn get_workspace_stats(&self, workspace_id: Uuid) -> Result<WorkspaceStats> {
        self.pg_get_workspace_stats(workspace_id).await
    }

    async fn add_membership(&self, membership: Membership) -> Result<Membership> {
        self.pg_add_membership(membership).await
    }

    async fn get_user_memberships(&self, user_id: Uuid) -> Result<Vec<Membership>> {
        self.pg_get_user_memberships(user_id).await
    }

    async fn get_tenant_memberships(&self, tenant_id: Uuid) -> Result<Vec<Membership>> {
        self.pg_get_tenant_memberships(tenant_id).await
    }

    async fn remove_membership(&self, membership_id: Uuid) -> Result<()> {
        self.pg_remove_membership(membership_id).await
    }

    async fn check_tenant_access(&self, user_id: Uuid, tenant_id: Uuid) -> Result<bool> {
        self.pg_check_tenant_access(user_id, tenant_id).await
    }

    async fn check_workspace_access(&self, user_id: Uuid, workspace_id: Uuid) -> Result<bool> {
        self.pg_check_workspace_access(user_id, workspace_id).await
    }

    async fn get_server_default_max_workspaces(&self) -> Result<usize> {
        self.pg_get_server_default_max_workspaces().await
    }

    async fn set_server_default_max_workspaces(&self, value: usize) -> Result<usize> {
        self.pg_set_server_default_max_workspaces(value).await
    }

    async fn create_workspace(
        &self,
        tenant_id: Uuid,
        request: CreateWorkspaceRequest,
    ) -> Result<Workspace> {
        self.pg_create_workspace(tenant_id, request).await
    }

    async fn get_workspace_by_slug(
        &self,
        tenant_id: Uuid,
        slug: &str,
    ) -> Result<Option<Workspace>> {
        self.pg_get_workspace_by_slug(tenant_id, slug).await
    }

    async fn update_workspace(
        &self,
        workspace_id: Uuid,
        request: UpdateWorkspaceRequest,
    ) -> Result<Workspace> {
        self.pg_update_workspace(workspace_id, request).await
    }

    async fn record_metrics_snapshot(
        &self,
        workspace_id: Uuid,
        trigger_type: MetricsTriggerType,
    ) -> Result<MetricsSnapshot> {
        self.pg_record_metrics_snapshot(workspace_id, trigger_type)
            .await
    }

    async fn get_metrics_history(
        &self,
        workspace_id: Uuid,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<MetricsSnapshot>> {
        self.pg_get_metrics_history(workspace_id, limit, offset)
            .await
    }

    async fn update_membership_role(
        &self,
        membership_id: Uuid,
        role: MembershipRole,
    ) -> Result<Membership> {
        self.pg_update_membership_role(membership_id, role).await
    }

    async fn get_user_role(
        &self,
        user_id: Uuid,
        tenant_id: Uuid,
    ) -> Result<Option<MembershipRole>> {
        self.pg_get_user_role(user_id, tenant_id).await
    }

    async fn build_context(
        &self,
        user_id: Uuid,
        tenant_id: Uuid,
        workspace_id: Option<Uuid>,
    ) -> Result<TenantContext> {
        self.pg_build_context(user_id, tenant_id, workspace_id)
            .await
    }

    async fn update_tenant_quota(
        &self,
        tenant_id: Uuid,
        new_max_workspaces: usize,
    ) -> Result<UpdateTenantQuotaResult> {
        self.pg_update_tenant_quota(tenant_id, new_max_workspaces)
            .await
    }
}
