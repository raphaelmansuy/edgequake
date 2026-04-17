//! Costs resource.

use crate::client::EdgeQuakeClient;
use crate::error::Result;
use crate::types::operations::*;

pub struct CostsResource<'a> {
    pub(crate) client: &'a EdgeQuakeClient,
}

impl<'a> CostsResource<'a> {
    /// `GET /api/v1/costs/summary`
    pub async fn summary(&self) -> Result<CostSummary> {
        self.client.get("/api/v1/costs/summary").await
    }

    /// `GET /api/v1/costs/history`
    pub async fn history(&self) -> Result<Vec<CostEntry>> {
        self.client.get("/api/v1/costs/history").await
    }

    /// `GET /api/v1/costs/budget`
    pub async fn budget(&self) -> Result<BudgetInfo> {
        self.client.get("/api/v1/costs/budget").await
    }

    /// `PATCH /api/v1/costs/budget` — Update budget settings.
    pub async fn update_budget(&self, body: &serde_json::Value) -> Result<BudgetInfo> {
        self.client.patch("/api/v1/costs/budget", Some(body)).await
    }

    /// `GET /api/v1/pipeline/costs/pricing` — Get model pricing.
    pub async fn pricing(&self) -> Result<serde_json::Value> {
        self.client.get("/api/v1/pipeline/costs/pricing").await
    }

    /// `POST /api/v1/pipeline/costs/estimate` — Estimate cost for a workload.
    pub async fn estimate(&self, body: &serde_json::Value) -> Result<serde_json::Value> {
        self.client
            .post("/api/v1/pipeline/costs/estimate", Some(body))
            .await
    }
}
