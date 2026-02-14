//! Settings resource — provider status and available providers.
//!
//! WHY: Exposes /api/v1/settings/* endpoints for switching and checking
//! LLM/embedding providers. Added in OODA-31.

use crate::client::EdgeQuakeClient;
use crate::error::Result;
use crate::types::operations::ProviderStatus;

pub struct SettingsResource<'a> {
    pub(crate) client: &'a EdgeQuakeClient,
}

impl<'a> SettingsResource<'a> {
    /// `GET /api/v1/settings/provider/status`
    ///
    /// Returns current provider health and configuration status.
    pub async fn provider_status(&self) -> Result<ProviderStatus> {
        self.client.get("/api/v1/settings/provider/status").await
    }

    /// `GET /api/v1/settings/providers`
    ///
    /// Lists all available providers and their capabilities.
    pub async fn list_providers(&self) -> Result<Vec<serde_json::Value>> {
        self.client.get("/api/v1/settings/providers").await
    }
}
