//! Query resource.

use crate::client::EdgeQuakeClient;
use crate::error::Result;
use crate::types::query::*;

pub struct QueryResource<'a> {
    pub(crate) client: &'a EdgeQuakeClient,
}

impl<'a> QueryResource<'a> {
    /// `POST /api/v1/query`
    pub async fn execute(&self, req: &QueryRequest) -> Result<QueryResponse> {
        self.client.post("/api/v1/query", Some(req)).await
    }
}
