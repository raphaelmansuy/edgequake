//! Workspaces resource.

use crate::client::EdgeQuakeClient;
use crate::error::Result;
use crate::types::workspaces::*;

pub struct WorkspacesResource<'a> {
    pub(crate) client: &'a EdgeQuakeClient,
}

impl<'a> WorkspacesResource<'a> {
    /// `GET /api/v1/tenants/{tenant_id}/workspaces`
    pub async fn list(&self, tenant_id: &str) -> Result<Vec<WorkspaceInfo>> {
        self.client
            .get(&format!("/api/v1/tenants/{tenant_id}/workspaces"))
            .await
    }

    /// `POST /api/v1/tenants/{tenant_id}/workspaces`
    pub async fn create(
        &self,
        tenant_id: &str,
        req: &CreateWorkspaceRequest,
    ) -> Result<WorkspaceInfo> {
        self.client
            .post(
                &format!("/api/v1/tenants/{tenant_id}/workspaces"),
                Some(req),
            )
            .await
    }

    /// `GET /api/v1/workspaces/{id}/stats`
    pub async fn stats(&self, workspace_id: &str) -> Result<WorkspaceStats> {
        self.client
            .get(&format!("/api/v1/workspaces/{workspace_id}/stats"))
            .await
    }

    /// `POST /api/v1/workspaces/{id}/rebuild`
    pub async fn rebuild(&self, workspace_id: &str) -> Result<RebuildResponse> {
        self.client
            .post::<(), RebuildResponse>(
                &format!("/api/v1/workspaces/{workspace_id}/rebuild"),
                None,
            )
            .await
    }
}
