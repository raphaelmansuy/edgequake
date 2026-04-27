//! Query resource.

use bytes::Bytes;
use futures_core::Stream;
use futures_util::StreamExt;

use crate::client::EdgeQuakeClient;
use crate::error::{Error, Result};
use crate::types::query::*;

pub struct QueryResource<'a> {
    pub(crate) client: &'a EdgeQuakeClient,
}

impl<'a> QueryResource<'a> {
    /// `POST /api/v1/query`
    pub async fn execute(&self, req: &QueryRequest) -> Result<QueryResponse> {
        self.client.post("/api/v1/query", Some(req)).await
    }

    /// `POST /api/v1/query/stream` — SSE / chunked response bytes.
    pub async fn stream_execute(
        &self,
        req: &QueryRequest,
    ) -> Result<impl Stream<Item = std::result::Result<Bytes, Error>> + Send + Unpin> {
        let resp = self.client.post_raw("/api/v1/query/stream", req).await?;
        if !resp.status().is_success() {
            return Err(Error::from_response(resp).await);
        }
        Ok(resp.bytes_stream().map(|r| r.map_err(Error::Network)))
    }
}
