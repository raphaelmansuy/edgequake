//! Document DTO types for the documents API handlers.
//!
//! This module contains all request/response data transfer objects used by
//! the document management endpoints. Extracted from documents.rs for modularity.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

// ============================================================================
// Default value functions
// ============================================================================

pub fn default_enable_gleaning() -> bool {
    true
}

pub fn default_max_gleaning() -> usize {
    1
}

pub fn default_use_llm_summarization() -> bool {
    true
}

pub fn default_page() -> usize {
    1
}

pub fn default_page_size() -> usize {
    20
}

pub fn default_recursive() -> bool {
    true
}

pub fn default_max_files() -> usize {
    1000
}

/// Default true value for document-related boolean fields.
pub fn documents_default_true() -> bool {
    true
}

pub fn default_max_reprocess() -> usize {
    100
}

pub fn default_stuck_threshold_minutes() -> u64 {
    10
}

// ============================================================================
// Upload DTOs
// ============================================================================

/// Document upload request.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct UploadDocumentRequest {
    /// Document content.
    pub content: String,

    /// Optional document title.
    #[serde(default)]
    pub title: Option<String>,

    /// Optional document metadata.
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,

    /// Whether to process asynchronously (default: false for backwards compatibility)
    #[serde(default)]
    pub async_processing: bool,

    /// Optional track ID for batch grouping. If not provided, one will be generated.
    #[serde(default)]
    pub track_id: Option<String>,

    /// Enable gleaning (multiple extraction passes) for higher quality entity extraction.
    #[serde(default = "default_enable_gleaning")]
    pub enable_gleaning: bool,

    /// Maximum number of gleaning passes (1-3 recommended).
    #[serde(default = "default_max_gleaning")]
    pub max_gleaning: usize,

    /// Enable LLM-powered description summarization during merge.
    #[serde(default = "default_use_llm_summarization")]
    pub use_llm_summarization: bool,
}

/// Document upload response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct UploadDocumentResponse {
    /// Generated document ID.
    pub document_id: String,

    /// Processing status.
    pub status: String,

    /// Task track ID (only set when async_processing is true).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,

    /// Track ID for batch grouping.
    pub track_id: String,

    /// ID of existing document if this is a duplicate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duplicate_of: Option<String>,

    /// Number of chunks created (only set for sync processing).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunk_count: Option<usize>,

    /// Number of entities extracted (only set for sync processing).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_count: Option<usize>,

    /// Number of relationships extracted (only set for sync processing).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relationship_count: Option<usize>,

    /// Cost information (only set for sync processing).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost: Option<DocumentCostInfo>,
}

/// Cost information for a processed document.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct DocumentCostInfo {
    /// Total cost in USD.
    pub total_cost_usd: f64,

    /// Formatted cost string (e.g., "$0.0045").
    pub formatted_cost: String,

    /// Total input tokens used.
    pub input_tokens: usize,

    /// Total output tokens used.
    pub output_tokens: usize,

    /// Total tokens (input + output).
    pub total_tokens: usize,

    /// LLM model used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub llm_model: Option<String>,

    /// Embedding model used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding_model: Option<String>,
}

impl DocumentCostInfo {
    /// Format cost to 6 decimal places.
    pub fn format_cost(cost: f64) -> String {
        format!("${:.6}", cost)
    }

    /// Create a new DocumentCostInfo from raw values.
    /// Used when constructing from pipeline result stats.
    pub fn new(
        cost_usd: f64,
        input_tokens: usize,
        output_tokens: usize,
        total_tokens: usize,
        llm_model: Option<String>,
        embedding_model: Option<String>,
    ) -> Self {
        Self {
            total_cost_usd: cost_usd,
            formatted_cost: Self::format_cost(cost_usd),
            input_tokens,
            output_tokens,
            total_tokens,
            llm_model,
            embedding_model,
        }
    }
}

// ============================================================================
// List DTOs
// ============================================================================

/// List documents request.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ListDocumentsRequest {
    /// Page number.
    #[serde(default = "default_page")]
    pub page: usize,

    /// Page size.
    #[serde(default = "default_page_size")]
    pub page_size: usize,
}

/// Status counts for document filtering.
///
/// @implements FIX-5: Strict mode enforcement with partial_failure status
///
/// Document statuses:
/// - `pending`: Queued for processing, not yet started
/// - `processing`: Currently being processed
/// - `completed`: Processing finished successfully with entities extracted
/// - `partial_failure`: Processing completed but with issues (e.g., 0 entities extracted)
/// - `failed`: Processing failed with an error
/// - `cancelled`: Processing was cancelled by user
#[derive(Debug, Clone, Serialize, Default, ToSchema)]
pub struct StatusCounts {
    /// Number of pending documents.
    pub pending: usize,
    /// Number of processing documents.
    pub processing: usize,
    /// Number of completed documents.
    pub completed: usize,
    /// Number of documents with partial failure (processed but 0 entities).
    /// @implements FIX-5
    pub partial_failure: usize,
    /// Number of failed documents.
    pub failed: usize,
    /// Number of cancelled documents.
    pub cancelled: usize,
}

/// List documents response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ListDocumentsResponse {
    /// List of documents.
    pub documents: Vec<DocumentSummary>,

    /// Total document count.
    pub total: usize,

    /// Current page (1-indexed).
    pub page: usize,

    /// Page size.
    pub page_size: usize,

    /// Total number of pages.
    pub total_pages: usize,

    /// Whether there are more pages after this one.
    pub has_more: bool,

    /// Status counts for all documents (not just current page).
    pub status_counts: StatusCounts,
}

/// Document summary.
///
/// @implements SPEC-002: Unified Ingestion Pipeline
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct DocumentSummary {
    /// Document ID.
    pub id: String,

    /// Document title.
    pub title: Option<String>,

    /// Original file name (used for display if title is not set).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_name: Option<String>,

    /// First 200 characters of document content (preview).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_summary: Option<String>,

    /// Total length of document content in characters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_length: Option<usize>,

    /// Number of chunks.
    pub chunk_count: usize,

    /// Number of entities extracted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_count: Option<usize>,

    /// Document processing status (legacy - use current_stage for new code).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,

    /// Error message if processing failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,

    /// Track ID for batch grouping.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub track_id: Option<String>,

    /// Creation timestamp (ISO 8601 format).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,

    /// Last update timestamp (ISO 8601 format).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,

    /// Total cost in USD for processing this document.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_usd: Option<f64>,

    /// Input tokens used for processing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_tokens: Option<usize>,

    /// Output tokens used for processing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_tokens: Option<usize>,

    /// Total tokens (input + output).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_tokens: Option<usize>,

    /// LLM model used for processing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub llm_model: Option<String>,

    /// Embedding model used for processing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding_model: Option<String>,

    // ========================================================================
    // SPEC-002: Unified Ingestion Pipeline Fields
    // ========================================================================
    /// Document source type (pdf, markdown, text).
    /// @implements SPEC-002
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "pdf")]
    pub source_type: Option<String>,

    /// Current ingestion stage (aligned with UnifiedStage enum).
    /// Stages: uploading, converting, preprocessing, chunking, extracting,
    /// gleaning, merging, summarizing, embedding, storing, completed, failed.
    /// @implements SPEC-002
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "extracting")]
    pub current_stage: Option<String>,

    /// Progress within current stage (0.0 to 1.0).
    /// @implements SPEC-002
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = json!(0.45))]
    pub stage_progress: Option<f32>,

    /// Human-readable message for current stage.
    /// @implements SPEC-002
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "Extracting entities from chunk 5/12")]
    pub stage_message: Option<String>,

    /// Linked PDF document ID (only set if source_type is "pdf").
    /// Used to fetch PDF content for viewing.
    /// @implements SPEC-002
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "8866e3c3-bbd6-4384-b86f-215c9844914d")]
    pub pdf_id: Option<String>,
}

// ============================================================================
// Detail DTOs
// ============================================================================

/// Get document by ID request.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct GetDocumentRequest {
    /// Document ID.
    pub document_id: String,
}

/// Document details response with full content.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct DocumentDetailResponse {
    /// Document ID.
    pub id: String,

    /// Document title.
    pub title: Option<String>,

    /// Original file name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_name: Option<String>,

    /// Full document content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,

    /// Content summary (first 200 chars).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_summary: Option<String>,

    /// Total content length in characters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_length: Option<usize>,

    /// Content hash (SHA-256) for deduplication.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_hash: Option<String>,

    /// Number of chunks.
    pub chunk_count: usize,

    /// Number of entities extracted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_count: Option<usize>,

    /// Number of relationships extracted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relationship_count: Option<usize>,

    /// Document processing status.
    pub status: String,

    /// Error message if processing failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,

    /// Source type (file, text, url).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_type: Option<String>,

    /// MIME type of the document.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,

    /// File size in bytes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_size: Option<usize>,

    /// Track ID for batch grouping.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub track_id: Option<String>,

    /// Tenant ID for multi-tenancy.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,

    /// Workspace ID for multi-tenancy.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_id: Option<String>,

    /// Creation timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,

    /// Last update timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,

    /// Processing completed timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub processed_at: Option<String>,

    /// Extraction lineage information.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lineage: Option<DocumentLineage>,

    /// Additional custom metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,

    /// Linked PDF document ID (only set if source_type is "pdf").
    /// Used to fetch PDF content for viewing.
    /// @implements SPEC-002
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pdf_id: Option<String>,
}

/// Extraction lineage information for a document.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct DocumentLineage {
    /// LLM model used for entity extraction.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub llm_model: Option<String>,

    /// Embedding model used for vector embeddings.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding_model: Option<String>,

    /// Embedding dimensions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding_dimensions: Option<usize>,

    /// List of keywords extracted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keywords: Option<Vec<String>>,

    /// Entity types extracted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_types: Option<Vec<String>>,

    /// Relationship types extracted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relationship_types: Option<Vec<String>>,

    /// Chunking strategy used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunking_strategy: Option<String>,

    /// Average chunk size in characters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avg_chunk_size: Option<usize>,

    /// Processing duration in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub processing_duration_ms: Option<u64>,

    /// Input tokens consumed during LLM processing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_tokens: Option<usize>,

    /// Output tokens generated during LLM processing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_tokens: Option<usize>,

    /// Total tokens (input + output).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_tokens: Option<usize>,

    /// Estimated cost in USD.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_usd: Option<f64>,
}

// ============================================================================
// Delete DTOs
// ============================================================================

/// Document deletion response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct DeleteDocumentResponse {
    /// Document ID.
    pub document_id: String,

    /// Whether the document was deleted.
    pub deleted: bool,

    /// Number of chunks deleted.
    pub chunks_deleted: usize,

    /// Number of entities affected.
    pub entities_affected: usize,

    /// Number of relationships affected.
    pub relationships_affected: usize,
}

/// Bulk document deletion response.
///
/// WHY: Frontend "Clear All" button needs a bulk delete endpoint.
/// Returns aggregated deletion statistics across all documents.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct DeleteAllDocumentsResponse {
    /// Total number of documents deleted.
    pub deleted_count: usize,

    /// Total number of chunks deleted across all documents.
    pub total_chunks_deleted: usize,

    /// Total number of entities removed (no other references).
    pub total_entities_removed: usize,

    /// Total number of relationships removed.
    pub total_relationships_removed: usize,

    /// Total number of PDF documents deleted from separate storage.
    pub total_pdfs_deleted: usize,

    /// Number of documents skipped (processing/pending status).
    pub skipped_count: usize,

    /// Document IDs that were skipped due to active processing.
    pub skipped_documents: Vec<String>,
}

/// Document deletion impact analysis response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct DeletionImpactResponse {
    /// Document ID.
    pub document_id: String,

    /// Number of chunks that would be deleted.
    pub chunks_to_delete: usize,

    /// Number of entities that would be completely removed (no other sources).
    pub entities_to_remove: usize,

    /// Number of entities that would be updated (some sources remaining).
    pub entities_to_update: usize,

    /// Number of relationships that would be completely removed.
    pub relationships_to_remove: usize,

    /// Number of relationships that would be updated.
    pub relationships_to_update: usize,

    /// Preview is read-only; document NOT deleted.
    pub preview_only: bool,
}

// ============================================================================
// File Upload DTOs
// ============================================================================

/// File upload response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct FileUploadResponse {
    /// Generated document ID.
    pub document_id: String,

    /// Original filename.
    pub filename: String,

    /// File size in bytes.
    pub size: usize,

    /// Content hash (SHA-256).
    pub content_hash: String,

    /// Processing status.
    pub status: String,

    /// Number of chunks created.
    pub chunk_count: usize,

    /// Number of entities extracted.
    pub entity_count: usize,

    /// Number of relationships extracted.
    pub relationship_count: usize,

    /// Whether this was a duplicate (already processed).
    pub is_duplicate: bool,
}

// ============================================================================
// Batch Upload DTOs
// ============================================================================

/// Batch file upload response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct BatchUploadResponse {
    /// Total files received.
    pub total_files: usize,

    /// Successfully processed files.
    pub processed: usize,

    /// Duplicate files (skipped).
    pub duplicates: usize,

    /// Failed files.
    pub failed: usize,

    /// Results for each file.
    pub results: Vec<BatchFileResult>,
}

/// Result for a single file in batch upload.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct BatchFileResult {
    /// Original filename.
    pub filename: String,

    /// Document ID if successful.
    pub document_id: Option<String>,

    /// Status: processed, duplicate, or failed.
    pub status: String,

    /// Error message if failed.
    pub error: Option<String>,
}

// ============================================================================
// Track Status DTOs
// ============================================================================

/// Track status response for batch grouping.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct TrackStatusResponse {
    /// Track ID for this batch.
    pub track_id: String,

    /// When the first document was uploaded.
    pub created_at: Option<String>,

    /// Documents in this batch.
    pub documents: Vec<DocumentSummary>,

    /// Total number of documents.
    pub total_count: usize,

    /// Status summary for the batch.
    pub status_summary: StatusCounts,

    /// Whether processing is complete (all docs completed or failed).
    pub is_complete: bool,

    /// Latest processing message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_message: Option<String>,
}

// ============================================================================
// Directory Scan DTOs
// ============================================================================

/// Request to scan a directory for documents.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ScanDirectoryRequest {
    /// Path to the directory to scan.
    pub path: String,

    /// File extensions to include (e.g., ["txt", "md", "pdf"]).
    /// If empty, all files are included.
    #[serde(default)]
    pub extensions: Vec<String>,

    /// Whether to scan subdirectories recursively.
    #[serde(default = "default_recursive")]
    pub recursive: bool,

    /// Maximum number of files to scan.
    #[serde(default = "default_max_files")]
    pub max_files: usize,

    /// Whether to process documents asynchronously.
    #[serde(default = "documents_default_true")]
    pub async_processing: bool,

    /// Optional track ID for batch grouping.
    #[serde(default)]
    pub track_id: Option<String>,
}

/// Response from directory scan.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ScanDirectoryResponse {
    /// Track ID for the scan batch.
    pub track_id: String,

    /// Number of files found.
    pub files_found: usize,

    /// Number of files queued for processing.
    pub files_queued: usize,

    /// Number of files skipped (already processed or filtered).
    pub files_skipped: usize,

    /// List of queued file paths.
    pub queued_files: Vec<String>,

    /// List of skipped file paths with reasons.
    pub skipped_files: Vec<SkippedFile>,
}

/// Information about a skipped file.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SkippedFile {
    /// Path to the file.
    pub path: String,

    /// Reason for skipping.
    pub reason: String,
}

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
}

/// Response from reprocess operation.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ReprocessFailedResponse {
    /// Track ID for the reprocess batch.
    pub track_id: String,

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

/// Default maximum retry attempts for chunks.
pub fn default_max_chunk_retries() -> usize {
    3
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
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_upload_request_validation() {
        let request = UploadDocumentRequest {
            content: "Test content".to_string(),
            title: Some("Test".to_string()),
            metadata: None,
            async_processing: false,
            track_id: None,
            enable_gleaning: true,
            max_gleaning: 1,
            use_llm_summarization: true,
        };

        assert!(!request.content.is_empty());
    }

    #[test]
    fn test_upload_request_serialization() {
        let json = r#"{
            "content": "Hello world",
            "title": "Test Doc",
            "metadata": {"key": "value"}
        }"#;

        let request: UploadDocumentRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.content, "Hello world");
        assert_eq!(request.title, Some("Test Doc".to_string()));
        assert!(request.metadata.is_some());
    }

    #[test]
    fn test_upload_request_minimal() {
        let json = r#"{"content": "Just content"}"#;

        let request: UploadDocumentRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.content, "Just content");
        assert!(request.title.is_none());
        assert!(request.metadata.is_none());
    }

    #[test]
    fn test_upload_response_serialization() {
        let response = UploadDocumentResponse {
            document_id: "doc-123".to_string(),
            status: "processed".to_string(),
            task_id: None,
            track_id: "upload_20240101_abc12345".to_string(),
            duplicate_of: None,
            chunk_count: Some(5),
            entity_count: Some(3),
            relationship_count: Some(2),
            cost: Some(DocumentCostInfo {
                total_cost_usd: 0.0045,
                formatted_cost: "$0.004500".to_string(),
                input_tokens: 1000,
                output_tokens: 500,
                total_tokens: 1500,
                llm_model: Some("gpt-4o-mini".to_string()),
                embedding_model: Some("text-embedding-3-small".to_string()),
            }),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("doc-123"));
        assert!(json.contains("processed"));
        assert!(json.contains("cost"));
        assert!(json.contains("0.0045"));
    }

    #[test]
    fn test_list_documents_request_defaults() {
        let json = r#"{}"#;
        let request: ListDocumentsRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.page, 1);
        assert_eq!(request.page_size, 20);
    }

    #[test]
    fn test_list_documents_request_custom() {
        let json = r#"{"page": 3, "page_size": 50}"#;
        let request: ListDocumentsRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.page, 3);
        assert_eq!(request.page_size, 50);
    }

    #[test]
    fn test_document_summary_serialization() {
        let summary = DocumentSummary {
            id: "doc-456".to_string(),
            title: Some("My Document".to_string()),
            file_name: None,
            content_summary: Some("This is the first 200 chars of the document...".to_string()),
            content_length: Some(5000),
            chunk_count: 10,
            entity_count: None,
            status: Some("completed".to_string()),
            error_message: None,
            track_id: Some("upload_20240101_abc12345".to_string()),
            created_at: None,
            updated_at: None,
            cost_usd: None,
            input_tokens: None,
            output_tokens: None,
            total_tokens: None,
            llm_model: None,
            embedding_model: None,
            // SPEC-002 fields
            source_type: Some("markdown".to_string()),
            current_stage: Some("completed".to_string()),
            stage_progress: Some(1.0),
            stage_message: None,
            pdf_id: None,
        };

        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("doc-456"));
        assert!(json.contains("My Document"));
    }

    #[test]
    fn test_list_documents_response_serialization() {
        let response = ListDocumentsResponse {
            documents: vec![DocumentSummary {
                id: "doc-1".to_string(),
                title: None,
                file_name: None,
                content_summary: None,
                content_length: None,
                chunk_count: 5,
                entity_count: None,
                status: Some("completed".to_string()),
                error_message: None,
                track_id: None,
                created_at: None,
                updated_at: None,
                cost_usd: None,
                input_tokens: None,
                output_tokens: None,
                total_tokens: None,
                llm_model: None,
                embedding_model: None,
                // SPEC-002 fields
                source_type: None,
                current_stage: Some("completed".to_string()),
                stage_progress: None,
                stage_message: None,
                pdf_id: None,
            }],
            total: 1,
            page: 1,
            page_size: 20,
            total_pages: 1,
            has_more: false,
            status_counts: StatusCounts {
                pending: 0,
                processing: 0,
                completed: 1,
                partial_failure: 0,
                failed: 0,
                cancelled: 0,
            },
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("doc-1"));
        assert!(json.contains("\"total\":1"));
        assert!(json.contains("\"total_pages\":1"));
        assert!(json.contains("\"has_more\":false"));
    }

    #[test]
    fn test_document_detail_response_serialization() {
        let response = DocumentDetailResponse {
            id: "doc-789".to_string(),
            title: Some("Test".to_string()),
            file_name: None,
            content: None,
            content_summary: None,
            content_length: None,
            content_hash: None,
            chunk_count: 5,
            entity_count: None,
            relationship_count: None,
            status: "processed".to_string(),
            error_message: None,
            source_type: None,
            mime_type: None,
            file_size: None,
            track_id: None,
            tenant_id: None,
            workspace_id: None,
            created_at: None,
            updated_at: None,
            processed_at: None,
            lineage: None,
            metadata: None,
            pdf_id: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("doc-789"));
        assert!(json.contains("processed"));
    }

    #[test]
    fn test_delete_document_response_serialization() {
        let response = DeleteDocumentResponse {
            document_id: "doc-to-delete".to_string(),
            deleted: true,
            chunks_deleted: 7,
            entities_affected: 2,
            relationships_affected: 1,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("doc-to-delete"));
        assert!(json.contains("\"deleted\":true"));
        assert!(json.contains("\"chunks_deleted\":7"));
    }

    #[test]
    fn test_default_page() {
        assert_eq!(default_page(), 1);
    }

    #[test]
    fn test_default_page_size() {
        assert_eq!(default_page_size(), 20);
    }

    #[test]
    fn test_track_status_response_serialization() {
        let response = TrackStatusResponse {
            track_id: "upload_20240101_abc12345".to_string(),
            created_at: Some("2024-01-01T00:00:00Z".to_string()),
            documents: vec![DocumentSummary {
                id: "doc-1".to_string(),
                title: Some("Test Doc".to_string()),
                file_name: None,
                content_summary: None,
                content_length: None,
                chunk_count: 5,
                entity_count: Some(3),
                status: Some("completed".to_string()),
                error_message: None,
                track_id: Some("upload_20240101_abc12345".to_string()),
                created_at: None,
                updated_at: None,
                cost_usd: None,
                input_tokens: None,
                output_tokens: None,
                total_tokens: None,
                llm_model: None,
                embedding_model: None,
                // SPEC-002 fields
                source_type: None,
                current_stage: Some("completed".to_string()),
                stage_progress: None,
                stage_message: None,
                pdf_id: None,
            }],
            total_count: 1,
            status_summary: StatusCounts {
                pending: 0,
                processing: 0,
                completed: 1,
                partial_failure: 0,
                failed: 0,
                cancelled: 0,
            },
            is_complete: true,
            latest_message: Some("All documents processed successfully".to_string()),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("upload_20240101_abc12345"));
        assert!(json.contains("\"is_complete\":true"));
        assert!(json.contains("\"total_count\":1"));
    }
}
