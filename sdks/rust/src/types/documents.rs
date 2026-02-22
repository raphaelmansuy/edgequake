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

/// Options for PDF upload (v0.4.0+).
///
/// Vision pipeline renders each page to an image and passes it to a
/// multimodal LLM for high-fidelity Markdown extraction.
#[derive(Debug, Clone, Default, Serialize)]
pub struct PdfUploadOptions {
    /// Enable LLM vision pipeline for high-fidelity extraction.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub enable_vision: bool,
    /// Override vision provider (e.g. "openai", "ollama").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vision_provider: Option<String>,
    /// Override vision model (e.g. "gpt-4o", "gemma3").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vision_model: Option<String>,
    /// Human-readable title.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Batch track ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub track_id: Option<String>,
    /// Re-process even if document already exists.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub force_reindex: bool,
}

/// PDF upload response.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PdfUploadResponse {
    /// Primary identifier returned by the API (v0.4.0+).
    #[serde(default)]
    pub pdf_id: Option<String>,
    /// Legacy field — same as pdf_id on older servers.
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub document_id: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub track_id: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub estimated_time_seconds: Option<u64>,
    #[serde(default)]
    pub duplicate_of: Option<String>,
}

impl PdfUploadResponse {
    /// Return the canonical PDF ID regardless of which field the server used.
    pub fn canonical_id(&self) -> Option<&str> {
        self.pdf_id.as_deref().or_else(|| self.id.as_deref())
    }
}

/// PDF document metadata returned by list/get endpoints.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PdfInfo {
    pub pdf_id: String,
    #[serde(default)]
    pub document_id: Option<String>,
    /// Original filename uploaded.
    #[serde(default, alias = "file_name")]
    pub filename: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub file_size: Option<u64>,
    #[serde(default)]
    pub page_count: Option<u32>,
    /// Extraction method: "vision", "text", or "ocr" (v0.4.0+).
    #[serde(default)]
    pub extraction_method: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
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
