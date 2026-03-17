//! List/query request and response DTOs, including shared summary types.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::defaults::{default_page, default_page_size};

/// List documents request.
///
/// @implements SPEC-005: Document date and pattern filters
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ListDocumentsRequest {
    /// Page number.
    #[serde(default = "default_page")]
    pub page: usize,

    /// Page size.
    #[serde(default = "default_page_size")]
    pub page_size: usize,

    /// Filter: start date (inclusive) in ISO 8601 format.
    /// Only documents created on or after this date are returned.
    /// @implements SPEC-005
    #[serde(default)]
    pub date_from: Option<String>,

    /// Filter: end date (inclusive) in ISO 8601 format.
    /// Only documents created on or before this date are returned.
    /// @implements SPEC-005
    #[serde(default)]
    pub date_to: Option<String>,

    /// Filter: case-insensitive substring pattern for document titles.
    /// Comma-separated values are OR conditions (e.g., "report,summary").
    /// @implements SPEC-005
    #[serde(default)]
    pub document_pattern: Option<String>,
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
