//! Document-related types.

use serde::{Deserialize, Serialize};

use super::common::PaginationInfo;

/// Response from uploading a document.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UploadDocumentResponse {
    pub id: String,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub track_id: Option<String>,
}

/// Document summary in list responses.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DocumentSummary {
    pub id: String,
    #[serde(default)]
    pub file_name: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub file_size: Option<u64>,
    #[serde(default)]
    pub mime_type: Option<String>,
    #[serde(default)]
    pub entity_count: Option<u32>,
    #[serde(default)]
    pub chunk_count: Option<u32>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

/// Response from listing documents.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ListDocumentsResponse {
    #[serde(default)]
    pub documents: Vec<DocumentSummary>,
    #[serde(default)]
    pub pagination: Option<PaginationInfo>,
}

/// Response from tracking document processing status.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TrackStatusResponse {
    pub track_id: String,
    pub status: String,
    #[serde(default)]
    pub progress: Option<f64>,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub document_id: Option<String>,
}

/// Response from directory scanning.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ScanResponse {
    #[serde(default)]
    pub files_found: u32,
    #[serde(default)]
    pub files_queued: u32,
    #[serde(default)]
    pub files_skipped: u32,
}

/// Response from deletion impact analysis.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DeletionImpactResponse {
    #[serde(default)]
    pub entity_count: u32,
    #[serde(default)]
    pub relationship_count: u32,
    #[serde(default)]
    pub chunk_count: u32,
}

/// PDF upload response.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PdfUploadResponse {
    pub id: String,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub track_id: Option<String>,
}

/// PDF progress response.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PdfProgressResponse {
    pub track_id: String,
    pub status: String,
    #[serde(default)]
    pub progress: Option<f64>,
}

/// PDF content response.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PdfContentResponse {
    pub id: String,
    #[serde(default)]
    pub markdown: Option<String>,
}

/// Scan request parameters.
#[derive(Debug, Clone, Serialize)]
pub struct ScanRequest {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recursive: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<Vec<String>>,
}
