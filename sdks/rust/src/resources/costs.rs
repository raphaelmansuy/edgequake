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
}
