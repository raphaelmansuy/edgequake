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

    /// `GET /ready` — Kubernetes readiness probe (plain text, usually `OK`).
    pub async fn ready(&self) -> Result<String> {
        self.client.get_text("/ready").await
    }

    /// `GET /live` — Kubernetes liveness probe (plain text, usually `OK`).
    pub async fn live(&self) -> Result<String> {
        self.client.get_text("/live").await
    }

    /// `GET /metrics` — Prometheus text exposition format.
    pub async fn metrics(&self) -> Result<String> {
        self.client.get_text("/metrics").await
    }
}
