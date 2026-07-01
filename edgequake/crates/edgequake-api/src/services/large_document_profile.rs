//! Large-document ingestion profile (SPEC-038 SSOT).
//!
//! Centralizes timeout budgets, text-density checks, routing policy, failure
//! classification, and ETA helpers for PDFs with hundreds of pages.

use edgequake_pdf::PdfParserBackend;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::safety_limits::{is_local_provider, vision_outer_timeout_secs, VISION_MAX_OUTER_TIMEOUT_SECS};

/// Page-count threshold for large-PDF admission UX and gleaning policy.
pub const LARGE_PDF_PAGE_THRESHOLD: usize = 100;

/// Disable gleaning at or above this page count (SPEC-038).
pub const LARGE_PDF_GLEANING_DISABLE_THRESHOLD: usize = 500;

/// Minimum extractable characters per page to treat a PDF as born-digital.
pub const DEFAULT_MIN_CHARS_PER_PAGE: usize = 200;

/// Worker timeout floor (matches default worker pool).
pub const TASK_TIMEOUT_FLOOR_SECS: u64 = 7200;

/// Worker timeout ceiling (24 h sanity cap).
pub const TASK_TIMEOUT_CEILING_SECS: u64 = VISION_MAX_OUTER_TIMEOUT_SECS;

/// Upload-time ingestion estimate (SPEC-038 REQ-038-04).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub struct IngestionEstimate {
    pub backend: String,
    pub convert_seconds: u64,
    pub extract_seconds: u64,
    pub total_seconds_pessimistic: u64,
    pub recommended_backend: String,
}

/// Typed failure classes for large-document ingestion (SPEC-038 REQ-038-09).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IngestionFailureClass {
    TimeoutPhaseConvert,
    TimeoutPhaseExtract,
    CircuitBreaker,
    DocumentTooLarge,
    EmbeddingLimit,
    ProviderUnavailable,
    Unknown,
}

impl IngestionFailureClass {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::TimeoutPhaseConvert => "timeout_phase_convert",
            Self::TimeoutPhaseExtract => "timeout_phase_extract",
            Self::CircuitBreaker => "circuit_breaker",
            Self::DocumentTooLarge => "document_too_large",
            Self::EmbeddingLimit => "embedding_limit",
            Self::ProviderUnavailable => "provider_unavailable",
            Self::Unknown => "unknown",
        }
    }

    pub fn recommended_action(self) -> &'static str {
        match self {
            Self::TimeoutPhaseConvert => "reprocess_edgeparse",
            Self::TimeoutPhaseExtract => "retry_faster_model",
            Self::CircuitBreaker => "reprocess_edgeparse",
            Self::DocumentTooLarge => "split_document",
            Self::EmbeddingLimit => "retry_or_support",
            Self::ProviderUnavailable => "check_provider",
            Self::Unknown => "retry",
        }
    }
}

/// Classify a permanent failure message into a stable `failure_class` key.
pub fn classify_ingestion_failure(error_msg: &str) -> IngestionFailureClass {
    let lower = error_msg.to_ascii_lowercase();
    if lower.contains("circuit breaker") {
        return IngestionFailureClass::CircuitBreaker;
    }
    if lower.contains("document too large") || lower.contains("exceeds maximum size") {
        return IngestionFailureClass::DocumentTooLarge;
    }
    if lower.contains("too many inputs") || lower.contains("embedding") && lower.contains("400") {
        return IngestionFailureClass::EmbeddingLimit;
    }
    if lower.contains("timed out") || lower.contains("timeout") {
        if lower.contains("vision") || lower.contains("convert") || lower.contains("markdown") {
            return IngestionFailureClass::TimeoutPhaseConvert;
        }
        return IngestionFailureClass::TimeoutPhaseExtract;
    }
    if lower.contains("provider") && (lower.contains("unavailable") || lower.contains("failed to create")) {
        return IngestionFailureClass::ProviderUnavailable;
    }
    IngestionFailureClass::Unknown
}

/// Profile inputs for a PDF at admission or worker time.
#[derive(Debug, Clone, PartialEq)]
pub struct LargeDocumentProfile {
    pub page_count: usize,
    pub file_size_bytes: u64,
}

impl LargeDocumentProfile {
    pub fn new(page_count: usize, file_size_bytes: u64) -> Self {
        Self {
            page_count: page_count.max(1),
            file_size_bytes,
        }
    }

    pub fn min_chars_per_page() -> usize {
        std::env::var("EDGEQUAKE_TEXT_PROBE_MIN_CHARS_PER_PAGE")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_MIN_CHARS_PER_PAGE)
    }

    /// Returns true when markdown has enough text per page to skip Vision OCR.
    pub fn markdown_has_text_layer(markdown: &str, page_count: usize) -> bool {
        let pages = page_count.max(1);
        let non_ws_chars = markdown.chars().filter(|c| !c.is_whitespace()).count();
        let chars_per_page = non_ws_chars / pages;
        chars_per_page >= Self::min_chars_per_page()
    }

    /// Outer vision conversion timeout for this document.
    pub fn vision_convert_secs(&self, provider: &str) -> u64 {
        vision_outer_timeout_secs(provider, self.page_count)
    }

    /// EdgeParse conversion estimate: ~0.5 s/page + 60 s overhead.
    pub fn edgeparse_convert_secs(&self) -> u64 {
        60_u64.saturating_add(self.page_count as u64 / 2)
    }

    /// Entity extraction estimate: ⌈pages / 16⌉ × 25 s (mock/cloud median).
    pub fn extract_secs(&self) -> u64 {
        let chunks = self.page_count;
        let waves = chunks.div_ceil(16);
        waves as u64 * 25
    }

    /// Embed + merge headroom.
    pub const PERSIST_BUFFER_SECS: u64 = 600;

    /// Total worker timeout budget for a PDF processing task.
    pub fn task_timeout_secs(&self, backend: PdfParserBackend, provider: &str) -> u64 {
        if let Ok(val) = std::env::var("TASK_PROCESSING_TIMEOUT_SECS") {
            if let Ok(n) = val.parse::<u64>() {
                return n.clamp(60, TASK_TIMEOUT_CEILING_SECS);
            }
        }

        let convert = match backend {
            PdfParserBackend::EdgeParse => self.edgeparse_convert_secs(),
            PdfParserBackend::Vision => self.vision_convert_secs(provider),
        };
        let raw = convert
            .saturating_add(self.extract_secs())
            .saturating_add(Self::PERSIST_BUFFER_SECS);
        let adjusted = match backend {
            PdfParserBackend::Vision if self.page_count >= 200 => {
                // Large vision jobs need headroom beyond naive sum (variance + merge).
                raw.saturating_add(convert / 4)
            }
            _ => raw,
        };
        adjusted.clamp(TASK_TIMEOUT_FLOOR_SECS, TASK_TIMEOUT_CEILING_SECS)
    }

    /// Upload ETA surfaced to clients (seconds).
    pub fn estimated_total_secs(&self, backend: PdfParserBackend, provider: &str) -> u64 {
        self.task_timeout_secs(backend, provider)
            .min(TASK_TIMEOUT_CEILING_SECS)
    }

    pub fn auto_routing_enabled() -> bool {
        match std::env::var("EDGEQUAKE_AUTO_PDF_ROUTING") {
            Ok(v) => !matches!(v.to_ascii_lowercase().as_str(), "0" | "false" | "no"),
            Err(_) => true,
        }
    }

    pub fn large_pdf_page_threshold() -> usize {
        std::env::var("EDGEQUAKE_LARGE_PDF_PAGE_THRESHOLD")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(LARGE_PDF_PAGE_THRESHOLD)
    }

    pub fn is_large_pdf(&self) -> bool {
        self.page_count >= Self::large_pdf_page_threshold()
    }

    pub fn should_disable_gleaning(&self) -> bool {
        self.page_count >= LARGE_PDF_GLEANING_DISABLE_THRESHOLD
    }

    /// Whether auto-routing may attempt EdgeParse before Vision.
    pub fn should_try_edgeparse_before_vision(
        backend: PdfParserBackend,
        backend_explicit: bool,
    ) -> bool {
        Self::auto_routing_enabled()
            && !backend_explicit
            && backend == PdfParserBackend::Vision
    }

    /// Build upload-time estimate DTO.
    pub fn ingestion_estimate(
        &self,
        backend: PdfParserBackend,
        provider: &str,
    ) -> IngestionEstimate {
        let convert = match backend {
            PdfParserBackend::EdgeParse => self.edgeparse_convert_secs(),
            PdfParserBackend::Vision => self.vision_convert_secs(provider),
        };
        let extract = self.extract_secs();
        let total = self.task_timeout_secs(backend, provider);
        let recommended = if self.is_large_pdf() && backend == PdfParserBackend::Vision {
            "edgeparse"
        } else {
            backend.as_str()
        };
        IngestionEstimate {
            backend: backend.as_str().to_string(),
            convert_seconds: convert,
            extract_seconds: extract,
            total_seconds_pessimistic: total,
            recommended_backend: recommended.to_string(),
        }
    }

    /// Median seconds per page for provider-aware upload estimates (legacy helper).
    pub fn secs_per_page_estimate(provider: &str) -> f64 {
        if is_local_provider(provider) {
            30.0
        } else {
            8.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reproducer_603_pages_edgeparse_under_worker_floor() {
        let profile = LargeDocumentProfile::new(603, 11_043_120);
        let timeout = profile.task_timeout_secs(PdfParserBackend::EdgeParse, "mistral");
        assert!(timeout >= TASK_TIMEOUT_FLOOR_SECS);
        assert!(profile.edgeparse_convert_secs() < 600);
    }

    #[test]
    fn reproducer_603_pages_vision_exceeds_old_cap_without_scale() {
        let profile = LargeDocumentProfile::new(603, 11_043_120);
        let vision_convert = profile.vision_convert_secs("mistral");
        assert!(vision_convert > 4000);
        let scaled = profile.task_timeout_secs(PdfParserBackend::Vision, "mistral");
        assert!(scaled > TASK_TIMEOUT_FLOOR_SECS);
    }

    #[test]
    fn born_digital_markdown_detection() {
        let pages = 603;
        let chars_per_page = LargeDocumentProfile::min_chars_per_page();
        let markdown = "x".repeat(pages * chars_per_page);
        assert!(LargeDocumentProfile::markdown_has_text_layer(&markdown, pages));
    }

    #[test]
    fn sparse_markdown_not_born_digital() {
        let markdown = "short";
        assert!(!LargeDocumentProfile::markdown_has_text_layer(&markdown, 603));
    }

    #[test]
    fn classify_timeout_convert() {
        let class = super::classify_ingestion_failure("Vision extraction timed out after 4944s");
        assert_eq!(class, super::IngestionFailureClass::TimeoutPhaseConvert);
    }

    #[test]
    fn ingestion_estimate_recommends_edgeparse_for_large_vision_default() {
        let profile = LargeDocumentProfile::new(603, 11_043_120);
        let est = profile.ingestion_estimate(PdfParserBackend::Vision, "mistral");
        assert_eq!(est.recommended_backend, "edgeparse");
    }
}
