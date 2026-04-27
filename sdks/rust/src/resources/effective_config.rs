//! Effective resolved configuration (`GET /api/v1/config/effective`).

use crate::client::EdgeQuakeClient;
use crate::error::Result;

pub struct EffectiveConfigResource<'a> {
    pub(crate) client: &'a EdgeQuakeClient,
}

impl<'a> EffectiveConfigResource<'a> {
    pub async fn get(&self) -> Result<serde_json::Value> {
        self.client.get("/api/v1/config/effective").await
    }
}
