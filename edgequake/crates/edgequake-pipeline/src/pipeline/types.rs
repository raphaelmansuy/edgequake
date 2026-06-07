//! Pipeline result types and progress callbacks (SPEC-017 SRP).

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::chunker::TextChunk;
use crate::extractor::ExtractionResult;

/// Result of processing a document through the pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingResult {
    /// Document ID.
    pub document_id: String,

    /// Generated chunks.
    pub chunks: Vec<TextChunk>,

    /// Extraction results per chunk.
    pub extractions: Vec<ExtractionResult>,

    /// Processing statistics.
    pub stats: ProcessingStats,

    /// Document lineage tracking (optional).
    pub lineage: Option<crate::lineage::DocumentLineage>,
}

/// Statistics from pipeline processing.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProcessingStats {
    /// Number of chunks created.
    pub chunk_count: usize,

    /// Number of chunks successfully extracted.
    #[serde(default)]
    pub successful_chunks: usize,

    /// Number of chunks that failed extraction after all retries.
    #[serde(default)]
    pub failed_chunks: usize,

    /// Error messages for each failed chunk.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunk_errors: Option<Vec<ChunkErrorInfo>>,

    /// Number of entities extracted.
    pub entity_count: usize,

    /// Number of relationships extracted.
    pub relationship_count: usize,

    /// Processing time in milliseconds.
    pub processing_time_ms: u64,

    /// Number of LLM calls made.
    pub llm_calls: usize,

    /// Total tokens used.
    pub total_tokens: usize,

    /// LLM model used for entity extraction.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub llm_model: Option<String>,

    /// LLM provider used for entity extraction.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub llm_provider: Option<String>,

    /// Embedding model used for vector embeddings.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding_model: Option<String>,

    /// Embedding provider used for vector embeddings.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding_provider: Option<String>,

    /// Embedding dimensions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding_dimensions: Option<usize>,

    /// Entity types extracted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_types: Option<Vec<String>>,

    /// Relationship types extracted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relationship_types: Option<Vec<String>>,

    /// Keywords extracted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keywords: Option<Vec<String>>,

    /// Chunking strategy used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunking_strategy: Option<String>,

    /// Average chunk size in characters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avg_chunk_size: Option<usize>,

    /// Input tokens used (for LLM calls).
    #[serde(default)]
    pub input_tokens: usize,

    /// Output tokens used (for LLM calls).
    #[serde(default)]
    pub output_tokens: usize,

    /// Total cost in USD (calculated from token usage).
    #[serde(default)]
    pub cost_usd: f64,

    /// Cost breakdown by operation (extraction, embedding, etc.).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_breakdown: Option<CostBreakdownStats>,

    /// Storage-level error details (graph/vector DB failures).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_details: Option<String>,
}

/// Information about a failed chunk for error reporting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkErrorInfo {
    /// Chunk ID (for correlation with source document).
    pub chunk_id: String,
    /// Chunk index (0-based position in document).
    pub chunk_index: usize,
    /// Error message (user-friendly).
    pub error_message: String,
    /// Whether this was a timeout vs other error.
    pub was_timeout: bool,
    /// Number of retry attempts made.
    pub retry_attempts: u32,
}

/// Cost breakdown by operation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CostBreakdownStats {
    /// Cost for entity extraction.
    #[serde(default)]
    pub extraction_cost_usd: f64,

    /// Cost for embedding generation.
    #[serde(default)]
    pub embedding_cost_usd: f64,

    /// Cost for summarization.
    #[serde(default)]
    pub summarization_cost_usd: f64,

    /// Extraction input tokens.
    #[serde(default)]
    pub extraction_input_tokens: usize,

    /// Extraction output tokens.
    #[serde(default)]
    pub extraction_output_tokens: usize,

    /// Embedding tokens.
    #[serde(default)]
    pub embedding_tokens: usize,
}

/// Progress update for a single chunk during extraction.
#[derive(Debug, Clone)]
pub struct ChunkProgressUpdate {
    /// Index of the chunk being processed (0-based).
    pub chunk_index: usize,
    /// Total number of chunks in the document.
    pub total_chunks: usize,
    /// Preview of the chunk content (first 100 chars).
    pub chunk_preview: String,
    /// Time taken to process this chunk in milliseconds.
    pub processing_time_ms: u64,
    /// Input tokens consumed for this chunk.
    pub input_tokens: usize,
    /// Output tokens generated for this chunk.
    pub output_tokens: usize,
    /// Cost in USD for this chunk's LLM call.
    pub chunk_cost_usd: f64,
    /// Cumulative input tokens across all processed chunks.
    pub cumulative_input_tokens: u64,
    /// Cumulative output tokens across all processed chunks.
    pub cumulative_output_tokens: u64,
    /// Cumulative cost in USD.
    pub cumulative_cost_usd: f64,
    /// Average time per chunk in milliseconds (for ETA calculation).
    pub avg_time_per_chunk_ms: f64,
    /// Estimated remaining time in seconds.
    pub eta_seconds: u64,
}

/// Callback function type for chunk progress updates.
pub type ChunkProgressCallback = Arc<dyn Fn(ChunkProgressUpdate) + Send + Sync>;

/// Progress update emitted during the embedding generation phase.
#[derive(Debug, Clone)]
pub struct EmbedProgressUpdate {
    /// Sub-stage: "chunks", "entities", or "relationships".
    pub stage: &'static str,
    /// Number of items whose embeddings have been requested so far.
    pub current: usize,
    /// Total number of items to embed in this sub-stage.
    pub total: usize,
}

/// Callback invoked at key milestones during embedding generation.
pub type EmbedProgressCallback = Arc<dyn Fn(EmbedProgressUpdate) + Send + Sync>;
