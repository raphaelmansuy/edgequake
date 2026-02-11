//! Chat resource.

use crate::client::EdgeQuakeClient;
use crate::error::Result;
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
}
