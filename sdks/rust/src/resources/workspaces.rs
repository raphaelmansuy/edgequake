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

    /// `GET /api/v1/workspaces/{id}` — Get workspace by ID.
    pub async fn get(&self, workspace_id: &str) -> Result<WorkspaceInfo> {
        self.client
            .get(&format!("/api/v1/workspaces/{workspace_id}"))
            .await
    }

    /// `PUT /api/v1/workspaces/{id}` — Update workspace.
    pub async fn update(
        &self,
        workspace_id: &str,
        body: &serde_json::Value,
    ) -> Result<WorkspaceInfo> {
        self.client
            .put(
                &format!("/api/v1/workspaces/{workspace_id}"),
                Some(body),
            )
            .await
    }

    /// `DELETE /api/v1/workspaces/{id}` — Delete workspace.
    pub async fn delete(&self, workspace_id: &str) -> Result<()> {
        self.client
            .delete_no_content(&format!("/api/v1/workspaces/{workspace_id}"))
            .await
    }

    /// `GET /api/v1/workspaces/{id}/metrics-history`
    pub async fn metrics_history(&self, workspace_id: &str) -> Result<Vec<serde_json::Value>> {
        self.client
            .get(&format!("/api/v1/workspaces/{workspace_id}/metrics-history"))
            .await
    }

    /// `POST /api/v1/workspaces/{id}/rebuild-embeddings`
    pub async fn rebuild_embeddings(&self, workspace_id: &str) -> Result<serde_json::Value> {
        self.client
            .post::<(), serde_json::Value>(
                &format!("/api/v1/workspaces/{workspace_id}/rebuild-embeddings"),
                None,
            )
            .await
    }

    /// `POST /api/v1/workspaces/{id}/rebuild-knowledge-graph`
    pub async fn rebuild_knowledge_graph(&self, workspace_id: &str) -> Result<serde_json::Value> {
        self.client
            .post::<(), serde_json::Value>(
                &format!("/api/v1/workspaces/{workspace_id}/rebuild-knowledge-graph"),
                None,
            )
            .await
    }

    /// `POST /api/v1/workspaces/{id}/reprocess-documents`
    pub async fn reprocess_documents(&self, workspace_id: &str) -> Result<serde_json::Value> {
        self.client
            .post::<(), serde_json::Value>(
                &format!("/api/v1/workspaces/{workspace_id}/reprocess-documents"),
                None,
            )
            .await
    }
}
