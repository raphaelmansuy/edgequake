//! PDF resource.
//!
//! WHY: PDF endpoints live under /api/v1/documents/pdf/ per routes.rs.

use std::collections::HashMap;

use crate::client::EdgeQuakeClient;
use crate::error::Result;
use crate::types::documents::*;

pub struct PdfResource<'a> {
    pub(crate) client: &'a EdgeQuakeClient,
}

impl<'a> PdfResource<'a> {
    /// Upload a PDF file for extraction.
    ///
    /// POST /api/v1/documents/pdf
    ///
    /// ```no_run
    /// # use edgequake_sdk::{EdgeQuakeClient, types::documents::PdfUploadOptions};
    /// # async fn example(client: &EdgeQuakeClient) -> edgequake_sdk::error::Result<()> {
    /// let bytes = std::fs::read("report.pdf").expect("failed to read PDF file");
    /// let resp = client.pdf().upload(bytes, "report.pdf", PdfUploadOptions {
    ///     enable_vision: true,
    ///     vision_model: Some("gpt-4o".into()),
    ///     ..Default::default()
    /// }).await?;
    /// println!("PDF ID: {:?}", resp.canonical_id());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn upload(
        &self,
        file_bytes: Vec<u8>,
        filename: &str,
        options: PdfUploadOptions,
    ) -> Result<PdfUploadResponse> {
        let mut fields: HashMap<String, String> = HashMap::new();
        if options.enable_vision {
            fields.insert("enable_vision".into(), "true".into());
        }
        if let Some(vp) = options.vision_provider {
            fields.insert("vision_provider".into(), vp);
        }
        if let Some(vm) = options.vision_model {
            fields.insert("vision_model".into(), vm);
        }
        if let Some(title) = options.title {
            fields.insert("title".into(), title);
        }
        if let Some(tid) = options.track_id {
            fields.insert("track_id".into(), tid);
        }
        if options.force_reindex {
            fields.insert("force_reindex".into(), "true".into());
        }

        self.client
            .upload_multipart(
                "/api/v1/documents/pdf",
                file_bytes,
                filename,
                "application/pdf",
                fields,
            )
            .await
    }

    /// List all uploaded PDFs.
    ///
    /// GET /api/v1/documents/pdf
    pub async fn list(&self) -> Result<Vec<PdfInfo>> {
        self.client.get("/api/v1/documents/pdf").await
    }

    /// Get PDF processing status by ID.
    ///
    /// GET /api/v1/documents/pdf/{pdf_id}
    pub async fn get(&self, pdf_id: &str) -> Result<PdfInfo> {
        self.client
            .get(&format!("/api/v1/documents/pdf/{pdf_id}"))
            .await
    }

    /// Get PDF extraction progress by track ID.
    ///
    /// GET /api/v1/documents/pdf/progress/{track_id}
    pub async fn progress(&self, track_id: &str) -> Result<PdfProgressResponse> {
        self.client
            .get(&format!("/api/v1/documents/pdf/progress/{track_id}"))
            .await
    }

    /// Get extracted PDF content (Markdown).
    ///
    /// GET /api/v1/documents/pdf/{pdf_id}/content
    pub async fn content(&self, pdf_id: &str) -> Result<PdfContentResponse> {
        self.client
            .get(&format!("/api/v1/documents/pdf/{pdf_id}/content"))
            .await
    }

    /// `GET /api/v1/documents/pdf/{pdf_id}/download` — raw PDF bytes.
    pub async fn download(&self, pdf_id: &str) -> Result<Vec<u8>> {
        self.client
            .get_raw(&format!("/api/v1/documents/pdf/{pdf_id}/download"))
            .await
    }

    /// `POST /api/v1/documents/pdf/{pdf_id}/retry`
    pub async fn retry(&self, pdf_id: &str) -> Result<()> {
        self.client
            .post_no_content::<()>(&format!("/api/v1/documents/pdf/{pdf_id}/retry"), None)
            .await
    }

    /// `DELETE /api/v1/documents/pdf/{pdf_id}/cancel`
    pub async fn cancel(&self, pdf_id: &str) -> Result<()> {
        self.client
            .delete_no_content(&format!("/api/v1/documents/pdf/{pdf_id}/cancel"))
            .await
    }

    /// `DELETE /api/v1/documents/pdf/{pdf_id}`
    pub async fn delete(&self, pdf_id: &str) -> Result<()> {
        self.client
            .delete_no_content(&format!("/api/v1/documents/pdf/{pdf_id}"))
            .await
    }
}
