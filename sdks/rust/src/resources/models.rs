//! Models resource.

use crate::client::EdgeQuakeClient;
use crate::error::Result;
use crate::types::operations::*;

pub struct ModelsResource<'a> {
    pub(crate) client: &'a EdgeQuakeClient,
}

impl<'a> ModelsResource<'a> {
    /// `GET /api/v1/models`
    pub async fn list(&self) -> Result<Vec<ModelInfo>> {
        self.client.get("/api/v1/models").await
    }

    /// `GET /api/v1/models/providers/health`
    pub async fn providers_health(&self) -> Result<ProvidersHealth> {
        self.client.get("/api/v1/models/providers/health").await
    }

    /// `GET /api/v1/settings/provider`
    pub async fn current_provider(&self) -> Result<ProviderStatus> {
        self.client.get("/api/v1/settings/provider").await
    }

    /// `PUT /api/v1/settings/provider`
    pub async fn set_provider(&self, provider: &str) -> Result<ProviderStatus> {
        let body = serde_json::json!({ "provider": provider });
        self.client
            .put("/api/v1/settings/provider", Some(&body))
            .await
    }
}
