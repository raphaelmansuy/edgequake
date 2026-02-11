//! Health endpoints.

use crate::client::EdgeQuakeClient;
use crate::error::Result;
use crate::types::common::HealthResponse;

pub struct HealthResource<'a> {
    pub(crate) client: &'a EdgeQuakeClient,
}

impl<'a> HealthResource<'a> {
    /// `GET /health`
    pub async fn check(&self) -> Result<HealthResponse> {
        self.client.get("/health").await
    }
}
