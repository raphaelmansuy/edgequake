//! Task-specific data payloads.
//!
//! Typed payloads for each task type, serialized into the
//! `task_data` JSON field of a Task.

use edgequake_pdf::PdfParserBackend;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Document upload task payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentUploadData {
    pub file_path: String,
    pub content_type: String,
    pub workspace_id: String,
    pub metadata: Option<serde_json::Value>,
}

/// Reprocess intent for a PDF document (DRY single knob).
///
/// - `Full`: re-run the PDF -> markdown conversion from the stored PDF bytes
///   (vision/pdf2md), clearing any cached markdown. Chosen when the user
///   explicitly wants re-conversion (Replace, or "Re-convert from PDF").
/// - `EntitiesOnly`: reuse the existing cached markdown and only re-run the
///   knowledge-graph pipeline (chunk / extract / embed). This is the safe
///   default for retries of failed mid-pipeline runs.
///
/// WHY: `restart_from_scratch` alone was ambiguous and never set to `true` in
/// production, so reprocessing silently reused stale markdown. Making intent
/// explicit keeps the resume shortcut driven by a single, named source of
/// truth.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ReprocessMode {
    /// Re-convert the PDF to markdown from scratch (vision tokens spent).
    Full,
    /// Reuse cached markdown; only re-run entity extraction (default).
    #[default]
    EntitiesOnly,
}

impl ReprocessMode {
    /// Returns `true` when the PDF -> markdown conversion must be re-run.
    pub fn restart_from_scratch(self) -> bool {
        matches!(self, ReprocessMode::Full)
    }
}

impl std::fmt::Display for ReprocessMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReprocessMode::Full => write!(f, "full"),
            ReprocessMode::EntitiesOnly => write!(f, "entities"),
        }
    }
}

impl std::str::FromStr for ReprocessMode {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "full" | "reconvert" | "re-convert" => Ok(ReprocessMode::Full),
            "entities" | "entities_only" | "extract" => Ok(ReprocessMode::EntitiesOnly),
            other => Err(format!("unknown reprocess mode '{other}'")),
        }
    }
}

/// PDF processing task payload
///
/// @implements SPEC-007: PDF Upload Support
///
/// This structure contains all information needed to process a PDF document:
/// - Extract content (text or vision)
/// - Convert to markdown
/// - Ingest into knowledge graph
///
/// @implements SPEC-002: Unified Ingestion Pipeline
/// OODA-05: Added tenant_id for multi-tenant context propagation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfProcessingData {
    /// PDF document ID
    pub pdf_id: Uuid,

    /// Tenant ID for multi-tenant isolation
    /// OODA-05: Required for document metadata to be visible in workspace queries
    pub tenant_id: Uuid,

    /// Workspace ID for isolation
    pub workspace_id: Uuid,

    /// Enable vision LLM processing
    pub enable_vision: bool,

    /// Vision provider to use (openai, ollama)
    pub vision_provider: String,

    /// Optional vision model override
    pub vision_model: Option<String>,

    /// Existing document ID to reuse during rebuild/reprocessing.
    /// WHY: When rebuilding knowledge graph or reprocessing PDF documents,
    /// we must reuse the existing document ID so the old document is updated
    /// in-place rather than creating an orphaned duplicate. Without this,
    /// the old document still references the same pdf_id whose markdown_content
    /// was overwritten, causing it to display wrong/hallucinated content.
    #[serde(default)]
    pub existing_document_id: Option<String>,

    /// PDF parser backend to use for this task.
    /// Old queued tasks omit this field and therefore default to Vision.
    #[serde(default)]
    pub pdf_parser_backend: PdfParserBackend,

    /// When true, the user/workspace/env explicitly chose `pdf_parser_backend`.
    /// SPEC-038: auto-routing to EdgeParse is disabled when this is set.
    #[serde(default)]
    pub pdf_parser_backend_explicit: bool,

    /// If true, ignore any saved conversion checkpoint and restart from page 1.
    /// WHY: Resume should be the safe default for long-running PDFs. A full restart
    /// must be an explicit choice, not an accidental side effect of reprocessing.
    #[serde(default)]
    pub restart_from_scratch: bool,

    /// Explicit reprocess intent. Backward compatible: older queued tasks and
    /// fresh uploads leave this `None`, which behaves as `EntitiesOnly` for
    /// retries and as a no-op for first-time uploads.
    #[serde(default)]
    pub reprocess_mode: Option<ReprocessMode>,

    /// LightRAG-style multimodal flags: `"i"` images, `"t"` tables, `"e"` equations.
    /// When set (e.g. `"i"` or `"ite"`), post-conversion markdown is scanned for
    /// inline placeholders and enriched via VLM where enabled.
    #[serde(default)]
    pub multimodal_process_options: Option<String>,
}

/// Text insert task payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextInsertData {
    pub text: String,
    pub file_source: String,
    pub workspace_id: String,
    pub metadata: Option<serde_json::Value>,
}

/// Knowledge injection task payload (SPEC-024 Phase 1.2).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeInjectionData {
    pub doc_id: String,
    pub content: String,
    pub workspace_id: String,
    pub meta_key: String,
    pub injection_id: String,
    pub name: String,
    pub source_type: String,
    pub source_filename: Option<String>,
    pub version: u32,
    pub created_at: String,
    pub data_tenant_id: Option<String>,
}

/// Directory scan task payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryScanData {
    pub directory_path: String,
    pub recursive: bool,
    pub file_pattern: Option<String>,
    pub workspace_id: String,
}

/// Reindex task payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReindexData {
    pub document_ids: Vec<String>,
    pub workspace_id: String,
    pub reason: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_mode_requests_fresh_conversion() {
        assert!(ReprocessMode::Full.restart_from_scratch());
    }

    #[test]
    fn entities_mode_reuses_markdown() {
        assert!(!ReprocessMode::EntitiesOnly.restart_from_scratch());
    }

    #[test]
    fn default_mode_is_entities_only() {
        // Backward-compat: unspecified intent must not re-run vision conversion.
        assert_eq!(ReprocessMode::default(), ReprocessMode::EntitiesOnly);
    }

    #[test]
    fn from_str_accepts_aliases() {
        assert_eq!(
            "full".parse::<ReprocessMode>().unwrap(),
            ReprocessMode::Full
        );
        assert_eq!(
            "re-convert".parse::<ReprocessMode>().unwrap(),
            ReprocessMode::Full
        );
        assert_eq!(
            "entities".parse::<ReprocessMode>().unwrap(),
            ReprocessMode::EntitiesOnly
        );
        assert_eq!(
            "extract".parse::<ReprocessMode>().unwrap(),
            ReprocessMode::EntitiesOnly
        );
    }

    #[test]
    fn from_str_rejects_unknown() {
        assert!("banana".parse::<ReprocessMode>().is_err());
    }

    #[test]
    fn display_roundtrips_serde() {
        // serde rename_all = "lowercase" must match Display so the API query
        // param and the JSON payload use the same vocabulary.
        let full = serde_json::to_string(&ReprocessMode::Full).unwrap();
        assert_eq!(full, "\"full\"");
        assert_eq!(ReprocessMode::Full.to_string(), "full");
    }
}
