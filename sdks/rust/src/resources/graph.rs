//! Graph resource.

use bytes::Bytes;
use futures_core::Stream;
use futures_util::StreamExt;

use crate::client::EdgeQuakeClient;
use crate::error::{Error, Result};
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

    /// `POST /api/v1/graph/degrees/batch` — bulk node degree lookup.
    pub async fn degrees_batch(&self, node_ids: &[String]) -> Result<DegreesBatchResponse> {
        let body = BatchDegreeRequest {
            node_ids: node_ids.to_vec(),
        };
        self.client
            .post("/api/v1/graph/degrees/batch", Some(&body))
            .await
    }

    /// `GET /api/v1/graph/stream` — SSE byte stream (caller parses events).
    ///
    /// Optional `query` is the raw query string **without** `?` (e.g. `limit=100`).
    pub async fn stream(
        &self,
        query: Option<&str>,
    ) -> Result<impl Stream<Item = std::result::Result<Bytes, Error>> + Send + Unpin> {
        let path = match query {
            Some(q) if !q.is_empty() => format!("/api/v1/graph/stream?{}", q),
            _ => "/api/v1/graph/stream".to_string(),
        };
        let resp = self.client.get_raw_response(&path).await?;
        if !resp.status().is_success() {
            return Err(Error::from_response(resp).await);
        }
        Ok(resp.bytes_stream().map(|r| r.map_err(Error::Network)))
    }
}
