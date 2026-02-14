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

    /// `GET /api/v1/graph/nodes/{node_id}` — Get a single node by ID.
    pub async fn get_node(&self, node_id: &str) -> Result<serde_json::Value> {
        self.client
            .get(&format!(
                "/api/v1/graph/nodes/{}",
                urlencoding::encode(node_id)
            ))
            .await
    }

    /// `GET /api/v1/graph/labels/search?q=…` — Search labels.
    pub async fn search_labels(&self, query: &str) -> Result<Vec<serde_json::Value>> {
        self.client
            .get(&format!(
                "/api/v1/graph/labels/search?q={}",
                urlencoding::encode(query)
            ))
            .await
    }

    /// `GET /api/v1/graph/labels/popular` — Get popular labels.
    pub async fn popular_labels(&self) -> Result<Vec<serde_json::Value>> {
        self.client.get("/api/v1/graph/labels/popular").await
    }
}
