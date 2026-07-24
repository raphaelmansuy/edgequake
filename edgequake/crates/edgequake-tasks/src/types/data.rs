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
///   default for retries of failed mid-pipeline runs. Prefer durable
///   extraction snapshot when present (SPEC-047 P7e).
/// - `MergeOnly`: reuse stored extractions (crash checkpoint or durable
///   snapshot) and skip LLM extract entirely — merge (+ re-embed if slim).
///   Fails closed if no snapshot exists (SPEC-047 P7e).
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
    /// Reuse cached markdown; prefer extraction snapshot, else re-extract.
    #[default]
    #[serde(rename = "entities")]
    EntitiesOnly,
    /// SPEC-047 P7e: skip LLM extract — merge from durable snapshot only.
    #[serde(rename = "merge")]
    MergeOnly,
}

impl ReprocessMode {
    /// Returns `true` when the PDF -> markdown conversion must be re-run.
    pub fn restart_from_scratch(self) -> bool {
        matches!(self, ReprocessMode::Full)
    }

    /// Returns `true` when LLM entity extraction must not run (snapshot required).
    pub fn merge_only(self) -> bool {
        matches!(self, ReprocessMode::MergeOnly)
    }
}

impl std::fmt::Display for ReprocessMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReprocessMode::Full => write!(f, "full"),
            ReprocessMode::EntitiesOnly => write!(f, "entities"),
            ReprocessMode::MergeOnly => write!(f, "merge"),
        }
    }
}

impl std::str::FromStr for ReprocessMode {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "full" | "reconvert" | "re-convert" => Ok(ReprocessMode::Full),
            "entities" | "entities_only" | "extract" => Ok(ReprocessMode::EntitiesOnly),
            "merge" | "merge_only" | "kg_only" => Ok(ReprocessMode::MergeOnly),
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

/// Async document deletion task payload.
///
/// Handler admits `status=deleting` and enqueues this task; the worker runs the
/// authoritative cascade (vectors → graph → KV → relational) and broadcasts
/// SPEC-050 deletion phases.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeletionTaskData {
    /// Document id as requested by the client (JSON id).
    pub document_id: String,
    /// Resolved KV key prefix (may differ from `document_id` on mismatch).
    pub key_prefix: String,
    /// Workspace id string used for vector/KV scoping.
    pub workspace_id: String,
    /// Tenant id string (may be "default").
    pub tenant_id: String,
    /// Transient deletion operation id for WebSocket correlation.
    pub deletion_track_id: String,
    /// Metadata KV key (`{key_prefix}-metadata`) when present.
    #[serde(default)]
    pub metadata_key: Option<String>,
    /// Chunk KV ids discovered at admit time.
    #[serde(default)]
    pub chunk_ids: Vec<String>,
    /// Whether content key existed at admit time.
    #[serde(default)]
    pub has_content: bool,
    /// Content hash for duplicate-detection key cleanup.
    #[serde(default)]
    pub content_hash: Option<String>,
    /// Linked PDF id when this document came from PDF upload.
    #[serde(default)]
    pub pdf_id: Option<String>,
    /// In-flight ingestion track_id to cancel (if any).
    #[serde(default)]
    pub ingest_track_id: Option<String>,
    /// Document status at admit time (pending/processing/deleting/…).
    #[serde(default)]
    pub document_status: Option<String>,
}

/// Selected multi-document deletion (SPEC-084 / GH-317).
///
/// One durable lifecycle task processes `document_ids` serially so the UI does
/// not storm N× `TaskType::Deletion` admits (pool / fairness park).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchDeletionTaskData {
    pub document_ids: Vec<String>,
    pub tenant_id: String,
    pub workspace_id: String,
    pub batch_track_id: String,
    #[serde(default)]
    pub deleted_count: usize,
    #[serde(default)]
    pub failed_ids: Vec<String>,
}

/// Checkpoint phase for durable workspace wipe-all (issue #309).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceWipePhase {
    #[default]
    Admitted,
    CancellingInflight,
    ClearingGraph,
    ClearingVectors,
    PurgingDocumentKv,
    ClearingRelational,
    Completed,
}

/// Policy for documents that are still processing when wipe-all is admitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum WipeActivePolicy {
    /// Cancel all in-flight workspace tasks, then clear everything.
    #[default]
    ForceCancelAll,
}

/// Durable workspace wipe-all task payload.
///
/// Handler admits and enqueues this task; the worker cancels inflight work,
/// clears graph/vectors once, then purges document KV/PDF/mm/relational rows.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceWipeTaskData {
    pub tenant_id: String,
    pub workspace_id: String,
    /// WebSocket / track correlation id (also used as task.track_id when possible).
    pub wipe_track_id: String,
    #[serde(default)]
    pub phase: WorkspaceWipePhase,
    #[serde(default)]
    pub deleted_count: usize,
    #[serde(default)]
    pub skipped_document_ids: Vec<String>,
    /// Resume cursor for batched KV purge (`metadata_key`).
    #[serde(default)]
    pub cursor_metadata_key: Option<String>,
    #[serde(default)]
    pub active_policy: WipeActivePolicy,
    #[serde(default)]
    pub total_chunks_deleted: usize,
    #[serde(default)]
    pub total_entities_removed: usize,
    #[serde(default)]
    pub total_relationships_removed: usize,
    #[serde(default)]
    pub total_pdfs_deleted: usize,
    /// Planned delete count captured at admit (for progress UI).
    #[serde(default)]
    pub planned_delete_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_wipe_task_data_round_trip() {
        let data = WorkspaceWipeTaskData {
            tenant_id: "default".into(),
            workspace_id: "ws".into(),
            wipe_track_id: "workspace_wipe-1".into(),
            phase: WorkspaceWipePhase::ClearingGraph,
            deleted_count: 3,
            skipped_document_ids: vec![],
            cursor_metadata_key: Some("doc-2-metadata".into()),
            active_policy: WipeActivePolicy::ForceCancelAll,
            total_chunks_deleted: 10,
            total_entities_removed: 4,
            total_relationships_removed: 2,
            total_pdfs_deleted: 1,
            planned_delete_count: 5,
        };
        let v = serde_json::to_value(&data).unwrap();
        let back: WorkspaceWipeTaskData = serde_json::from_value(v).unwrap();
        assert_eq!(back.phase, WorkspaceWipePhase::ClearingGraph);
        assert_eq!(back.wipe_track_id, "workspace_wipe-1");
        assert_eq!(back.planned_delete_count, 5);
        assert_eq!(
            crate::types::TaskType::WorkspaceWipe.to_string(),
            "workspace_wipe"
        );
    }

    #[test]
    fn full_mode_requests_fresh_conversion() {
        assert!(ReprocessMode::Full.restart_from_scratch());
    }

    #[test]
    fn entities_mode_reuses_markdown() {
        assert!(!ReprocessMode::EntitiesOnly.restart_from_scratch());
        assert!(!ReprocessMode::EntitiesOnly.merge_only());
    }

    #[test]
    fn merge_only_skips_extract_and_vision() {
        assert!(!ReprocessMode::MergeOnly.restart_from_scratch());
        assert!(ReprocessMode::MergeOnly.merge_only());
        assert_eq!(
            "merge_only".parse::<ReprocessMode>().unwrap(),
            ReprocessMode::MergeOnly
        );
        assert_eq!(ReprocessMode::MergeOnly.to_string(), "merge");
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
