//! PDF resource.

use crate::client::EdgeQuakeClient;
use crate::error::Result;
use crate::types::documents::*;

pub struct PdfResource<'a> {
    pub(crate) client: &'a EdgeQuakeClient,
}

impl<'a> PdfResource<'a> {
    /// `GET /api/v1/pdf/{id}/progress`
    pub async fn progress(&self, id: &str) -> Result<PdfProgressResponse> {
        self.client
            .get(&format!("/api/v1/pdf/{id}/progress"))
            .await
    }

    /// `GET /api/v1/pdf/{id}/content`
    pub async fn content(&self, id: &str) -> Result<PdfContentResponse> {
        self.client
            .get(&format!("/api/v1/pdf/{id}/content"))
            .await
    }
}
