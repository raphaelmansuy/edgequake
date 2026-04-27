use edgequake_core::Workspace;
use edgequake_pdf::PdfParserBackend;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

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
        // WHY filter empty strings: Docker Compose ${VAR:-} → "" issue.
        self.vision_provider
            .clone()
            .filter(|s| !s.is_empty())
            .or_else(|| non_empty_env("EDGEQUAKE_VISION_PROVIDER"))
            .or_else(|| non_empty_env("EDGEQUAKE_VISION_LLM_PROVIDER"))
            .or_else(|| non_empty_env("EDGEQUAKE_DEFAULT_LLM_PROVIDER"))
            .or_else(|| non_empty_env("EDGEQUAKE_LLM_PROVIDER"))
            .unwrap_or_else(|| "ollama".to_string())
    }

    /// Get the vision model to use (with fallback from provider).
    ///
    /// WHY filter empty strings: if workspace stored an empty model string,
    /// treat it the same as "not configured" and fall back to the provider default.
    pub fn vision_model(&self) -> String {
        self.vision_model
            .clone()
            .filter(|s| !s.is_empty()) // treat "" same as None
            .unwrap_or_else(|| default_vision_model_for_provider(&self.resolved_vision_provider()))
    }

    /// Resolve the effective PDF parser backend.
    pub fn resolved_backend(&self, workspace: Option<&Workspace>) -> PdfParserBackend {
        self.pdf_parser_backend
            .or_else(|| workspace.and_then(|ws| ws.pdf_parser_backend))
            .or_else(PdfParserBackend::from_env)
            .unwrap_or_default()
    }
}

/// Read an env var, treating empty strings as None.
/// WHY: Docker Compose `${VAR:-}` maps unset host vars to "" inside containers.
fn non_empty_env(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|s| !s.is_empty())
}

/// Return a sensible default vision model for the given provider.
///
/// WHY (First Principle): The model MUST be compatible with the resolved provider.
/// Previous code returned `EDGEQUAKE_VISION_MODEL` for ALL providers, which caused
/// gpt-4.1-nano to be sent to Ollama when a stale env var leaked in.
///
/// Resolution chain (same priority for all providers):
///   1. EDGEQUAKE_VISION_MODEL env (only if compatible with provider)
///   2. EDGEQUAKE_VISION_LLM_MODEL env (only if compatible with provider)
///   3. EDGEQUAKE_DEFAULT_LLM_MODEL env (only if compatible with provider)
///   4. EDGEQUAKE_LLM_MODEL env (legacy alias, only if compatible)
///   5. Provider-specific hardcoded default
pub(crate) fn default_vision_model_for_provider(provider: &str) -> String {
    use crate::safety_limits::is_model_provider_mismatch;

    let candidates = [
        non_empty_env("EDGEQUAKE_VISION_MODEL"),
        non_empty_env("EDGEQUAKE_VISION_LLM_MODEL"),
        non_empty_env("EDGEQUAKE_DEFAULT_LLM_MODEL"),
        non_empty_env("EDGEQUAKE_LLM_MODEL"),
    ];

    // Pick the first candidate that is compatible with the provider.
    for candidate in candidates.into_iter().flatten() {
        if !is_model_provider_mismatch(provider, &candidate) {
            return candidate;
        }
        tracing::warn!(
            provider,
            model = %candidate,
            "Skipping incompatible vision model from env — model '{}' cannot run on provider '{}'",
            candidate,
            provider,
        );
    }

    // Hardcoded provider-specific defaults (always compatible).
    match provider {
        "openai" => "gpt-4.1-nano".to_string(),
        "anthropic" => "claude-sonnet-4-20250514".to_string(),
        // Pixtral Large is Mistral's flagship vision model for PDF ingestion.
        "mistral" => "pixtral-large-latest".to_string(),
        _ => "gemma4:latest".to_string(),
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

    /// PDF metadata.
    pub metadata: PdfMetadata,

    /// ID of the existing duplicate PDF, present when status is "duplicate".
    /// WHY: Frontend uses this field to show the DuplicateUploadDialog and
    /// offer the user a choice to reprocess or skip the duplicate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duplicate_of: Option<String>,
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
