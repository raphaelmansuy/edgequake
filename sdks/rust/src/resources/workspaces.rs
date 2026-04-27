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
        let page: WorkspaceListResponse = self
            .client
            .get(&format!("/api/v1/tenants/{tenant_id}/workspaces"))
            .await?;
        Ok(page.items)
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

    /// `POST /api/v1/workspaces/{id}/metrics-snapshot`
    pub async fn trigger_metrics_snapshot(
        &self,
        workspace_id: &str,
    ) -> Result<serde_json::Value> {
        self.client
            .post::<(), serde_json::Value>(
                &format!("/api/v1/workspaces/{workspace_id}/metrics-snapshot"),
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
            .put(&format!("/api/v1/workspaces/{workspace_id}"), Some(body))
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
            .get(&format!(
                "/api/v1/workspaces/{workspace_id}/metrics-history"
            ))
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

    /// `PUT /api/v1/workspaces/{workspace_id}/injection` — knowledge injection (text).
    pub async fn put_injection(
        &self,
        workspace_id: &str,
        body: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        self.client
            .put(
                &format!("/api/v1/workspaces/{workspace_id}/injection"),
                Some(body),
            )
            .await
    }

    /// `PUT /api/v1/workspaces/{workspace_id}/injection/file` — multipart file injection.
    pub async fn put_injection_file(
        &self,
        workspace_id: &str,
        name: &str,
        file_bytes: Vec<u8>,
        filename: &str,
        content_type: &str,
    ) -> Result<serde_json::Value> {
        let mut fields = std::collections::HashMap::new();
        fields.insert("name".into(), name.to_string());
        self.client
            .upload_multipart(
                &format!("/api/v1/workspaces/{workspace_id}/injection/file"),
                file_bytes,
                filename,
                content_type,
                fields,
            )
            .await
    }

    /// `GET /api/v1/workspaces/{workspace_id}/injections`
    pub async fn list_injections(&self, workspace_id: &str) -> Result<serde_json::Value> {
        self.client
            .get(&format!("/api/v1/workspaces/{workspace_id}/injections"))
            .await
    }

    /// `GET /api/v1/workspaces/{workspace_id}/injections/{injection_id}`
    pub async fn get_injection(
        &self,
        workspace_id: &str,
        injection_id: &str,
    ) -> Result<serde_json::Value> {
        self.client
            .get(&format!(
                "/api/v1/workspaces/{workspace_id}/injections/{injection_id}"
            ))
            .await
    }

    /// `PATCH /api/v1/workspaces/{workspace_id}/injections/{injection_id}`
    pub async fn patch_injection(
        &self,
        workspace_id: &str,
        injection_id: &str,
        body: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        self.client
            .patch(
                &format!(
                    "/api/v1/workspaces/{workspace_id}/injections/{injection_id}"
                ),
                Some(body),
            )
            .await
    }

    /// `DELETE /api/v1/workspaces/{workspace_id}/injections/{injection_id}`
    pub async fn delete_injection(&self, workspace_id: &str, injection_id: &str) -> Result<()> {
        self.client
            .delete_no_content(&format!(
                "/api/v1/workspaces/{workspace_id}/injections/{injection_id}"
            ))
            .await
    }
}
