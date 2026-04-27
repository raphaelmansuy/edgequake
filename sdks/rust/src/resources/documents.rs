//! Documents resource.

use crate::client::EdgeQuakeClient;
use crate::error::Result;
use crate::types::documents::*;
use crate::types::operations::DocumentFullLineage;

pub struct DocumentsResource<'a> {
    pub(crate) client: &'a EdgeQuakeClient,
}

impl<'a> DocumentsResource<'a> {
    /// `GET /api/v1/documents` with server default pagination and filters.
    pub async fn list(&self) -> Result<ListDocumentsResponse> {
        self.list_with_query(&DocumentListQuery::default()).await
    }

    /// `GET /api/v1/documents` with `page`, `page_size`, `date_from`, `date_to`, `document_pattern`.
    pub async fn list_with_query(&self, q: &DocumentListQuery) -> Result<ListDocumentsResponse> {
        let qs = crate::types::documents::document_list_query_string(q);
        let path = if qs.is_empty() {
            "/api/v1/documents".to_string()
        } else {
            format!("/api/v1/documents?{qs}")
        };
        self.client.get(&path).await
    }

    /// `GET /api/v1/documents/{id}`
    pub async fn get(&self, id: &str) -> Result<DocumentSummary> {
        self.client.get(&format!("/api/v1/documents/{id}")).await
    }

    /// `DELETE /api/v1/documents/{id}`
    pub async fn delete(&self, id: &str) -> Result<()> {
        self.client
            .delete_no_content(&format!("/api/v1/documents/{id}"))
            .await
    }

    /// `POST /api/v1/documents` — upload text/JSON body for ingestion.
    pub async fn upload(&self, req: &UploadDocumentRequest) -> Result<UploadDocumentResponse> {
        self.client.post("/api/v1/documents", Some(req)).await
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

    // ========================================================================
    // Lineage Methods (OODA-14)
    // ========================================================================

    /// `GET /api/v1/documents/{id}/lineage`
    ///
    /// Returns complete document lineage (persisted pipeline lineage + metadata).
    /// @implements F5 — Single API call retrieves complete lineage tree.
    pub async fn get_lineage(&self, id: &str) -> Result<DocumentFullLineage> {
        self.client
            .get(&format!("/api/v1/documents/{id}/lineage"))
            .await
    }

    /// `GET /api/v1/documents/{id}/metadata`
    ///
    /// Returns all document metadata stored in KV storage.
    /// @implements F1 — All document metadata retrievable.
    pub async fn get_metadata(&self, id: &str) -> Result<serde_json::Value> {
        self.client
            .get(&format!("/api/v1/documents/{id}/metadata"))
            .await
    }

    // ========================================================================
    // Bulk / Recovery Operations (OODA-32)
    // ========================================================================

    /// `DELETE /api/v1/documents` — Delete all documents in workspace.
    pub async fn delete_all(&self) -> Result<serde_json::Value> {
        self.client.delete("/api/v1/documents").await
    }

    /// `POST /api/v1/documents/reprocess` — Retry all failed documents.
    pub async fn reprocess(&self) -> Result<serde_json::Value> {
        self.client
            .post::<(), serde_json::Value>("/api/v1/documents/reprocess", None)
            .await
    }

    /// `POST /api/v1/documents/recover-stuck` — Recover stuck processing docs.
    pub async fn recover_stuck(&self) -> Result<serde_json::Value> {
        self.client
            .post::<(), serde_json::Value>("/api/v1/documents/recover-stuck", None)
            .await
    }

    /// `POST /api/v1/documents/{id}/retry-chunks` — Retry failed chunks for a document.
    pub async fn retry_chunks(&self, id: &str) -> Result<serde_json::Value> {
        self.client
            .post::<(), serde_json::Value>(&format!("/api/v1/documents/{id}/retry-chunks"), None)
            .await
    }

    /// `GET /api/v1/documents/{id}/failed-chunks` — List failed chunks for a document.
    pub async fn failed_chunks(&self, id: &str) -> Result<Vec<serde_json::Value>> {
        self.client
            .get(&format!("/api/v1/documents/{id}/failed-chunks"))
            .await
    }
}
