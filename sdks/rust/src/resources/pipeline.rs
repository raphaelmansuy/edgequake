//! Pipeline resource.

use crate::client::EdgeQuakeClient;
use crate::error::Result;
use crate::types::operations::*;

pub struct PipelineResource<'a> {
    pub(crate) client: &'a EdgeQuakeClient,
}

impl<'a> PipelineResource<'a> {
    /// `GET /api/v1/pipeline/status`
    pub async fn status(&self) -> Result<PipelineStatus> {
        self.client.get("/api/v1/pipeline/status").await
    }

    /// `GET /api/v1/pipeline/queue-metrics`
    pub async fn metrics(&self) -> Result<QueueMetrics> {
        self.client.get("/api/v1/pipeline/queue-metrics").await
    }

    /// `POST /api/v1/pipeline/cancel` — Cancel active pipeline.
    pub async fn cancel(&self) -> Result<()> {
        self.client
            .post_no_content::<()>("/api/v1/pipeline/cancel", None)
            .await
    }
}
