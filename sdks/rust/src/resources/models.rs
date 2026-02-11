//! Models resource.
//!
//! WHY: Models endpoints verified against routes.rs.

use crate::client::EdgeQuakeClient;
use crate::error::Result;
use crate::types::operations::*;

pub struct ModelsResource<'a> {
    pub(crate) client: &'a EdgeQuakeClient,
}

impl<'a> ModelsResource<'a> {
    /// `GET /api/v1/models` — returns provider catalog.
    pub async fn list(&self) -> Result<ProviderCatalog> {
        self.client.get("/api/v1/models").await
    }

    /// `GET /api/v1/models/health` — returns bare array of provider health.
    pub async fn providers_health(&self) -> Result<Vec<ProviderHealthInfo>> {
        self.client.get("/api/v1/models/health").await
    }

    /// `GET /api/v1/settings/provider/status`
    pub async fn current_provider(&self) -> Result<ProviderStatus> {
        self.client.get("/api/v1/settings/provider/status").await
    }

    /// `PUT /api/v1/settings/provider`
    pub async fn set_provider(&self, provider: &str) -> Result<ProviderStatus> {
        let body = serde_json::json!({ "provider": provider });
        self.client
            .put("/api/v1/settings/provider", Some(&body))
            .await
    }
}
