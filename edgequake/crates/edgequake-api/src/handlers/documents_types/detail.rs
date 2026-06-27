//! Document detail and lineage DTOs.

use serde::Serialize;
use utoipa::ToSchema;

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

    /// Non-fatal processing notice (e.g. vision parser fallback).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning_message: Option<String>,

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

    /// Multimodal analyze aggregate counts (LightRAG sidecar summary).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub multimodal_summary: Option<crate::services::MultimodalSummary>,

    /// Per-item multimodal analyze status (E52).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub multimodal_items: Option<Vec<crate::services::MultimodalItemStatusView>>,
}

/// Get document by ID request.
#[derive(Debug, Clone, serde::Deserialize, ToSchema)]
pub struct GetDocumentRequest {
    /// Document ID.
    pub document_id: String,
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

    /// Vision LLM model used for PDF extraction (PDF documents only).
    /// Set when the document was extracted using Vision LLM (SPEC-040).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pdf_vision_model: Option<String>,

    /// PDF extraction method used: "vision" or "text" (PDF documents only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pdf_extraction_method: Option<String>,

    /// PDF extraction warning (for example low text content from EdgeParse).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pdf_extraction_warning: Option<String>,
}
