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

    /// `GET /api/v1/models/llm` — List LLM models only.
    pub async fn list_llm(&self) -> Result<serde_json::Value> {
        self.client.get("/api/v1/models/llm").await
    }

    /// `GET /api/v1/models/embedding` — List embedding models only.
    pub async fn list_embedding(&self) -> Result<serde_json::Value> {
        self.client.get("/api/v1/models/embedding").await
    }

    /// `GET /api/v1/models/{provider}` — Get models for a specific provider.
    pub async fn get_provider(&self, provider: &str) -> Result<serde_json::Value> {
        self.client.get(&format!("/api/v1/models/{provider}")).await
    }

    /// `GET /api/v1/models/{provider}/{model}` — Get specific model details.
    pub async fn get_model(&self, provider: &str, model: &str) -> Result<serde_json::Value> {
        self.client
            .get(&format!("/api/v1/models/{provider}/{model}"))
            .await
    }
}
