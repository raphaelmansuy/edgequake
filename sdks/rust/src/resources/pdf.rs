//! PDF resource.
//!
//! WHY: PDF endpoints live under /api/v1/documents/pdf/ per routes.rs.

use crate::client::EdgeQuakeClient;
use crate::error::Result;
use crate::types::documents::*;

pub struct PdfResource<'a> {
    pub(crate) client: &'a EdgeQuakeClient,
}

impl<'a> PdfResource<'a> {
    /// `GET /api/v1/documents/pdf/progress/{track_id}`
    pub async fn progress(&self, track_id: &str) -> Result<PdfProgressResponse> {
        self.client
            .get(&format!("/api/v1/documents/pdf/progress/{track_id}"))
            .await
    }

    /// `GET /api/v1/documents/pdf/{pdf_id}/content`
    pub async fn content(&self, pdf_id: &str) -> Result<PdfContentResponse> {
        self.client
            .get(&format!("/api/v1/documents/pdf/{pdf_id}/content"))
            .await
    }
}
