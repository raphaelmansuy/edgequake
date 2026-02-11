//! Documents resource.

use crate::client::EdgeQuakeClient;
use crate::error::Result;
use crate::types::documents::*;

pub struct DocumentsResource<'a> {
    pub(crate) client: &'a EdgeQuakeClient,
}

impl<'a> DocumentsResource<'a> {
    /// `GET /api/v1/documents`
    pub async fn list(&self) -> Result<ListDocumentsResponse> {
        self.client.get("/api/v1/documents").await
    }

    /// `GET /api/v1/documents/{id}`
    pub async fn get(&self, id: &str) -> Result<DocumentSummary> {
        self.client.get(&format!("/api/v1/documents/{id}")).await
    }

    /// `DELETE /api/v1/documents/{id}`
    pub async fn delete(&self, id: &str) -> Result<()> {
        self.client.delete_no_content(&format!("/api/v1/documents/{id}")).await
    }

    /// `GET /api/v1/documents/{id}/status`
    pub async fn status(&self, id: &str) -> Result<TrackStatusResponse> {
        self.client
            .get(&format!("/api/v1/documents/{id}/status"))
            .await
    }

    /// `POST /api/v1/documents/upload/text`
    pub async fn upload_text(&self, body: &serde_json::Value) -> Result<UploadDocumentResponse> {
        self.client
            .post("/api/v1/documents/upload/text", Some(body))
            .await
    }

    /// `POST /api/v1/documents/scan`
    pub async fn scan(&self, req: &ScanRequest) -> Result<ScanResponse> {
        self.client.post("/api/v1/documents/scan", Some(req)).await
    }

    /// `GET /api/v1/documents/{id}/deletion-impact`
    pub async fn deletion_impact(&self, id: &str) -> Result<DeletionImpactResponse> {
        self.client
            .get(&format!("/api/v1/documents/{id}/deletion-impact"))
            .await
    }

    /// `GET /api/v1/documents/track/{track_id}`
    pub async fn track(&self, track_id: &str) -> Result<TrackStatusResponse> {
        self.client
            .get(&format!("/api/v1/documents/track/{track_id}"))
            .await
    }
}
