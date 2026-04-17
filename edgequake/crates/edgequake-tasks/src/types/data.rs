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

    /// If true, ignore any saved conversion checkpoint and restart from page 1.
    /// WHY: Resume should be the safe default for long-running PDFs. A full restart
    /// must be an explicit choice, not an accidental side effect of reprocessing.
    #[serde(default)]
    pub restart_from_scratch: bool,
}

/// Text insert task payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextInsertData {
    pub text: String,
    pub file_source: String,
    pub workspace_id: String,
    pub metadata: Option<serde_json::Value>,
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
