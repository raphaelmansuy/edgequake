//! Delete, file upload, and batch operation DTOs.

use serde::Serialize;
use utoipa::ToSchema;

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

    /// Background task ID when upload is async (SPEC-024 Phase 1.1).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,

    /// Track ID for status polling.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub track_id: Option<String>,

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
