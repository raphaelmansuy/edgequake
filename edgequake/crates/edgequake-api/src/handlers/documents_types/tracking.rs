//! Track status and directory scan DTOs.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::defaults::{default_max_files, default_recursive, documents_default_true};
use super::listing::{DocumentSummary, StatusCounts};

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

    /// SPEC-084 / GH-318: expected documents in this client batch (when declared).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_count: Option<usize>,

    /// SPEC-084 / GH-318: documents currently registered for this track.
    #[serde(default)]
    pub registered_count: usize,

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
