//! Document reprocessing, recovery, and chunk retry DTOs.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::defaults::{
    default_max_chunk_retries, default_max_reprocess, default_stuck_threshold_minutes,
};

// ============================================================================
// Reprocess DTOs
// ============================================================================

/// Request to reprocess documents.
/// Can filter by document_id (specific document), track_id (batch), or neither (all failed).
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ReprocessFailedRequest {
    /// Optional document ID to reprocess a specific document.
    /// If provided, reprocesses this document regardless of its status.
    #[serde(default)]
    pub document_id: Option<String>,

    /// Optional track ID to reprocess. If not provided, all failed documents are reprocessed.
    #[serde(default)]
    pub track_id: Option<String>,

    /// Maximum number of documents to reprocess.
    #[serde(default = "default_max_reprocess")]
    pub max_documents: usize,

    /// Force reprocess even if document is not failed. Default: false.
    #[serde(default)]
    pub force: bool,

    /// Reprocess intent for PDF documents.
    /// - `entities` (default): reuse cached markdown, only re-run the KG pipeline.
    /// - `full`: re-run PDF -> markdown conversion from the stored PDF bytes
    ///   (spends vision tokens) before re-running the KG pipeline.
    ///
    /// WHY: Users must be able to re-convert a PDF to markdown when the cached
    /// conversion is stale or was produced with a different vision model.
    /// Non-PDF documents ignore this field.
    #[serde(default)]
    pub mode: Option<String>,
}

/// WHY: Manual Default impl ensures max_documents defaults to 100 (same as serde default),
/// not 0 (usize::default()). Used when handler receives missing/empty request body.
impl Default for ReprocessFailedRequest {
    fn default() -> Self {
        Self {
            document_id: None,
            track_id: None,
            max_documents: default_max_reprocess(),
            force: false,
            mode: None,
        }
    }
}

/// Response from reprocess operation.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ReprocessFailedResponse {
    /// Track ID for the reprocess batch.
    pub track_id: String,

    /// Level 4 v2 migration hint (additive).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub v2_migration: Option<crate::services::job_registry::V2MigrationHint>,

    /// Number of failed documents found.
    pub failed_found: usize,

    /// Number of documents queued for reprocessing.
    pub requeued: usize,

    /// List of document IDs being reprocessed.
    pub document_ids: Vec<String>,
}

// ============================================================================
// Recovery DTOs
// ============================================================================

/// Request to recover stuck processing documents.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct RecoverStuckRequest {
    /// Minimum age in minutes for a document to be considered "stuck".
    /// Default: 10 minutes.
    #[serde(default = "default_stuck_threshold_minutes")]
    pub stuck_threshold_minutes: u64,

    /// Maximum number of documents to recover.
    #[serde(default = "default_max_reprocess")]
    pub max_documents: usize,

    /// Optional list of specific document IDs to recover.
    #[serde(default)]
    pub document_ids: Option<Vec<String>>,
}

/// Response from recover stuck operation.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RecoverStuckResponse {
    /// Track ID for the recovery batch.
    pub track_id: String,

    /// Level 4 v2 migration hint (additive).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub v2_migration: Option<crate::services::job_registry::V2MigrationHint>,

    /// Number of stuck documents found.
    pub stuck_found: usize,

    /// Number of documents queued for reprocessing.
    pub requeued: usize,

    /// List of document IDs being recovered.
    pub document_ids: Vec<String>,

    /// List of document titles for reference.
    pub document_titles: Vec<String>,
}

// ============================================================================
// Retry Chunks DTOs
// ============================================================================

/// Request to retry failed chunks for a document.
///
/// @implements FEAT0411 (Chunk retry types)
///
/// # OODA-03: Chunk-Level Retry Queue
///
/// Allows retrying specific failed chunks without reprocessing the entire document.
/// This is more efficient for large documents where only a few chunks failed.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct RetryChunksRequest {
    /// Specific chunk indices to retry. If empty, retries all failed chunks for the document.
    #[serde(default)]
    pub chunk_indices: Vec<usize>,

    /// Force retry even if chunk already succeeded. Default: false.
    #[serde(default)]
    pub force: bool,

    /// Maximum number of retry attempts per chunk.
    #[serde(default = "default_max_chunk_retries")]
    pub max_retries: usize,
}

/// Response from retry chunks operation.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RetryChunksResponse {
    /// Document ID.
    pub document_id: String,

    /// Number of chunks queued for retry.
    pub chunks_queued: usize,

    /// Specific chunk indices being retried.
    pub chunk_indices: Vec<usize>,

    /// Status message.
    pub message: String,

    /// Whether the feature is fully implemented.
    /// When false, the endpoint accepts the request but retry is not yet processed.
    pub implemented: bool,
}

/// Information about a failed chunk for retry purposes.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FailedChunkInfo {
    /// Chunk index within the document.
    pub chunk_index: usize,

    /// Chunk identifier (e.g., "doc-xxx-chunk-0").
    pub chunk_id: String,

    /// Error message from the failed extraction.
    pub error_message: String,

    /// Whether the failure was due to timeout.
    pub was_timeout: bool,

    /// Number of retry attempts so far.
    pub retry_attempts: usize,

    /// Current status: pending, retrying, succeeded, abandoned.
    pub status: String,
}

/// Response listing failed chunks for a document.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ListFailedChunksResponse {
    /// Document ID.
    pub document_id: String,

    /// List of failed chunks.
    pub failed_chunks: Vec<FailedChunkInfo>,

    /// Total number of chunks in the document.
    pub total_chunks: usize,

    /// Number of successful chunks.
    pub successful_chunks: usize,
}

// ============================================================================
// Multimodal re-analyze DTOs (Phase 4h — LightRAG analyze-without-reparse)
// ============================================================================

fn default_reindex_true() -> bool {
    true
}

/// Request to re-run multimodal analyze on stored markdown (no PDF re-convert).
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ReanalyzeMultimodalRequest {
    /// Override stored `multimodal_process_options` (e.g. `"ite"`).
    #[serde(default)]
    pub process_options: Option<String>,

    /// Re-run entity extraction after analyze. Default: true.
    #[serde(default = "default_reindex_true")]
    pub reindex: bool,
}

/// Response from multimodal re-analyze.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ReanalyzeMultimodalResponse {
    pub document_id: String,
    pub track_id: Option<String>,
    pub requeued: bool,
    pub success: u32,
    pub skipped: u32,
    pub failed: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub v2_migration: Option<crate::services::job_registry::V2MigrationHint>,
}
