use edgequake_core::{Tenant, Workspace};
use edgequake_pdf::{resolve_pdf_parser_choice, PdfParserBackend, ResolvedPdfParser};
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
    /// Optional client batch/request correlation ID (SPEC-054 / #300).
    ///
    /// NOT the progress-store key — that is always the server `task_id`
    /// (`pdf-<uuid>`). Shared across multi-file WebUI batches.
    pub track_id: Option<String>,
    /// Force re-indexing of duplicate PDF (default: false).
    /// WHY (OODA-08): When true, existing graph/vector data is cleared
    /// and the document is re-processed with current LLM/config.
    pub force_reindex: bool,
    /// Explicit parser backend override for this upload.
    pub pdf_parser_backend: Option<PdfParserBackend>,
    /// Multimodal process options (LightRAG `i`/`t`/`e` flags), e.g. `"i"` or `"ite"`.
    pub process_options: Option<String>,
    /// SPEC-109: vision/VLM reasoning effort override for this upload.
    pub vision_reasoning_effort: Option<String>,
    /// SPEC-015V: sparse vision extract overlay (None fields inherit workspace).
    pub vision_extract: edgequake_pdf::VisionExtractOverlay,
}

impl PdfUploadOptions {
    /// SPEC-123: vision LLM is **not** copied into upload fields —
    /// use [`Self::resolved_vision_llm`] (Upload → Workspace → Tenant → Env).
    ///
    /// Kept as a no-op-compatible hook for call sites that still invoke it
    /// after loading workspace (parser already uses SSOT; vision now does too).
    pub fn apply_workspace(&mut self, _workspace: &Workspace) {
        // Intentionally empty — do not mutate upload fields from workspace
        // (destroys provenance; LAW-123-5).
    }

    /// Resolve SPEC-015V extract policy (upload overlay over workspace metadata).
    pub fn resolved_vision_extract(
        &self,
        workspace: Option<&Workspace>,
    ) -> Result<edgequake_pdf::VisionExtractConfig, String> {
        let empty = std::collections::HashMap::new();
        let meta = workspace.map(|w| &w.metadata).unwrap_or(&empty);
        edgequake_pdf::VisionExtractConfig::resolve(meta, &self.vision_extract)
    }

    /// SPEC-123 SSOT: Upload → Workspace vision_* → Tenant vision → Workspace LLM → Env.
    pub fn resolved_vision_llm(
        &self,
        workspace: Option<&Workspace>,
        tenant: Option<&Tenant>,
    ) -> edgequake_core::ResolvedProviderModel {
        edgequake_core::resolve_vision_llm_choice(
            self.vision_provider.as_deref(),
            self.vision_model.as_deref(),
            workspace,
            tenant,
        )
    }

    /// Get the resolved vision provider (with fallback to server default).
    pub fn resolved_vision_provider(
        &self,
        workspace: Option<&Workspace>,
        tenant: Option<&Tenant>,
    ) -> String {
        self.resolved_vision_llm(workspace, tenant).provider
    }

    /// Get the vision model to use (with fallback from provider).
    pub fn vision_model(&self, workspace: Option<&Workspace>, tenant: Option<&Tenant>) -> String {
        self.resolved_vision_llm(workspace, tenant).model
    }

    /// SPEC-123 SSOT: Upload → Workspace → Tenant → Env → Vision.
    pub fn resolved_pdf_parser(
        &self,
        workspace: Option<&Workspace>,
        tenant: Option<&Tenant>,
    ) -> ResolvedPdfParser {
        resolve_pdf_parser_choice(
            self.pdf_parser_backend,
            workspace.and_then(|ws| ws.pdf_parser_backend),
            tenant.and_then(|t| t.pdf_parser_backend),
            PdfParserBackend::from_env(),
        )
    }

    /// Resolve the effective runtime PDF parser backend (Vision|EdgeParse).
    pub fn resolved_backend(
        &self,
        workspace: Option<&Workspace>,
        tenant: Option<&Tenant>,
    ) -> PdfParserBackend {
        self.resolved_pdf_parser(workspace, tenant).runtime_backend
    }

    /// Resolve multimodal process flags for this upload.
    ///
    /// First principle (SPEC-047 FP1): figure/chart assets extracted during Vision
    /// PDF conversion must receive Pass B VLM analysis or their semantics never
    /// land in indexable markdown. When the client omits `process_options`, default
    /// to `"i"` for vision-enabled uploads on the Vision backend.
    pub fn resolved_process_options(
        &self,
        workspace: Option<&Workspace>,
        tenant: Option<&Tenant>,
    ) -> Option<String> {
        if let Some(opts) = self
            .process_options
            .as_ref()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
        {
            return Some(opts.to_string());
        }
        if self.enable_vision
            && self.resolved_backend(workspace, tenant) == PdfParserBackend::Vision
        {
            Some("i".to_string())
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn resolved_process_options_defaults_i_for_vision_upload() {
        let opts = PdfUploadOptions {
            enable_vision: true,
            ..Default::default()
        };
        assert_eq!(
            opts.resolved_process_options(None, None).as_deref(),
            Some("i")
        );
    }

    #[test]
    fn resolved_process_options_respects_explicit_override() {
        let opts = PdfUploadOptions {
            enable_vision: true,
            process_options: Some("te".into()),
            ..Default::default()
        };
        assert_eq!(
            opts.resolved_process_options(None, None).as_deref(),
            Some("te")
        );
    }

    #[test]
    fn resolved_process_options_skipped_when_vision_disabled() {
        let opts = PdfUploadOptions {
            enable_vision: false,
            ..Default::default()
        };
        assert!(opts.resolved_process_options(None, None).is_none());
    }

    #[test]
    fn apply_workspace_does_not_mutate_vision_fields() {
        let mut ws = Workspace::new(Uuid::nil(), "ws", "ws");
        ws.llm_provider = "mistral".into();
        ws.vision_llm_provider = Some("mistral".into());
        ws.vision_llm_model = Some("mistral-small-latest".into());
        ws.pdf_parser_backend = Some(PdfParserBackend::Vision);

        let mut opts = PdfUploadOptions::default();
        opts.apply_workspace(&ws);

        // SPEC-123: upload fields stay unset; resolve reads workspace/tenant layers.
        assert_eq!(opts.vision_provider, None);
        assert_eq!(opts.vision_model, None);
        assert_eq!(opts.pdf_parser_backend, None);
        let vision = opts.resolved_vision_llm(Some(&ws), None);
        assert_eq!(vision.provider, "mistral");
        assert_eq!(vision.model, "mistral-small-latest");
        assert_eq!(
            opts.resolved_backend(Some(&ws), None),
            PdfParserBackend::Vision
        );
        assert!(opts.resolved_pdf_parser(Some(&ws), None).backend_explicit());
    }

    #[test]
    fn apply_workspace_preserves_explicit_upload_overrides() {
        let mut ws = Workspace::new(Uuid::nil(), "ws", "ws");
        ws.vision_llm_provider = Some("mistral".into());
        ws.vision_llm_model = Some("mistral-small-latest".into());
        ws.pdf_parser_backend = Some(PdfParserBackend::Vision);

        let mut opts = PdfUploadOptions {
            vision_provider: Some("openai".into()),
            vision_model: Some("gpt-4o".into()),
            pdf_parser_backend: Some(PdfParserBackend::EdgeParse),
            ..Default::default()
        };
        opts.apply_workspace(&ws);

        assert_eq!(opts.vision_provider.as_deref(), Some("openai"));
        assert_eq!(opts.vision_model.as_deref(), Some("gpt-4o"));
        assert_eq!(opts.pdf_parser_backend, Some(PdfParserBackend::EdgeParse));
        assert_eq!(
            opts.resolved_backend(Some(&ws), None),
            PdfParserBackend::EdgeParse
        );
        let vision = opts.resolved_vision_llm(Some(&ws), None);
        assert_eq!(vision.provider, "openai");
        assert_eq!(vision.model, "gpt-4o");
    }

    #[test]
    fn vision_tenant_layer_wins_when_workspace_vision_unset() {
        let mut ws = Workspace::new(Uuid::nil(), "ws", "ws");
        ws.vision_llm_provider = None;
        ws.vision_llm_model = None;
        let mut tenant = Tenant::new("t", "t");
        tenant.default_vision_llm_provider = Some("mistral".into());
        tenant.default_vision_llm_model = Some("mistral-small-latest".into());
        let opts = PdfUploadOptions::default();
        let vision = opts.resolved_vision_llm(Some(&ws), Some(&tenant));
        assert_eq!(vision.provider, "mistral");
        assert_eq!(vision.model, "mistral-small-latest");
    }

    #[test]
    fn server_default_vision_is_inviolable() {
        let mut ws = Workspace::new(Uuid::nil(), "ws", "ws");
        ws.pdf_parser_backend = None;
        let opts = PdfUploadOptions::default();
        let resolved = opts.resolved_pdf_parser(Some(&ws), None);
        assert_eq!(resolved.runtime_backend, PdfParserBackend::Vision);
        assert!(!resolved.allows_auto_route);
        assert!(resolved.backend_explicit());
    }

    #[test]
    fn tenant_layer_wins_over_env_when_workspace_unset() {
        let mut ws = Workspace::new(Uuid::nil(), "ws", "ws");
        ws.pdf_parser_backend = None;
        let mut tenant = Tenant::new("t", "t");
        tenant.pdf_parser_backend = Some(PdfParserBackend::EdgeParse);
        let opts = PdfUploadOptions::default();
        assert_eq!(
            opts.resolved_backend(Some(&ws), Some(&tenant)),
            PdfParserBackend::EdgeParse
        );
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

    /// Authoritative progress / cancel / retry identity (`pdf-<uuid>`).
    ///
    /// SPEC-054 / GitHub #300: clients MUST subscribe to this id for progress.
    pub task_id: String,

    /// Optional client batch/request correlation ID (echoed if provided).
    ///
    /// Not a progress-store key — see `task_id`.
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

    /// Queue projection (SPEC-091 QW2 / LAW-Q4): 1-based FCFS pending position
    /// at admission. Only set on a fresh enqueue.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub queue_position: Option<u64>,

    /// Estimated seconds until claim (measured drain; clamped when unknown).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eta_seconds: Option<u64>,

    /// ETA basis: `measured` or `no_history` (honest uncertainty, R-15).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eta_basis: Option<String>,

    /// SPEC-160: the extraction mode this document will run with (`llm` or `decision`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extraction_mode: Option<String>,

    /// SPEC-160: who chose the mode (`document`, `workspace`, `env`, `default`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extraction_mode_source: Option<String>,
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

    /// Keyset from a previous `next_cursor`. When set, `page` is not an offset.
    #[serde(default)]
    pub cursor: Option<String>,
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

    /// Pass this as `cursor` to read the next page without walking skipped rows.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}
