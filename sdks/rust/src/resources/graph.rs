//! Graph resource.

use crate::client::EdgeQuakeClient;
use crate::error::Result;
use crate::types::graph::*;

pub struct GraphResource<'a> {
    pub(crate) client: &'a EdgeQuakeClient,
}

impl<'a> GraphResource<'a> {
    /// `GET /api/v1/graph` — full graph.
    pub async fn get(&self) -> Result<GraphResponse> {
        self.client.get("/api/v1/graph").await
    }

    /// `GET /api/v1/graph/nodes/search?q=…`
    pub async fn search(&self, query: &str) -> Result<SearchNodesResponse> {
        self.client
            .get(&format!(
                "/api/v1/graph/nodes/search?q={}",
                urlencoding::encode(query)
            ))
            .await
    }
}
