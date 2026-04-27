//! Document-related types.

use serde::{Deserialize, Serialize};

use super::common::PaginationInfo;

/// Text / JSON document upload (`POST /api/v1/documents`).
#[derive(Debug, Clone, Serialize)]
pub struct UploadDocumentRequest {
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub async_processing: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub track_id: Option<String>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub enable_gleaning: bool,
    pub max_gleaning: usize,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub use_llm_summarization: bool,
}

impl UploadDocumentRequest {
    /// Minimal request with API-default gleaning / summarization flags.
    pub fn text(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            title: None,
            metadata: None,
            async_processing: false,
            track_id: None,
            enable_gleaning: true,
            max_gleaning: 1,
            use_llm_summarization: true,
        }
    }
}

/// Response from uploading a document.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UploadDocumentResponse {
    /// Server returns `document_id`; legacy mocks may use `id`.
    #[serde(rename = "document_id", alias = "id")]
    pub id: String,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub track_id: Option<String>,
    #[serde(default)]
    pub task_id: Option<String>,
    #[serde(default)]
    pub duplicate_of: Option<String>,
    #[serde(default)]
    pub chunk_count: Option<usize>,
    #[serde(default)]
    pub entity_count: Option<usize>,
    #[serde(default)]
    pub relationship_count: Option<usize>,
}

/// Query for `GET /api/v1/documents` (see `ListDocumentsRequest` in edgequake-api).
#[derive(Debug, Clone, Default)]
pub struct DocumentListQuery {
    pub page: Option<usize>,
    pub page_size: Option<usize>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub document_pattern: Option<String>,
}

/// Serialize document list filters to a query string (no leading `?`).
pub fn document_list_query_string(q: &DocumentListQuery) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(p) = q.page {
        parts.push(format!("page={p}"));
    }
    if let Some(ps) = q.page_size {
        parts.push(format!("page_size={ps}"));
    }
    if let Some(ref d) = q.date_from {
        parts.push(format!("date_from={}", urlencoding::encode(d)));
    }
    if let Some(ref d) = q.date_to {
        parts.push(format!("date_to={}", urlencoding::encode(d)));
    }
    if let Some(ref pat) = q.document_pattern {
        parts.push(format!(
            "document_pattern={}",
            urlencoding::encode(pat)
        ));
    }
    parts.join("&")
}

/// Status counts on document list responses (all documents in workspace, not only page).
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct DocumentStatusCounts {
    #[serde(default)]
    pub pending: usize,
    #[serde(default)]
    pub processing: usize,
    #[serde(default)]
    pub completed: usize,
    #[serde(default)]
    pub partial_failure: usize,
    #[serde(default)]
    pub failed: usize,
    #[serde(default)]
    pub cancelled: usize,
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
    pub content_summary: Option<String>,
    #[serde(default)]
    pub content_length: Option<usize>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub file_size: Option<u64>,
    #[serde(default)]
    pub mime_type: Option<String>,
    #[serde(default)]
    pub entity_count: Option<usize>,
    #[serde(default)]
    pub chunk_count: usize,
    #[serde(default)]
    pub error_message: Option<String>,
    #[serde(default)]
    pub track_id: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
    #[serde(default)]
    pub cost_usd: Option<f64>,
    #[serde(default)]
    pub input_tokens: Option<usize>,
    #[serde(default)]
    pub output_tokens: Option<usize>,
    #[serde(default)]
    pub total_tokens: Option<usize>,
    #[serde(default)]
    pub llm_model: Option<String>,
    #[serde(default)]
    pub embedding_model: Option<String>,
    #[serde(default)]
    pub source_type: Option<String>,
    #[serde(default)]
    pub current_stage: Option<String>,
    #[serde(default)]
    pub stage_progress: Option<f64>,
    #[serde(default)]
    pub stage_message: Option<String>,
    #[serde(default)]
    pub pdf_id: Option<String>,
}

/// Response from listing documents.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ListDocumentsResponse {
    #[serde(default)]
    pub documents: Vec<DocumentSummary>,
    #[serde(default)]
    pub total: usize,
    #[serde(default = "default_doc_list_page")]
    pub page: usize,
    #[serde(default = "default_doc_list_page_size")]
    pub page_size: usize,
    #[serde(default)]
    pub total_pages: usize,
    #[serde(default)]
    pub has_more: bool,
    #[serde(default)]
    pub status_counts: DocumentStatusCounts,
    /// Legacy nested pagination from older mocks (ignored when top-level fields present).
    #[serde(default)]
    pub pagination: Option<PaginationInfo>,
}

fn default_doc_list_page() -> usize {
    1
}

fn default_doc_list_page_size() -> usize {
    20
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
        self.pdf_id.as_deref().or(self.id.as_deref())
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
