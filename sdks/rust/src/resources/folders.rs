//! Folders resource.

use crate::client::EdgeQuakeClient;
use crate::error::Result;
use crate::types::conversations::*;

pub struct FoldersResource<'a> {
    pub(crate) client: &'a EdgeQuakeClient,
}

impl<'a> FoldersResource<'a> {
    /// `GET /api/v1/folders`
    pub async fn list(&self) -> Result<Vec<FolderInfo>> {
        self.client.get("/api/v1/folders").await
    }

    /// `POST /api/v1/folders`
    pub async fn create(&self, req: &CreateFolderRequest) -> Result<FolderInfo> {
        self.client.post("/api/v1/folders", Some(req)).await
    }

    /// `DELETE /api/v1/folders/{id}`
    pub async fn delete(&self, id: &str) -> Result<()> {
        self.client
            .delete_no_content(&format!("/api/v1/folders/{id}"))
            .await
    }

    /// `PATCH /api/v1/folders/{id}` — Update folder name or settings.
    pub async fn update(
        &self,
        id: &str,
        body: &serde_json::Value,
    ) -> Result<FolderInfo> {
        self.client
            .patch(&format!("/api/v1/folders/{id}"), Some(body))
            .await
    }
}
