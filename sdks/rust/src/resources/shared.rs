//! Public shared conversation (`GET /api/v1/shared/{share_id}`).

use crate::client::EdgeQuakeClient;
use crate::error::Result;
use crate::types::conversations::ConversationDetail;

pub struct SharedResource<'a> {
    pub(crate) client: &'a EdgeQuakeClient,
}

impl<'a> SharedResource<'a> {
    /// `GET /api/v1/shared/{share_id}` — same body shape as `GET /conversations/{id}`.
    pub async fn get(&self, share_id: &str) -> Result<ConversationDetail> {
        self.client
            .get(&format!("/api/v1/shared/{share_id}"))
            .await
    }
}
