use edgequake_core::Workspace;
use edgequake_pdf::PdfParserBackend;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Re-export progress DTOs from edgequake-tasks (SSOT for PDF pipeline progress).
pub use edgequake_tasks::{
    PdfUploadProgress, PhaseError, PhaseProgress, PhaseStatus, PipelinePhase,
};

/// PDF upload options.
#[derive(Debug, Clone, Default)]
pub struct PdfUploadOptions {
    /// Enable vision LLM processing (default: true).
    pub enable_vision: bool,
    /// Vision provider to use. None = use workspace config then server default.
    /// Explicitly set by form field `vision_provider`.
    pub vision_provider: Option<String>,
    /// Vision model override. None = use workspace config then provider default.
    /// Explicitly set by form field `vision_model`.
    pub vision_model: Option<String>,
    /// Document title (optional).
    pub title: Option<String>,
    /// Custom metadata (optional).
    pub metadata: Option<serde_json::Value>,
    /// Batch tracking ID (optional).
    pub track_id: Option<String>,
    /// Force re-indexing of duplicate PDF (default: false).
    /// WHY (OODA-08): When true, existing graph/vector data is cleared
    /// and the document is re-processed with current LLM/config.
    pub force_reindex: bool,
    /// Explicit parser backend override for this upload.
    pub pdf_parser_backend: Option<PdfParserBackend>,
    /// Multimodal process options (LightRAG `i`/`t`/`e` flags), e.g. `"i"` or `"ite"`.
    pub process_options: Option<String>,
}

impl PdfUploadOptions {
    /// Get the resolved vision provider (with fallback to server default).
    ///
    /// WHY (First Principle): Single resolution chain with explicit priority:
    ///   1. Explicit form field `vision_provider`
    ///   2. EDGEQUAKE_VISION_PROVIDER / EDGEQUAKE_VISION_LLM_PROVIDER env
    ///   3. EDGEQUAKE_DEFAULT_LLM_PROVIDER env (inherit from LLM)
    ///   4. EDGEQUAKE_LLM_PROVIDER env (legacy alias)
    ///   5. Hardcoded fallback: "ollama"
    pub fn resolved_vision_provider(&self) -> String {
        self.vision_provider
            .clone()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(crate::vision_env::resolved_vision_provider_from_env)
    }

    /// Get the vision model to use (with fallback from provider).
    pub fn vision_model(&self) -> String {
        self.vision_model
            .clone()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| {
                crate::vision_env::default_vision_model_for_provider(
                    &self.resolved_vision_provider(),
                )
            })
    }

    /// Resolve the effective PDF parser backend.
    pub fn resolved_backend(&self, workspace: Option<&Workspace>) -> PdfParserBackend {
        self.pdf_parser_backend
            .or_else(|| workspace.and_then(|ws| ws.pdf_parser_backend))
            .or_else(PdfParserBackend::from_env)
            .unwrap_or_default()
    }
}

/// PDF upload response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PdfUploadResponse {
    /// Generated PDF ID.
    pub pdf_id: String,

    /// Associated document ID (null during processing).
    pub document_id: Option<String>,

    /// Processing status.
    pub status: String,

    /// Background task ID.
    pub task_id: String,

    /// Batch tracking ID (if provided).
    pub track_id: Option<String>,

    /// Human-readable message.
    pub message: String,

    /// Estimated processing time in seconds.
    pub estimated_time_seconds: u64,

    /// SPEC-038: Detailed ingestion time breakdown.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ingestion_estimate: Option<crate::services::IngestionEstimate>,

    /// PDF metadata.
    pub metadata: PdfMetadata,

    /// ID of the existing duplicate PDF, present when status is "duplicate".
    /// WHY: Frontend uses this field to show the DuplicateUploadDialog and
    /// offer the user a choice to reprocess or skip the duplicate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duplicate_of: Option<String>,
}

/// Batch PDF upload response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PdfBatchUploadResponse {
    /// Total files received in the multipart request.
    pub total_files: usize,
    /// Number of files accepted for processing.
    pub accepted: usize,
    /// Number of duplicate files.
    pub duplicates: usize,
    /// Number of files that failed validation or processing setup.
    pub failed: usize,
    /// Per-file result details.
    pub results: Vec<PdfBatchFileResult>,
}

/// Result for a single file in batch PDF upload.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PdfBatchFileResult {
    /// Original filename.
    pub filename: String,
    /// Upload result status: `processing`, `duplicate`, `reindexing`, or `failed`.
    pub status: String,
    /// PDF ID when available.
    pub pdf_id: Option<String>,
    /// Processing task ID when available.
    pub task_id: Option<String>,
    /// Duplicate PDF ID if applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duplicate_of: Option<String>,
    /// Error text for failed items.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// PDF metadata in response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PdfMetadata {
    /// Original filename.
    pub filename: String,

    /// File size in bytes.
    pub file_size_bytes: i64,

    /// Number of pages (if detected).
    pub page_count: Option<i32>,

    /// SHA-256 checksum.
    pub sha256_checksum: String,

    /// Vision enabled flag.
    pub vision_enabled: bool,

    /// Vision model to use.
    pub vision_model: Option<String>,
}

/// PDF status response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PdfStatusResponse {
    /// PDF ID.
    pub pdf_id: String,

    /// Associated document ID (if completed).
    pub document_id: Option<String>,

    /// Processing status.
    pub status: String,

    /// Processing duration in milliseconds (if completed).
    pub processing_duration_ms: Option<i64>,

    /// PDF metadata.
    pub metadata: PdfStatusMetadata,

    /// Extraction errors (if any).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<serde_json::Value>,
}

/// PDF status metadata.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PdfStatusMetadata {
    /// Original filename.
    pub filename: String,

    /// Number of pages.
    pub page_count: Option<i32>,

    /// Extraction method used (if completed).
    pub extraction_method: Option<String>,

    /// Vision model used (if applicable).
    pub vision_model: Option<String>,

    /// When processing completed.
    pub processed_at: Option<String>,
}

/// PDF list query parameters.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ListPdfsQuery {
    /// Filter by status.
    #[serde(default)]
    pub status: Option<String>,

    /// Page number (1-indexed).
    #[serde(default = "default_page")]
    pub page: usize,

    /// Page size.
    #[serde(default = "default_page_size")]
    pub page_size: usize,
}

fn default_page() -> usize {
    1
}

fn default_page_size() -> usize {
    20
}

/// PDF list response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ListPdfsResponse {
    /// PDF items.
    pub items: Vec<PdfListItem>,

    /// Pagination info.
    pub pagination: PdfPaginationInfo,
}

/// PDF list item.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PdfListItem {
    /// PDF ID.
    pub pdf_id: String,

    /// Original filename.
    pub filename: String,

    /// Processing status.
    pub status: String,

    /// File size in bytes.
    pub file_size_bytes: i64,

    /// Number of pages.
    pub page_count: Option<i32>,

    /// When uploaded.
    pub created_at: String,

    /// When processed.
    pub processed_at: Option<String>,
}

/// Pagination information for PDF listing.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PdfPaginationInfo {
    /// Current page (1-indexed).
    pub page: usize,

    /// Page size.
    pub page_size: usize,

    /// Total item count.
    pub total_count: i64,

    /// Total pages.
    pub total_pages: usize,
}
