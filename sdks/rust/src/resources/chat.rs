//! Chat resource.

use bytes::Bytes;
use futures_core::Stream;
use futures_util::StreamExt;

use crate::client::EdgeQuakeClient;
use crate::error::{Error, Result};
use crate::types::chat::*;

pub struct ChatResource<'a> {
    pub(crate) client: &'a EdgeQuakeClient,
}

impl<'a> ChatResource<'a> {
    /// `POST /api/v1/chat/completions`
    pub async fn completions(&self, req: &ChatCompletionRequest) -> Result<ChatCompletionResponse> {
        self.client
            .post("/api/v1/chat/completions", Some(req))
            .await
    }

    /// `POST /api/v1/chat/completions/stream`
    pub async fn stream_completions(
        &self,
        req: &ChatCompletionRequest,
    ) -> Result<impl Stream<Item = std::result::Result<Bytes, Error>> + Send + Unpin> {
        let resp = self
            .client
            .post_raw("/api/v1/chat/completions/stream", req)
            .await?;
        if !resp.status().is_success() {
            return Err(Error::from_response(resp).await);
        }
        Ok(resp.bytes_stream().map(|r| r.map_err(Error::Network)))
    }
}
