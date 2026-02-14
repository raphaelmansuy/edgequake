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

    /// `GET /ready` — Kubernetes readiness probe.
    pub async fn ready(&self) -> Result<serde_json::Value> {
        self.client.get("/ready").await
    }

    /// `GET /live` — Kubernetes liveness probe.
    pub async fn live(&self) -> Result<serde_json::Value> {
        self.client.get("/live").await
    }

    /// `GET /metrics` — Prometheus-format metrics.
    pub async fn metrics(&self) -> Result<serde_json::Value> {
        self.client.get("/metrics").await
    }
}
