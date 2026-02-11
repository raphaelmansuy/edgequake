//! API Keys resource.

use crate::client::EdgeQuakeClient;
use crate::error::Result;
use crate::types::auth::*;

pub struct ApiKeysResource<'a> {
    pub(crate) client: &'a EdgeQuakeClient,
}

impl<'a> ApiKeysResource<'a> {
    /// `GET /api/v1/api-keys`
    pub async fn list(&self) -> Result<Vec<ApiKeyInfo>> {
        self.client.get("/api/v1/api-keys").await
    }

    /// `POST /api/v1/api-keys`
    pub async fn create(&self, name: &str) -> Result<ApiKeyResponse> {
        let body = serde_json::json!({ "name": name });
        self.client.post("/api/v1/api-keys", Some(&body)).await
    }

    /// `DELETE /api/v1/api-keys/{id}`
    pub async fn revoke(&self, id: &str) -> Result<()> {
        self.client
            .delete_no_content(&format!("/api/v1/api-keys/{id}"))
            .await
    }
}
